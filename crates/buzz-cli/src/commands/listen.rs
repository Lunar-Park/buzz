//! Persistent external-agent event stream.
//!
//! `buzz listen` prints one newline-delimited JSON record per matching relay
//! event. With `--envelope v1`, lifecycle records use the same stdout stream.
//! Human diagnostics remain on stderr.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use uuid::Uuid;

use crate::client::{normalize_events, BuzzClient};
use crate::error::CliError;
use crate::validate::validate_uuid;

/// Default kinds for channel traffic. Matches `messages get`.
const DEFAULT_KINDS: &[u32] = &[9, 40002, 40008, 45001, 45003];

pub(crate) fn parse_kinds(raw: Option<&str>) -> Result<Vec<u32>, CliError> {
    match raw {
        None => Ok(DEFAULT_KINDS.to_vec()),
        Some(s) if s.trim().is_empty() => Ok(DEFAULT_KINDS.to_vec()),
        Some(s) => {
            let mut kinds = Vec::new();
            for part in s.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let kind = part
                    .parse::<u32>()
                    .map_err(|_| CliError::Usage(format!("invalid kind in --kinds: {part}")))?;
                kinds.push(kind);
            }
            if kinds.is_empty() {
                return Err(CliError::Usage(
                    "--kinds must list at least one kind".into(),
                ));
            }
            Ok(kinds)
        }
    }
}

/// Build REQ filters for `buzz listen`.
///
/// When both channel and mention constraints are present, they are applied in a
/// single filter so the relay returns events matching channel AND mention.
pub(crate) fn build_listen_filters(
    channels: &[String],
    mentions_of_me: bool,
    my_pubkey_hex: &str,
    kinds: &[u32],
    since: Option<u64>,
) -> Result<Vec<serde_json::Value>, CliError> {
    if channels.is_empty() && !mentions_of_me {
        return Err(CliError::Usage(
            "buzz listen requires --channel <UUID> and/or --mentions-of-me".into(),
        ));
    }
    for channel in channels {
        validate_uuid(channel)?;
    }

    let mut filter = json!({
        "kinds": kinds,
    });
    if !channels.is_empty() {
        filter["#h"] = json!(channels);
    }
    if mentions_of_me {
        filter["#p"] = json!([my_pubkey_hex]);
    }
    if let Some(since) = since {
        filter["since"] = json!(since);
    }

    Ok(vec![filter])
}

fn http_to_ws(http_url: &str) -> String {
    http_url
        .replace("https://", "wss://")
        .replace("http://", "ws://")
}

fn lifecycle_record(state: &str, message: Option<&str>) -> serde_json::Value {
    let mut record = json!({
        "schema_version": 1,
        "type": "lifecycle",
        "state": state,
    });
    if let Some(message) = message {
        record["message"] = json!(message);
    }
    record
}

pub(crate) fn event_record(
    event: serde_json::Value,
    envelope: crate::ListenEnvelope,
) -> serde_json::Value {
    match envelope {
        crate::ListenEnvelope::Flat => event,
        crate::ListenEnvelope::V1 => json!({
            "schema_version": 1,
            "type": "event",
            "event": event,
        }),
    }
}

fn write_stdout_record(record: &serde_json::Value) -> Result<(), CliError> {
    let line = serde_json::to_string(record)
        .map_err(|e| CliError::Other(format!("serialize listen record: {e}")))?;
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}").map_err(|e| CliError::Other(format!("stdout write: {e}")))?;
    stdout
        .flush()
        .map_err(|e| CliError::Other(format!("stdout flush: {e}")))?;
    Ok(())
}

fn write_lifecycle(
    envelope: crate::ListenEnvelope,
    state: &str,
    message: Option<&str>,
) -> Result<(), CliError> {
    if matches!(envelope, crate::ListenEnvelope::V1) {
        write_stdout_record(&lifecycle_record(state, message))?;
    }
    Ok(())
}

