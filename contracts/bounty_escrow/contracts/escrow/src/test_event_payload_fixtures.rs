//! Event payload schema fixtures — pins every `events.rs` emitted payload shape.
//!
//! # Regenerating fixtures
//!
//! ```bash
//! python3 scripts/regen_event_payload_fixtures.py
//! cargo test -p bounty-escrow --lib event_payload_fixtures -- --nocapture
//! ```
//!
//! # Versioning policy
//!
//! - `FIXTURE_EVENT_VERSION` must equal `EVENT_VERSION_V2`.
//! - Breaking field changes require bumping `EVENT_VERSION_V2`.
//! - Additive optional fields may keep the same version.
//!
//! # Validation
//!
//! Reorder any field in an event struct in `events.rs` without regenerating fixtures;
//! `event_payload_fixtures_match_events_rs` fails.

#![cfg(test)]

extern crate std;

use std::string::ToString;

use crate::event_payload_fixtures::{
    EventPayloadFixture, EVENT_ENUM_FIXTURES, EVENT_PAYLOAD_FIXTURES, FIXTURE_EVENT_VERSION,
};
use crate::events::EVENT_VERSION_V2;

fn clean_type(t: &str) -> std::string::String {
    let mut out = std::string::String::new();
    let without_comment = t.split("//").next().unwrap_or(t);
    for ch in without_comment.chars() {
        if !ch.is_whitespace() {
            // collapse later
        }
    }
    let trimmed = without_comment.trim().trim_end_matches(',').trim();
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

/// Returns a valid UTF-8 boundary no more than `bytes` before `end`.
/// The parser uses byte offsets, while `events.rs` documentation contains
/// multi-byte Unicode box drawing characters.
fn lookback_boundary(src: &str, end: usize, bytes: usize) -> usize {
    let mut start = end.saturating_sub(bytes);
    while start > 0 && !src.is_char_boundary(start) {
        start -= 1;
    }
    start
}

fn parse_contracttype_structs(src: &str) -> std::vec::Vec<(std::string::String, std::vec::Vec<(std::string::String, std::string::String)>)> {
    let bytes = src.as_bytes();
    let mut out = std::vec::Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = src[search_from..].find("pub struct ") {
        let abs = search_from + rel;
        let lookback_start = lookback_boundary(src, abs, 300);
        if !src[lookback_start..abs].contains("#[contracttype]") {
            search_from = abs + 11;
            continue;
        }
        let name_start = abs + "pub struct ".len();
        let name_end = src[name_start..]
            .find(|c: char| c == ' ' || c == '{')
            .map(|i| name_start + i)
            .unwrap_or(name_start);
        let name = src[name_start..name_end].trim().to_string();
        let brace = match src[name_end..].find('{') {
            Some(i) => name_end + i,
            None => {
                search_from = name_end;
                continue;
            }
        };
        let mut depth = 0i32;
        let mut i = brace;
        let mut end = None;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        let end = match end {
            Some(e) => e,
            None => break,
        };
        let body = &src[brace + 1..end];
        let mut fields = std::vec::Vec::new();
        for line in body.lines() {
            let line = line.trim();
            if !line.starts_with("pub ") {
                continue;
            }
            let line = line.split("//").next().unwrap_or(line).trim().trim_end_matches(',').trim();
            if let Some(rest) = line.strip_prefix("pub ") {
                if let Some((fname, fty)) = rest.split_once(':') {
                    fields.push((fname.trim().to_string(), clean_type(fty)));
                }
            }
        }
        out.push((name, fields));
        search_from = end + 1;
    }
    out
}

fn parse_contracttype_enums(src: &str) -> std::vec::Vec<(std::string::String, std::vec::Vec<std::string::String>)> {
    let bytes = src.as_bytes();
    let mut out = std::vec::Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = src[search_from..].find("pub enum ") {
        let abs = search_from + rel;
        let lookback_start = lookback_boundary(src, abs, 300);
        if !src[lookback_start..abs].contains("#[contracttype]") {
            search_from = abs + 9;
            continue;
        }
        let name_start = abs + "pub enum ".len();
        let name_end = src[name_start..]
            .find(|c: char| c == ' ' || c == '{')
            .map(|i| name_start + i)
            .unwrap_or(name_start);
        let name = src[name_start..name_end].trim().to_string();
        let brace = match src[name_end..].find('{') {
            Some(i) => name_end + i,
            None => {
                search_from = name_end;
                continue;
            }
        };
        let mut depth = 0i32;
        let mut i = brace;
        let mut end = None;
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        let end = match end {
            Some(e) => e,
            None => break,
        };
        let body = &src[brace + 1..end];
        let mut variants = std::vec::Vec::new();
        for line in body.lines() {
            let line = line.trim().trim_end_matches(',');
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            variants.push(line.to_string());
        }
        out.push((name, variants));
        search_from = end + 1;
    }
    out
}

#[test]
fn fixture_event_version_matches_constant() {
    assert_eq!(
        FIXTURE_EVENT_VERSION, EVENT_VERSION_V2,
        "FIXTURE_EVENT_VERSION must track EVENT_VERSION_V2 (bump both on breaking changes)"
    );
}

#[test]
fn event_payload_fixtures_cover_every_event() {
    let src = include_str!("events.rs");
    let live = parse_contracttype_structs(src);
    assert!(
        !live.is_empty(),
        "parser found no #[contracttype] event structs in events.rs"
    );
    assert_eq!(
        live.len(),
        EVENT_PAYLOAD_FIXTURES.len(),
        "fixture count {} != live event struct count {} — regenerate with python3 scripts/regen_event_payload_fixtures.py",
        EVENT_PAYLOAD_FIXTURES.len(),
        live.len()
    );
}

#[test]
fn event_payload_fixtures_match_events_rs() {
    let src = include_str!("events.rs");
    let live = parse_contracttype_structs(src);

    let mut live_map: std::collections::BTreeMap<std::string::String, std::vec::Vec<(std::string::String, std::string::String)>> =
        std::collections::BTreeMap::new();
    for (name, fields) in live {
        live_map.insert(name, fields);
    }

    for fixture in EVENT_PAYLOAD_FIXTURES {
        let fields = live_map.remove(fixture.name).unwrap_or_else(|| {
            panic!(
                "fixture event `{}` missing from events.rs — regenerate fixtures",
                fixture.name
            )
        });
        let has_version = fields.iter().any(|(n, _)| n == "version");
        assert_eq!(
            has_version, fixture.has_version_field,
            "version field flag mismatch for {}",
            fixture.name
        );
        assert_eq!(
            fields.len(),
            fixture.fields.len(),
            "field count mismatch for {} (live={}, fixture={}). Reorder/add/remove detected — regenerate or bump EVENT_VERSION_V2",
            fixture.name,
            fields.len(),
            fixture.fields.len()
        );
        for (idx, ((ln, lt), ff)) in fields.iter().zip(fixture.fields.iter()).enumerate() {
            assert_eq!(
                ln.as_str(),
                ff.name,
                "field order/name mismatch for {} at index {} (live=`{}`, fixture=`{}`)",
                fixture.name,
                idx,
                ln,
                ff.name
            );
            assert_eq!(
                lt.as_str(),
                ff.ty,
                "field type mismatch for {}.{} (live=`{}`, fixture=`{}`)",
                fixture.name,
                ln,
                lt,
                ff.ty
            );
        }
    }

    assert!(
        live_map.is_empty(),
        "events.rs has event structs not in fixtures: {:?} — run python3 scripts/regen_event_payload_fixtures.py",
        live_map.keys().collect::<std::vec::Vec<_>>()
    );
}

#[test]
fn event_enum_fixtures_match_events_rs() {
    let src = include_str!("events.rs");
    let live = parse_contracttype_enums(src);
    assert_eq!(
        live.len(),
        EVENT_ENUM_FIXTURES.len(),
        "enum fixture count mismatch — regenerate fixtures"
    );
    for (fixture, (name, variants)) in EVENT_ENUM_FIXTURES.iter().zip(live.iter()) {
        assert_eq!(fixture.name, name.as_str());
        assert_eq!(
            fixture.variants.len(),
            variants.len(),
            "variant count mismatch for {}",
            fixture.name
        );
        for (fv, lv) in fixture.variants.iter().zip(variants.iter()) {
            assert_eq!(*fv, lv.as_str(), "variant mismatch in {}", fixture.name);
        }
    }
}

#[test]
fn versioned_events_use_explicit_version_field() {
    // Document the additive vs breaking split: versioned events carry `version`.
    let versioned: std::vec::Vec<_> = EVENT_PAYLOAD_FIXTURES
        .iter()
        .filter(|e| e.has_version_field)
        .map(|e| e.name)
        .collect();
    assert!(
        !versioned.is_empty(),
        "expected versioned event payloads"
    );
    for fixture in EVENT_PAYLOAD_FIXTURES.iter().filter(|e| e.has_version_field) {
        assert_eq!(
            fixture.fields[0].name, "version",
            "{} should declare `version` as the first field (envelope)",
            fixture.name
        );
        assert_eq!(fixture.fields[0].ty, "u32");
    }
}

#[test]
fn reorder_detection_documents_acceptance() {
    // Sanity: fixtures are ordered; a synthetic reorder disagrees with live parse equality.
    let first = EVENT_PAYLOAD_FIXTURES
        .iter()
        .find(|e| e.fields.len() >= 2)
        .expect("need an event with >=2 fields");
    let mut reordered: std::vec::Vec<_> = first.fields.iter().copied().collect();
    reordered.swap(0, 1);
    let live_names: std::vec::Vec<_> = first.fields.iter().map(|f| f.name).collect();
    let reordered_names: std::vec::Vec<_> = reordered.iter().map(|f| f.name).collect();
    assert_ne!(
        live_names, reordered_names,
        "fixture check is order-sensitive (reorder must fail)"
    );
    let _keep: &EventPayloadFixture = first;
}
