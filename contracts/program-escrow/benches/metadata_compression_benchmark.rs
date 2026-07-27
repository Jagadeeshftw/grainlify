//! Metadata compression benchmark.
//!
//! Compares the on-chain encoded size of `ProgramMetadata` custom fields
//! using legacy `String` keys vs. compressed `MetadataFieldKey` enum keys.
//!
//! Run with:
//! ```text
//! cargo test -p program-escrow --bench metadata_compression
//! ```
//!
//! Each `#[test]` function reports the encoded byte sizes via `eprintln!`.

#![no_std]
extern crate alloc;

use alloc::vec::Vec as StdVec;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Vec};

// Re-use the contract types.
use program_escrow::{
    CompressedCustomField, CompressedProgramMetadata, MetadataFieldKey, ProgramMetadata,
    ProgramMetadataField,
};

/// A representative metadata payload mimicking a real-world program.
fn representative_payload(env: &Env) -> ProgramMetadata {
    let mut tags: Vec<String> = Vec::new(env);
    tags.push_back(String::from_str(env, "hackathon"));
    tags.push_back(String::from_str(env, "defi"));

    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(env);
    // 3 known keys + 1 custom key (typical real-world usage)
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(env, "total_participants"),
        value: String::from_str(env, "150"),
    });
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(env, "prize_pool_usd"),
        value: String::from_str(env, "50000"),
    });
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(env, "sponsor"),
        value: String::from_str(env, "Stellar Development Foundation"),
    });
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(env, "project_repository"),
        value: String::from_str(env, "https://github.com/stellar/example"),
    });

    ProgramMetadata {
        program_name: Some(String::from_str(env, "Hackathon 2024")),
        program_type: Some(String::from_str(env, "hackathon")),
        ecosystem: Some(String::from_str(env, "stellar")),
        tags,
        start_date: Some(1_720_000_000),
        end_date: Some(1_750_000_000),
        custom_fields,
    }
}

/// Build the compressed equivalent, applying `MetadataFieldKey` to each
/// custom field.  The first three known keys map to enum variants; the
/// fourth ("project_repository") falls through to `Custom`.
fn compressed_payload(env: &Env) -> CompressedProgramMetadata {
    let legacy = representative_payload(env);
    CompressedProgramMetadata::from_legacy(env, &legacy)
}

/// Encode a `#[contracttype]` value to XDR bytes and return the length.
fn encoded_size<T>(env: &Env, value: &T) -> usize
where
    T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
    let val: soroban_sdk::Val = soroban_sdk::IntoVal::into_val(value.clone(), env);
    // Convert Val -> XDR bytes via raw conversion.
    // We use to_xdr on the bytes representation.
    let bytes: soroban_sdk::Bytes = val.to_xdr(env);
    bytes.len() as usize
}

#[test]
fn benchmark_metadata_compression() {
    let env = Env::default();

    let legacy = representative_payload(&env);
    let compressed = compressed_payload(&env);

    let legacy_size = encoded_size(&env, &legacy);
    let compressed_size = encoded_size(&env, &compressed);

    let savings_pct = if legacy_size > 0 {
        ((legacy_size - compressed_size) as f64 / legacy_size as f64 * 100.0)
    } else {
        0.0
    };

    eprintln!("=== Metadata Compression Benchmark ===");
    eprintln!("Legacy (String keys)   : {legacy_size} bytes");
    eprintln!("Compressed (enum keys) : {compressed_size} bytes");
    eprintln!("Savings                : {savings_pct:.1}%");
    eprintln!();
    eprintln!("Breakdown per custom field:");
    let legacy_fields: Vec<ProgramMetadataField> = legacy.custom_fields.iter().collect();
    let compressed_fields: Vec<CompressedCustomField> = compressed.custom_fields.iter().collect();
    for i in 0..legacy_fields.len() {
        let lf = &legacy_fields[i as usize];
        let cf = &compressed_fields[i as usize];
        let lf_size = encoded_size(&env, lf);
        let cf_size = encoded_size(&env, cf);
        let key_str = match &cf.key {
            MetadataFieldKey::Custom(s) => {
                let s: String = s.clone();
                let bytes: soroban_sdk::Bytes =
                    soroban_sdk::IntoVal::into_val(s, &env).to_xdr(&env);
                core::str::from_utf8(bytes.as_ref()).unwrap_or("<binary>")
            }
            _ => "<known-variant>",
        };
        eprintln!("  Field {i}: legacy={lf_size}B compressed={cf_size}B key={key_str:?}",);
    }
    eprintln!();

    // Assert that compression actually reduces size for this payload.
    assert!(
        compressed_size < legacy_size,
        "Compressed ({compressed_size}B) must be smaller than legacy ({legacy_size}B)"
    );
    eprintln!(
        "✓ Compression verified: {legacy_size}B → {compressed_size}B ({savings_pct:.1}% reduction)"
    );
}

