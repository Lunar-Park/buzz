//! Tests for host-side identity resolution and generation.
//!
//! The two payload shapes here are real: the resolved-pubkey case is what
//! `openclaw config get channels.buzz.agentPubkey` printed on a live host, and the
//! unset case is that command's actual `Config path not found` wording. Both were
//! captured read-only rather than inferred.

use super::*;

const AGENT_HEX: &str = "4687f50de3a9e235e28eb58d68b0746062d7be6401bbf78a766bbd6f96ffe3c9";

// ── resolving an existing identity ───────────────────────────────────────────

#[test]
fn a_configured_pubkey_resolves() {
    assert_eq!(
        parse_resolved_pubkey(AGENT_HEX).expect("parses"),
        Some(AGENT_HEX.to_string())
    );
}

#[test]
fn an_unset_config_path_is_no_identity_rather_than_a_failure() {
    // This is the case that makes offering generation honest. Treating it as an
    // error would hide exactly the situation the generate action exists for.
    let payload = "Config path not found: channels.buzz.agentPubkey. Run openclaw config \
                   validate to inspect config shape.";
    assert_eq!(parse_resolved_pubkey(payload).expect("parses"), None);
}

#[test]
fn empty_output_is_no_identity() {
    assert_eq!(parse_resolved_pubkey("   ").expect("parses"), None);
}

#[test]
fn a_quoted_or_uppercase_pubkey_is_normalized() {
    let payload = format!("\"{}\"", AGENT_HEX.to_uppercase());
    assert_eq!(
        parse_resolved_pubkey(&payload).expect("parses"),
        Some(AGENT_HEX.to_string())
    );
}

#[test]
fn a_non_pubkey_answer_is_an_error_not_a_silent_none() {
    // "The harness told us something we did not understand" and "the harness has
    // no identity" lead to different next actions, so they must not collapse.
    let error = parse_resolved_pubkey("some unexpected banner").expect_err("rejected");
    assert!(error.contains("cannot read as a pubkey"), "{error}");
}

#[test]
fn the_read_command_is_a_single_quoted_region_a_shell_accepts() {
    for recipe in IDENTITY_RECIPES {
        let command = build_read_command(recipe);
        assert_eq!(command.matches('\'').count(), 2, "{command}");
        assert!(command.contains("$SHELL -lc"), "{command}");
        assert!(!command.contains("-lic"), "{command}");
        let status = std::process::Command::new("/bin/sh")
            .arg("-n")
            .arg("-c")
            .arg(&command)
            .status()
            .expect("sh runs");
        assert!(status.success(), "shell rejected: {command}");
    }
}

// ── slug validation: the only user-influenced remote text ────────────────────

#[test]
fn ordinary_harness_agent_ids_are_accepted() {
    for slug in ["main", "astra", "cassian", "agent-1", "a_b.c", "A1"] {
        assert!(validate_identity_slug(slug).is_ok(), "{slug} should pass");
    }
}

#[test]
fn shell_metacharacters_are_refused_not_escaped() {
    // Escaping is a thing you get wrong once; refusing is checkable. Each of
    // these would otherwise end the quoted region or inject a command.
    for slug in [
        "a'b",
        "a\"b",
        "a;rm -rf /",
        "a b",
        "a$(id)",
        "a`id`",
        "a|b",
        "a&b",
        "a\nb",
        "a/b",
        "../escape",
        "-leading-dash",
        ".hidden",
        "",
        "   ",
    ] {
        assert!(
            validate_identity_slug(slug).is_err(),
            "{slug:?} must be refused"
        );
    }
}

#[test]
fn a_dot_dot_sequence_is_refused_even_with_legal_characters() {
    assert!(validate_identity_slug("a..b").is_err());
}

#[test]
fn an_overlong_slug_is_refused() {
    assert!(validate_identity_slug(&"a".repeat(65)).is_err());
    assert!(validate_identity_slug(&"a".repeat(64)).is_ok());
}