fn spawn_shutdown_watcher(running: Arc<AtomicBool>) {
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        running.store(false, Ordering::SeqCst);
    });
}

#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let interrupt = signal(SignalKind::interrupt());
    let terminate = signal(SignalKind::terminate());

    match (interrupt, terminate) {
        (Ok(mut interrupt), Ok(mut terminate)) => {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = terminate.recv() => {}
            }
        }
        _ => {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

/// Run the listen loop until shutdown or fatal error.
pub async fn cmd_listen(
    client: &BuzzClient,
    channels: Vec<String>,
    mentions_of_me: bool,
    kinds_raw: Option<String>,
    since: Option<u64>,
    envelope: crate::ListenEnvelope,
    reconnect: bool,
) -> Result<(), CliError> {
    let kinds = parse_kinds(kinds_raw.as_deref())?;
    let my_pubkey = client.keys().public_key().to_hex();
    let filters = build_listen_filters(&channels, mentions_of_me, &my_pubkey, &kinds, since)?;
    let ws_url = http_to_ws(client.relay_url());
    let running = Arc::new(AtomicBool::new(true));
    spawn_shutdown_watcher(running.clone());

    let mut backoff_ms = 500_u64;
    const MAX_BACKOFF_MS: u64 = 30_000;

    while running.load(Ordering::SeqCst) {
        match listen_session(client, &ws_url, &filters, envelope, running.clone()).await {
            Ok(()) => {
                if !running.load(Ordering::SeqCst) || !reconnect {
                    break;
                }
            }
            Err(error) => {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                if !reconnect {
                    let _ = write_lifecycle(envelope, "fatal", Some(&error.to_string()));
                    return Err(error);
                }
                eprintln!(
                    "{}",
                    json!({
                        "error": "listen_reconnect",
                        "message": error.to_string(),
                        "backoff_ms": backoff_ms,
                    })
                );
            }
        }

        if !reconnect || !running.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        backoff_ms = backoff_ms.saturating_mul(2).min(MAX_BACKOFF_MS);
    }

    Ok(())
}

async fn listen_session(
    client: &BuzzClient,
    ws_url: &str,
    filters: &[serde_json::Value],
    envelope: crate::ListenEnvelope,
    running: Arc<AtomicBool>,
) -> Result<(), CliError> {
    use buzz_ws_client::{NostrWsConnection, RelayMessage};

    let mut conn =
        NostrWsConnection::connect_authenticated(ws_url, client.keys(), client.auth_tag())
            .await
            .map_err(|e| CliError::Other(format!("websocket: {e}")))?;

    write_lifecycle(envelope, "connected", None)?;

    let sub_id = format!("buzz-listen-{}", &Uuid::new_v4().to_string()[..8]);
    let mut req = vec![json!("REQ"), json!(sub_id)];
    for filter in filters {
        req.push(filter.clone());
    }
    conn.send_raw(&json!(req))
        .await
        .map_err(|e| CliError::Other(format!("websocket send: {e}")))?;

    while running.load(Ordering::SeqCst) {
        let msg = match conn.next_event(Duration::from_millis(500)).await {
            Ok(msg) => msg,
            Err(buzz_ws_client::WsClientError::Timeout) => continue,
            Err(error) => return Err(CliError::Other(format!("websocket: {error}"))),
        };

        match msg {
            RelayMessage::Event { event, .. } => {
                let raw = serde_json::to_value(event.as_ref())
                    .map_err(|e| CliError::Other(format!("event serialize: {e}")))?;
                let normalized = normalize_events(std::slice::from_ref(&raw));
                let events = serde_json::from_str::<Vec<serde_json::Value>>(&normalized)
                    .unwrap_or_else(|_| vec![raw]);
                let event = events.into_iter().next().unwrap_or_else(|| json!({}));
                write_stdout_record(&event_record(event, envelope))?;
            }
            RelayMessage::Eose { .. } => write_lifecycle(envelope, "eose", None)?,
            RelayMessage::Closed { message, .. } => {
                write_lifecycle(envelope, "closed", Some(&message))?;
                return Err(CliError::Other(format!("subscription closed: {message}")));
            }
            RelayMessage::Notice { message } => {
                eprintln!("{}", json!({"notice": message}));
            }
            RelayMessage::Ok(_) | RelayMessage::Auth { .. } | RelayMessage::Count { .. } => {}
        }
    }

    let _ = conn.send_raw(&json!(["CLOSE", sub_id])).await;
    let _ = conn.disconnect().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_channel_or_mentions() {
        let err =
            build_listen_filters(&[], false, &"a".repeat(64), DEFAULT_KINDS, None).unwrap_err();
        assert!(matches!(err, CliError::Usage(_)));
    }

    #[test]
    fn channel_filter_uses_h_tag() {
        let channel = "11111111-1111-1111-1111-111111111111".to_string();
        let filters = build_listen_filters(
            std::slice::from_ref(&channel),
            false,
            &"a".repeat(64),
            DEFAULT_KINDS,
            None,
        )
        .unwrap();

        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0]["#h"][0], channel);
        assert!(filters[0].get("#p").is_none());
    }

    #[test]
    fn mentions_filter_uses_p_tag() {
        let pubkey = "b".repeat(64);
        let filters = build_listen_filters(&[], true, &pubkey, DEFAULT_KINDS, None).unwrap();

        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0]["#p"][0], pubkey);
        assert!(filters[0].get("#h").is_none());
    }

    #[test]
    fn channel_and_mentions_are_single_and_filter() {
        let channel = "11111111-1111-1111-1111-111111111111".to_string();
        let pubkey = "c".repeat(64);
        let filters = build_listen_filters(
            std::slice::from_ref(&channel),
            true,
            &pubkey,
            DEFAULT_KINDS,
            None,
        )
        .unwrap();

        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0]["#h"][0], channel);
        assert_eq!(filters[0]["#p"][0], pubkey);
    }

    #[test]
    fn since_is_added_to_filter() {
        let filters = build_listen_filters(
            &["11111111-1111-1111-1111-111111111111".to_string()],
            false,
            &"a".repeat(64),
            DEFAULT_KINDS,
            Some(1785100000),
        )
        .unwrap();

        assert_eq!(filters[0]["since"], 1785100000_u64);
    }

    #[test]
    fn parse_kinds_defaults_and_parses_csv() {
        assert_eq!(parse_kinds(None).unwrap(), DEFAULT_KINDS);
        assert_eq!(parse_kinds(Some("9, 40002")).unwrap(), vec![9, 40002]);
    }

    #[test]
    fn parse_kinds_rejects_invalid_value() {
        let err = parse_kinds(Some("9,nope")).unwrap_err();
        assert!(matches!(err, CliError::Usage(_)));
    }

    #[test]
    fn v1_event_record_wraps_flat_event() {
        let event = json!({
            "id": "event",
            "pubkey": "author",
            "kind": 40002,
            "content": "hello",
            "created_at": 1785100000_u64,
            "tags": [["h", "channel"]],
        });

        let record = event_record(event.clone(), crate::ListenEnvelope::V1);

        assert_eq!(record["schema_version"], 1);
        assert_eq!(record["type"], "event");
        assert_eq!(record["event"], event);
    }

    #[test]
    fn flat_event_record_is_unchanged() {
        let event = json!({"id": "event"});
        assert_eq!(
            event_record(event.clone(), crate::ListenEnvelope::Flat),
            event
        );
    }

    #[test]
    fn lifecycle_record_uses_v1_shape() {
        let record = lifecycle_record("eose", None);

        assert_eq!(
            record,
            json!({
                "schema_version": 1,
                "type": "lifecycle",
                "state": "eose",
            })
        );
    }
}
