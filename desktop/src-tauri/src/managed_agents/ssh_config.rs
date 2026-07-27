//! `~/.ssh/config` host enumeration.
//!
//! Buzz needs the user's own host list to offer "which machine is this agent
//! on?" without asking them to retype connection details they already
//! maintain. This is a deliberately partial parser: it reads the four keywords
//! needed to open a connection and ignores everything else. `ssh` itself
//! remains the authority on how a connection is actually made — we never
//! reimplement its resolution rules, we only enumerate candidate host aliases
//! to show the user and hand back to `ssh` verbatim.

use std::path::{Path, PathBuf};

/// One `Host` stanza from `~/.ssh/config`, reduced to the fields Buzz uses.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHost {
    /// The `Host` alias as written. This is what gets passed to `ssh`, not
    /// `hostname` — the alias is what carries the user's own config.
    pub host: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<String>,
    pub identity_file: Option<String>,
}

/// Parse `~/.ssh/config` and return its host stanzas in file order.
///
/// A missing or unreadable file yields an empty list rather than an error: not
/// having an ssh config is a normal state, and it means "no remote hosts to
/// offer", not "discovery failed".
pub fn parse_ssh_config() -> Vec<SshHost> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    parse_ssh_config_at(&home.join(".ssh").join("config"))
}

/// Testable core of [`parse_ssh_config`], reading from an explicit path.
pub fn parse_ssh_config_at(path: &Path) -> Vec<SshHost> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    parse_ssh_config_str(&content)
}

/// Pure parser over the config text.
///
/// Keyword matching is case-insensitive because ssh's own is; `Host`, `host`,
/// and `HOST` are all valid in a real config file.
pub fn parse_ssh_config_str(content: &str) -> Vec<SshHost> {
    let mut hosts: Vec<SshHost> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // ssh accepts `Key value` and `Key=value`; normalize the separator
        // before splitting so `User=miles` is not read as a key named
        // "user=miles".
        let normalized = line.replacen('=', " ", 1);
        let mut parts = normalized.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let value = parts.collect::<Vec<_>>().join(" ");
        if value.is_empty() {
            continue;
        }

        if key.eq_ignore_ascii_case("host") {
            // One `Host` line may declare several aliases. Each becomes its own
            // entry so the user can pick any of them, and subsequent keywords
            // in the stanza apply to all of them — which is ssh's behavior.
            for alias in value.split_whitespace() {
                // Patterns cannot be connected to, only matched against. `*` in
                // particular is the catch-all defaults stanza.
                if alias.contains('*') || alias.contains('?') || alias.starts_with('!') {
                    continue;
                }
                hosts.push(SshHost {
                    host: alias.to_string(),
                    hostname: None,
                    user: None,
                    port: None,
                    identity_file: None,
                });
            }
            continue;
        }

        // Keywords before any `Host` line are global defaults. We deliberately
        // do not model them: applying them would mean reimplementing ssh's
        // precedence rules, and `ssh` already applies them itself when we
        // invoke it with the alias.
        let Some(current_stanza_start) = last_host_line_group(&mut hosts) else {
            continue;
        };
        for entry in current_stanza_start {
            match key.to_ascii_lowercase().as_str() {
                "hostname" => entry.hostname = Some(value.clone()),
                "user" => entry.user = Some(value.clone()),
                "port" => entry.port = Some(value.clone()),
                "identityfile" => entry.identity_file = Some(value.clone()),
                _ => {}
            }
        }
    }

    hosts
}

/// Return the entries created by the most recent `Host` line, so a keyword
/// applies to every alias that line declared.
///
/// Aliases from one `Host` line are contiguous at the tail of `hosts` and all
/// still have `hostname`/`user`/`port`/`identity_file` unset at the moment the
/// first keyword arrives, but later keywords must reach the same group — so the
/// group boundary is tracked by scanning back over entries that share the tail's
/// current field values rather than by "is unset".
fn last_host_line_group(hosts: &mut [SshHost]) -> Option<&mut [SshHost]> {
    if hosts.is_empty() {
        return None;
    }
    let len = hosts.len();
    let tail = &hosts[len - 1];
    let (hostname, user, port, identity) = (
        tail.hostname.clone(),
        tail.user.clone(),
        tail.port.clone(),
        tail.identity_file.clone(),
    );
    let mut start = len - 1;
    while start > 0 {
        let candidate = &hosts[start - 1];
        if candidate.hostname == hostname
            && candidate.user == user
            && candidate.port == port
            && candidate.identity_file == identity
        {
            start -= 1;
        } else {
            break;
        }
    }
    Some(&mut hosts[start..])
}

