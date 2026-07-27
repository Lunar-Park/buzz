use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/external_agent_v1")
}

#[test]
fn external_agent_v1_fixtures_have_expected_parse_contract() {
    let dir = fixtures_dir();
    let expected_raw = fs::read_to_string(dir.join("expected_facts.json")).unwrap();
    let expected: Value = serde_json::from_str(&expected_raw).unwrap();

    let mut fixture_names = HashSet::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("ndjson") {
            fixture_names.insert(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }

    for record in expected["records"].as_array().unwrap() {
        let fixture = record["fixture"].as_str().unwrap();
        assert!(
            fixture_names.contains(fixture),
            "expected_facts references missing fixture {fixture}"
        );
    }

    for fixture in &fixture_names {
        let raw = fs::read_to_string(dir.join(fixture)).unwrap();
        for (idx, line) in raw.lines().enumerate() {
            let parsed = serde_json::from_str::<Value>(line);
            if fixture == "malformed_json_line.ndjson" {
                assert!(
                    parsed.is_err(),
                    "malformed_json_line.ndjson line {} should not parse",
                    idx + 1
                );
                continue;
            }

            let value = parsed.unwrap_or_else(|err| {
                panic!("{fixture} line {} should parse as JSON: {err}", idx + 1)
            });
            assert_eq!(
                value["schema_version"].as_u64(),
                Some(if fixture == "future_schema_version.ndjson" {
                    99
                } else {
                    1
                }),
                "{fixture} line {} schema_version mismatch",
                idx + 1
            );
            assert!(
                value["type"].as_str().is_some(),
                "{fixture} line {} missing type",
                idx + 1
            );
        }
    }
}