#[test]
fn benchmark_compression_max_keys() {
    let env = Env::default();

    // All 10 known keys + 2 custom keys.
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&env);
    let known_keys: &[&str] = &[
        "total_participants",
        "prize_pool_usd",
        "sponsor",
        "repository",
        "website",
        "contact_email",
        "difficulty",
        "category",
        "status",
        "version",
    ];
    for (i, key) in known_keys.iter().enumerate() {
        let val = alloc::format!("value_{}", i);
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&env, key),
            value: String::from_str(&env, &val),
        });
    }
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(&env, "custom_field_1"),
        value: String::from_str(&env, "abc"),
    });
    custom_fields.push_back(ProgramMetadataField {
        key: String::from_str(&env, "custom_field_2"),
        value: String::from_str(&env, "xyz"),
    });

    let legacy = ProgramMetadata {
        program_name: Some(String::from_str(&env, "MaxKeys")),
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields,
    };

    let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);

    let legacy_size = encoded_size(&env, &legacy);
    let compressed_size = encoded_size(&env, &compressed);

    let savings_pct = if legacy_size > 0 {
        ((legacy_size - compressed_size) as f64 / legacy_size as f64 * 100.0)
    } else {
        0.0
    };

    eprintln!("=== Max-Keys Benchmark (10 known + 2 custom) ===");
    eprintln!("Legacy (String keys)   : {legacy_size} bytes");
    eprintln!("Compressed (enum keys) : {compressed_size} bytes");
    eprintln!("Savings                : {savings_pct:.1}%");
    assert!(
        compressed_size < legacy_size,
        "Compressed ({compressed_size}B) must be smaller than legacy ({legacy_size}B) for max-keys payload"
    );
    eprintln!("✓ Verified: {legacy_size}B → {compressed_size}B ({savings_pct:.1}% reduction)");
}

#[test]
fn benchmark_compression_custom_only() {
    let env = Env::default();

    // Only custom (non-matching) keys — compression adds enum discriminant
    // overhead so we should still be at parity or slightly worse (but never
    // catastrophically larger).  This tests the worst case.
    let mut custom_fields: Vec<ProgramMetadataField> = Vec::new(&env);
    for i in 0..5 {
        let key = alloc::format!("custom_key_{}", i);
        let val = alloc::format!("value_{}", i);
        custom_fields.push_back(ProgramMetadataField {
            key: String::from_str(&env, &key),
            value: String::from_str(&env, &val),
        });
    }

    let legacy = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields,
    };

    let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);

    let legacy_size = encoded_size(&env, &legacy);
    let compressed_size = encoded_size(&env, &compressed);

    eprintln!("=== Custom-Only Benchmark (all keys are Custom) ===");
    eprintln!("Legacy (String keys)   : {legacy_size} bytes");
    eprintln!("Compressed (enum keys) : {compressed_size} bytes");
    let diff = if compressed_size > legacy_size {
        alloc::format!("+{}B overhead", compressed_size - legacy_size)
    } else {
        alloc::format!("{}B savings", legacy_size - compressed_size)
    };
    eprintln!("Difference             : {diff}");
}

#[test]
fn benchmark_empty_custom_fields() {
    let env = Env::default();

    let legacy = ProgramMetadata {
        program_name: None,
        program_type: None,
        ecosystem: None,
        tags: Vec::new(&env),
        start_date: None,
        end_date: None,
        custom_fields: Vec::new(&env),
    };

    let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
    let legacy_size = encoded_size(&env, &legacy);
    let compressed_size = encoded_size(&env, &compressed);

    eprintln!("=== Empty Custom Fields ===");
    eprintln!("Legacy                 : {legacy_size} bytes");
    eprintln!("Compressed             : {compressed_size} bytes");
    // Should be identical or very close (only struct layout differs).
    assert!(
        (compressed_size as i64 - legacy_size as i64).unsigned_abs() <= 8,
        "Empty custom fields should have similar size (diff={})",
        compressed_size as i64 - legacy_size as i64
    );
}