/// Resolve the `ssh` binary Buzz should invoke.
///
/// A GUI app on macOS inherits a minimal `PATH` from launchd, so a bare `ssh`
/// lookup can miss. The standard locations are checked before falling back to
/// the bare name for the platform's own resolution.
pub fn resolve_ssh_binary() -> PathBuf {
    for candidate in ["/usr/bin/ssh", "/bin/ssh", "/opt/homebrew/bin/ssh"] {
        let path = Path::new(candidate);
        if path.exists() {
            return path.to_path_buf();
        }
    }
    PathBuf::from("ssh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_basic_stanza() {
        let hosts = parse_ssh_config_str(
            "Host lunar02\n  HostName lunar02.example.ts.net\n  User miles\n  \
             IdentityFile ~/.ssh/id_ed25519\n",
        );
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "lunar02");
        assert_eq!(hosts[0].hostname.as_deref(), Some("lunar02.example.ts.net"));
        assert_eq!(hosts[0].user.as_deref(), Some("miles"));
        assert_eq!(hosts[0].identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert!(hosts[0].port.is_none());
    }

    #[test]
    fn parses_multiple_stanzas_independently() {
        let hosts = parse_ssh_config_str(
            "Host alpha\n  User a\n  Port 22\n\nHost beta\n  User b\n  Port 2222\n",
        );
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].host, "alpha");
        assert_eq!(hosts[0].user.as_deref(), Some("a"));
        assert_eq!(hosts[0].port.as_deref(), Some("22"));
        assert_eq!(hosts[1].host, "beta");
        assert_eq!(hosts[1].user.as_deref(), Some("b"));
        assert_eq!(hosts[1].port.as_deref(), Some("2222"));
    }

    #[test]
    fn skips_wildcard_and_negated_patterns() {
        // `*` is the defaults stanza and cannot be connected to; offering it as
        // a host would produce a guaranteed-failing probe.
        let hosts = parse_ssh_config_str(
            "Host *\n  ServerAliveInterval 60\n\nHost prod-?\n  User x\n\n\
             Host !secret\n  User y\n\nHost real\n  User z\n",
        );
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "real");
        assert_eq!(hosts[0].user.as_deref(), Some("z"));
    }

    #[test]
    fn one_host_line_with_several_aliases_shares_its_keywords() {
        let hosts = parse_ssh_config_str("Host one two three\n  User shared\n  Port 2200\n");
        assert_eq!(hosts.len(), 3);
        for entry in &hosts {
            assert_eq!(entry.user.as_deref(), Some("shared"));
            assert_eq!(entry.port.as_deref(), Some("2200"));
        }
        assert_eq!(
            hosts.iter().map(|h| h.host.as_str()).collect::<Vec<_>>(),
            vec!["one", "two", "three"]
        );
    }

    #[test]
    fn accepts_equals_separated_and_mixed_case_keywords() {
        let hosts = parse_ssh_config_str("HOST lunar\n  user=miles\n  HostName=box.local\n");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("miles"));
        assert_eq!(hosts[0].hostname.as_deref(), Some("box.local"));
    }

    #[test]
    fn ignores_comments_blank_lines_and_unknown_keywords() {
        let hosts = parse_ssh_config_str(
            "# a comment\n\nHost lunar\n  # inline comment line\n  \
             ForwardAgent yes\n  ProxyJump bastion\n  User miles\n",
        );
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("miles"));
    }

    #[test]
    fn keywords_before_any_host_line_are_ignored() {
        // Global defaults would require reimplementing ssh's precedence; ssh
        // applies them itself when we invoke it with the alias.
        let hosts = parse_ssh_config_str("User global\n\nHost lunar\n  Port 22\n");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "lunar");
        assert!(hosts[0].user.is_none());
    }

    #[test]
    fn missing_file_is_empty_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let hosts = parse_ssh_config_at(&dir.path().join("does-not-exist"));
        assert!(hosts.is_empty());
    }

    #[test]
    fn empty_config_yields_no_hosts() {
        assert!(parse_ssh_config_str("").is_empty());
        assert!(parse_ssh_config_str("\n\n# only comments\n").is_empty());
    }

    #[test]
    fn later_keyword_still_reaches_a_multi_alias_group() {
        // Regression guard for the group-tracking logic: `User` arrives first
        // and mutates all three entries, then `Port` must still find the same
        // group rather than only the tail entry.
        let hosts = parse_ssh_config_str("Host a b c\n  User u\n  Port 42\n");
        assert_eq!(hosts.len(), 3);
        for entry in &hosts {
            assert_eq!(entry.port.as_deref(), Some("42"), "host {}", entry.host);
        }
    }
}