#[test]
fn a_validated_slug_yields_a_command_a_shell_accepts() {
    for slug in ["main", "astra", "a_b.c"] {
        let command = build_generate_command(slug);
        assert_eq!(
            command.matches('\'').count(),
            2,
            "only the wrapping quote pair: {command}"
        );
        let status = std::process::Command::new("/bin/sh")
            .arg("-n")
            .arg("-c")
            .arg(&command)
            .status()
            .expect("sh runs");
        assert!(status.success(), "shell rejected: {command}");
    }
}

#[test]
fn generation_never_forces_an_overwrite() {
    // Re-running generation for an agent that already has a key must fail loudly
    // rather than destroy a live identity — the mistake a connect wizard is most
    // likely to cause and least likely to notice.
    let command = build_generate_command("main");
    assert!(!command.contains("--force"), "{command}");
}

#[test]
fn generation_never_asks_for_the_secret_on_stdout() {
    let command = build_generate_command("main");
    assert!(!command.contains("--stdout"), "{command}");
    assert!(command.contains("--out"), "{command}");
}

#[test]
fn the_key_path_is_derived_from_the_slug() {
    assert_eq!(secret_path_for("astra"), "~/.buzz/agents/astra.nsec");
}

// ── parsing a generated identity ─────────────────────────────────────────────

#[test]
fn a_generated_identity_parses_to_its_public_half() {
    let payload = format!(
        r#"{{"pubkey":"{AGENT_HEX}","npub":"npub1g6rl2r0","secret_key_path":"/Users/selene/.buzz/agents/astra.nsec"}}"#
    );
    let (pubkey, npub, path) = parse_generated_identity(&payload).expect("parses");
    assert_eq!(pubkey, AGENT_HEX);
    assert_eq!(npub, "npub1g6rl2r0");
    assert_eq!(path, "/Users/selene/.buzz/agents/astra.nsec");
}

#[test]
fn a_returned_secret_aborts_instead_of_being_discarded() {
    // If a future CLI ever prints a secret by default, silently dropping it would
    // leave it in this process's memory and in the ssh transcript. Refusing the
    // whole result is the only honest response.
    let payload = format!(
        r#"{{"pubkey":"{AGENT_HEX}","npub":"npub1x","secret_key_path":"/p","nsec":"nsec1leaked"}}"#
    );
    let error = parse_generated_identity(&payload).expect_err("must refuse");
    assert!(error.contains("never leave that machine"), "{error}");
    assert!(!error.contains("nsec1leaked"), "the error must not echo it");
}

#[test]
fn a_reply_without_a_key_path_is_refused() {
    // Without it Buzz cannot tell the user where to point the harness, which makes
    // the generated identity unusable.
    let payload = format!(r#"{{"pubkey":"{AGENT_HEX}","npub":"npub1x"}}"#);
    let error = parse_generated_identity(&payload).expect_err("must refuse");
    assert!(error.contains("where it wrote the key"), "{error}");
}

#[test]
fn a_malformed_pubkey_is_refused() {
    let payload = r#"{"pubkey":"nothex","npub":"npub1x","secret_key_path":"/p"}"#;
    assert!(parse_generated_identity(payload).is_err());
}

#[test]
fn login_shell_banners_before_the_json_are_tolerated() {
    let payload = format!(
        "Welcome to the machine\n{{\"pubkey\":\"{AGENT_HEX}\",\"npub\":\"n\",\"secret_key_path\":\"/p\"}}"
    );
    assert!(parse_generated_identity(&payload).is_ok());
}

#[test]
fn a_cli_error_instead_of_json_is_reported() {
    let error = parse_generated_identity("buzz: command not found").expect_err("rejected");
    assert!(
        error.contains("could not find the generated identity"),
        "{error}"
    );
}

#[test]
fn payload_extraction_requires_both_markers() {
    let good = format!("noise\n{IDENTITY_START}\nvalue\n{IDENTITY_END}\nmore");
    assert_eq!(extract_payload(&good), Some("value"));
    assert_eq!(extract_payload(&format!("{IDENTITY_START}\nvalue")), None);
    assert_eq!(extract_payload(&format!("value\n{IDENTITY_END}")), None);
}

#[test]
fn an_unknown_harness_is_unsupported_rather_than_failed() {
    assert!(recipe_for("hermes").is_none());
    assert!(recipe_for("openclaw").is_some());
}
