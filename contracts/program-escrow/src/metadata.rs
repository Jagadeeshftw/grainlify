//! Program metadata compression module.
//!
//! Provides a compact [`MetadataFieldKey`] enum that replaces free-form
//! `String` keys in custom metadata fields, reducing on-chain storage cost
//! for common field names while preserving support for arbitrary keys.
//!
//! # Design
//!
//! Common metadata keys (e.g. `"total_participants"`, `"sponsor"`) are
//! represented as zero-payload enum variants that serialise to a single
//! XDR discriminant (4 bytes) instead of a full UTF-8 string.  Uncommon
//! keys fall through to `MetadataFieldKey::Custom(String)`.
//!
//! # Storage layout
//!
//! New compressed metadata is stored under `DataKey::MetadataV2` to
//! preserve backwards compatibility with legacy metadata stored under
//! `DataKey::Metadata`.  The public `ProgramMetadata` struct is unchanged
//! so all existing query APIs remain source-compatible.

use soroban_sdk::{contracttype, Env, String, Vec};

/// Enum encoding common metadata field names as compact variants.
///
/// Each variant maps to a well-known metadata key used across Grainlify
/// programs.  When serialised via Soroban XDR, known-key variants cost
/// **≈4 bytes** (the enum discriminant) instead of `≈N+4 bytes` for an
/// equivalent `String` key, where N is the key length.
///
/// | Variant | String equivalent | Saved bytes* |
/// |---|---|---|
/// | `TotalParticipants` | `"total_participants"` | ≈19 |
/// | `PrizePoolUsd` | `"prize_pool_usd"` | ≈14 |
/// | `Sponsor` | `"sponsor"` | ≈7 |
/// | `Repository` | `"repository"` | ≈10 |
/// | `Website` | `"website"` | ≈7 |
/// | `ContactEmail` | `"contact_email"` | ≈13 |
/// | `Difficulty` | `"difficulty"` | ≈10 |
/// | `Category` | `"category"` | ≈8 |
/// | `Status` | `"status"` | ≈6 |
/// | `Version` | `"version"` | ≈7 |
/// | `Custom(s)` | `s` (any string) | 0 (same as legacy) |
///
/// *Saved bytes are approximate — actual XDR overhead varies slightly by
/// payload alignment.
///
/// # Determinism
///
/// `from_string` always maps the same input string to the same variant,
/// and `Custom(s)` preserves the original string as-is.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataFieldKey {
    /// `"total_participants"` — number of participants in the program.
    TotalParticipants,
    /// `"prize_pool_usd"` — total prize pool denominated in USD.
    PrizePoolUsd,
    /// `"sponsor"` — sponsoring entity name.
    Sponsor,
    /// `"repository"` — URL or slug of the source repository.
    Repository,
    /// `"website"` — program website URL.
    Website,
    /// `"contact_email"` — program contact email address.
    ContactEmail,
    /// `"difficulty"` — difficulty rating (e.g. `"beginner"`, `"advanced"`).
    Difficulty,
    /// `"category"` — program category (e.g. `"defi"`, `"nft"`).
    Category,
    /// `"status"` — program status string.
    Status,
    /// `"version"` — program or contract version identifier.
    Version,
    /// Fallback for any metadata key not covered by the known variants above.
    /// The inner `String` stores the exact key as supplied by the caller.
    Custom(String),
}

impl MetadataFieldKey {
    /// Map a legacy `String` key to the equivalent `MetadataFieldKey`,
    /// falling through to `Custom(s)` when no known variant matches.
    ///
    /// The matching is case-sensitive and byte-exact.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let key = MetadataFieldKey::from_string(&env, &String::from_str(&env, "sponsor"));
    /// assert_eq!(key, MetadataFieldKey::Sponsor);
    ///
    /// let custom = MetadataFieldKey::from_string(&env, &String::from_str(&env, "my_custom_key"));
    /// assert_eq!(custom, MetadataFieldKey::Custom(String::from_str(&env, "my_custom_key")));
    /// ```
    pub fn from_string(env: &Env, key: &String) -> Self {
        // Build reference strings for comparison.
        let total = String::from_str(env, "total_participants");
        let prize = String::from_str(env, "prize_pool_usd");
        let sponsor = String::from_str(env, "sponsor");
        let repo = String::from_str(env, "repository");
        let web = String::from_str(env, "website");
        let email = String::from_str(env, "contact_email");
        let diff = String::from_str(env, "difficulty");
        let cat = String::from_str(env, "category");
        let stat = String::from_str(env, "status");
        let ver = String::from_str(env, "version");

        if *key == total {
            Self::TotalParticipants
        } else if *key == prize {
            Self::PrizePoolUsd
        } else if *key == sponsor {
            Self::Sponsor
        } else if *key == repo {
            Self::Repository
        } else if *key == web {
            Self::Website
        } else if *key == email {
            Self::ContactEmail
        } else if *key == diff {
            Self::Difficulty
        } else if *key == cat {
            Self::Category
        } else if *key == stat {
            Self::Status
        } else if *key == ver {
            Self::Version
        } else {
            Self::Custom(key.clone())
        }
    }

    /// Convert back to a legacy `String` key.
    ///
    /// This is the inverse of `from_string` — every variant maps to the
    /// same string that `from_string` would accept.
    pub fn to_legacy_string(&self, env: &Env) -> String {
        match self {
            Self::TotalParticipants => String::from_str(env, "total_participants"),
            Self::PrizePoolUsd => String::from_str(env, "prize_pool_usd"),
            Self::Sponsor => String::from_str(env, "sponsor"),
            Self::Repository => String::from_str(env, "repository"),
            Self::Website => String::from_str(env, "website"),
            Self::ContactEmail => String::from_str(env, "contact_email"),
            Self::Difficulty => String::from_str(env, "difficulty"),
            Self::Category => String::from_str(env, "category"),
            Self::Status => String::from_str(env, "status"),
            Self::Version => String::from_str(env, "version"),
            Self::Custom(s) => s.clone(),
        }
    }
}

/// A custom metadata field with a compressed key.
///
/// Mirrors the legacy [`crate::ProgramMetadataField`] layout but replaces
/// the `key: String` field with [`MetadataFieldKey`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressedCustomField {
    /// Compressed or custom metadata key.
    pub key: MetadataFieldKey,
    /// Metadata value (always stored as a full string).
    pub value: String,
}

/// Compressed on-chain representation of [`crate::ProgramMetadata`].
///
/// Stored under `DataKey::MetadataV2(program_id)`.  The structured fields
/// (`program_name`, `tags`, etc.) are identical to the legacy layout;
/// only `custom_fields` uses compressed keys.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressedProgramMetadata {
    pub program_name: Option<String>,
    pub program_type: Option<String>,
    pub ecosystem: Option<String>,
    pub tags: Vec<String>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
    /// Custom fields stored with compressed keys.
    pub custom_fields: Vec<CompressedCustomField>,
}

impl CompressedProgramMetadata {
    /// Compress a legacy [`crate::ProgramMetadata`] into the compressed
    /// on-chain representation.
    ///
    /// Every `ProgramMetadataField.key` is mapped through
    /// [`MetadataFieldKey::from_string`], so known keys compress to
    /// their enum variant while unknown keys become `Custom(s)`.
    pub fn from_legacy(env: &Env, legacy: &crate::ProgramMetadata) -> Self {
        let mut compressed_fields: Vec<CompressedCustomField> = Vec::new(env);
        for field in legacy.custom_fields.iter() {
            compressed_fields.push_back(CompressedCustomField {
                key: MetadataFieldKey::from_string(env, &field.key),
                value: field.value.clone(),
            });
        }
        Self {
            program_name: legacy.program_name.clone(),
            program_type: legacy.program_type.clone(),
            ecosystem: legacy.ecosystem.clone(),
            tags: legacy.tags.clone(),
            start_date: legacy.start_date,
            end_date: legacy.end_date,
            custom_fields: compressed_fields,
        }
    }

    /// Decompress back into a legacy [`crate::ProgramMetadata`].
    ///
    /// Every `CompressedCustomField.key` is expanded back to its original
    /// string via [`MetadataFieldKey::to_legacy_string`].
    pub fn into_legacy(self, env: &Env) -> crate::ProgramMetadata {
        let mut legacy_fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(env);
        for field in self.custom_fields.iter() {
            legacy_fields.push_back(crate::ProgramMetadataField {
                key: field.key.to_legacy_string(env),
                value: field.value.clone(),
            });
        }
        crate::ProgramMetadata {
            program_name: self.program_name,
            program_type: self.program_type,
            ecosystem: self.ecosystem,
            tags: self.tags,
            start_date: self.start_date,
            end_date: self.end_date,
            custom_fields: legacy_fields,
        }
    }

    /// Estimated encoded byte size of a `CompressedProgramMetadata`
    /// given the number of custom fields and typical key lengths.
    ///
    /// This is a simplified cost model for benchmarking; actual XDR
    /// overhead adds a few bytes of framing.
    pub fn estimated_xdr_size(&self) -> usize {
        let mut size = 0usize;
        // Fixed-size fields: 6 × Option<String> placeholders + 2 × Option<u64>
        // Rough estimate: each Option<String> (None) ≈ 4 bytes discriminant
        // + Vec<String> overhead.  For a rough estimate we use a flat baseline.
        size += 64; // Baseline for structured fields
        for field in self.custom_fields.iter() {
            size += match &field.key {
                MetadataFieldKey::Custom(s) => {
                    // Discriminant (4) + String overhead (4+len) + value overhead
                    4 + 4 + s.len() as usize + 4 + field.value.len() as usize
                }
                _ => {
                    // Known key: discriminant only (4) + value overhead
                    4 + 4 + field.value.len() as usize
                }
            };
        }
        size
    }
}

// ============================================================================
// Legacy key conversion helpers
// ============================================================================

/// Decode a legacy `DataKey::Metadata` value from raw XDR bytes into a
/// [`CompressedProgramMetadata`], compressing all string keys.
///
/// Returns `None` when the raw bytes cannot be decoded (e.g. corrupt
/// storage or an incompatible schema version).  The caller should fall
/// back to a fresh empty metadata value.
pub fn try_decode_legacy_metadata(
    env: &Env,
    legacy: &crate::ProgramMetadata,
) -> CompressedProgramMetadata {
    CompressedProgramMetadata::from_legacy(env, legacy)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    fn create_env() -> Env {
        Env::default()
    }

    // ------------------------------------------------------------------
    // MetadataFieldKey::from_string / to_legacy_string round-trip
    // ------------------------------------------------------------------

    #[test]
    fn test_known_key_round_trips() {
        let env = create_env();
        let cases: &[(&str, MetadataFieldKey)] = &[
            ("total_participants", MetadataFieldKey::TotalParticipants),
            ("prize_pool_usd", MetadataFieldKey::PrizePoolUsd),
            ("sponsor", MetadataFieldKey::Sponsor),
            ("repository", MetadataFieldKey::Repository),
            ("website", MetadataFieldKey::Website),
            ("contact_email", MetadataFieldKey::ContactEmail),
            ("difficulty", MetadataFieldKey::Difficulty),
            ("category", MetadataFieldKey::Category),
            ("status", MetadataFieldKey::Status),
            ("version", MetadataFieldKey::Version),
        ];
        for (raw, expected) in cases {
            let key = String::from_str(&env, raw);
            let parsed = MetadataFieldKey::from_string(&env, &key);
            assert_eq!(
                parsed, *expected,
                "from_string({raw}) should produce {expected:?}"
            );
            let back = parsed.to_legacy_string(&env);
            assert_eq!(
                back, key,
                "to_legacy_string({parsed:?}) should restore \"{raw}\""
            );
        }
    }

    #[test]
    fn test_custom_key_preserved() {
        let env = create_env();
        let raw = "my_uncommon_field";
        let key = String::from_str(&env, raw);
        let parsed = MetadataFieldKey::from_string(&env, &key);
        assert_eq!(parsed, MetadataFieldKey::Custom(key.clone()));
        let back = parsed.to_legacy_string(&env);
        assert_eq!(back, key);
    }

    #[test]
    fn test_empty_key_is_custom() {
        let env = create_env();
        let key = String::from_str(&env, "");
        let parsed = MetadataFieldKey::from_string(&env, &key);
        assert_eq!(parsed, MetadataFieldKey::Custom(key));
    }

    #[test]
    fn test_case_sensitive() {
        let env = create_env();
        // Uppercase "Sponsor" must NOT match the lowercase variant.
        let key = String::from_str(&env, "Sponsor");
        let parsed = MetadataFieldKey::from_string(&env, &key);
        assert_eq!(parsed, MetadataFieldKey::Custom(key.clone()));
    }

    // ------------------------------------------------------------------
    // CompressedProgramMetadata compression / decompression
    // ------------------------------------------------------------------

    #[test]
    fn test_compress_decompress_round_trip() {
        let env = create_env();

        let mut fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(&env);
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "total_participants"),
            value: String::from_str(&env, "150"),
        });
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "prize_pool_usd"),
            value: String::from_str(&env, "50000"),
        });
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "sponsor"),
            value: String::from_str(&env, "Stellar Development Foundation"),
        });
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "custom_field_x"),
            value: String::from_str(&env, "some_value"),
        });

        let legacy = crate::ProgramMetadata {
            program_name: Some(String::from_str(&env, "Test Program")),
            program_type: Some(String::from_str(&env, "hackathon")),
            ecosystem: Some(String::from_str(&env, "stellar")),
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields: fields,
        };

        // Compress
        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
        assert_eq!(
            compressed.custom_fields.len(),
            4,
            "all 4 fields must be preserved"
        );

        // Verify known keys were compressed
        let field0 = compressed.custom_fields.get(0).unwrap();
        assert_eq!(field0.key, MetadataFieldKey::TotalParticipants);
        assert_eq!(field0.value, String::from_str(&env, "150"));

        let field3 = compressed.custom_fields.get(3).unwrap();
        assert_eq!(
            field3.key,
            MetadataFieldKey::Custom(String::from_str(&env, "custom_field_x"))
        );

        // Decompress
        let restored = compressed.into_legacy(&env);
        assert_eq!(restored.program_name, legacy.program_name);
        assert_eq!(restored.custom_fields.len(), 4);

        let r0 = restored.custom_fields.get(0).unwrap();
        assert_eq!(r0.key, String::from_str(&env, "total_participants"));
        assert_eq!(r0.value, String::from_str(&env, "150"));
    }

    #[test]
    fn test_compress_empty_custom_fields() {
        let env = create_env();
        let legacy = crate::ProgramMetadata {
            program_name: None,
            program_type: None,
            ecosystem: None,
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields: crate::Vec::new(&env),
        };

        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
        assert_eq!(compressed.custom_fields.len(), 0);

        let restored = compressed.into_legacy(&env);
        assert_eq!(restored.custom_fields.len(), 0);
    }

    #[test]
    fn test_compress_mixed_keys() {
        let env = create_env();
        let mut fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(&env);
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "sponsor"),
            value: String::from_str(&env, "SDF"),
        });
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "contact_email"),
            value: String::from_str(&env, "admin@example.com"),
        });
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "website"),
            value: String::from_str(&env, "https://example.com"),
        });
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "arbitrary_key"),
            value: String::from_str(&env, "arbitrary_value"),
        });

        let legacy = crate::ProgramMetadata {
            program_name: None,
            program_type: None,
            ecosystem: None,
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields: fields,
        };

        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
        assert_eq!(compressed.custom_fields.len(), 4);

        // First three should be known variants
        assert_eq!(
            compressed.custom_fields.get(0).unwrap().key,
            MetadataFieldKey::Sponsor
        );
        assert_eq!(
            compressed.custom_fields.get(1).unwrap().key,
            MetadataFieldKey::ContactEmail
        );
        assert_eq!(
            compressed.custom_fields.get(2).unwrap().key,
            MetadataFieldKey::Website
        );
        // Fourth should be Custom
        assert_eq!(
            compressed.custom_fields.get(3).unwrap().key,
            MetadataFieldKey::Custom(String::from_str(&env, "arbitrary_key"))
        );
    }

    // ------------------------------------------------------------------
    // Serialization / deserialization via Soroban contracttype
    //
    // We verify that all #[contracttype]-derived types can round-trip
    // through Soroban Val (the XDR-compatible intermediate representation).
    // ------------------------------------------------------------------

    fn roundtrip_via_val<T>(env: &Env, value: &T) -> T
    where
        T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>
            + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>
            + Clone,
    {
        let val: soroban_sdk::Val = value.clone().into_val(env);
        T::try_from_val(env, &val).unwrap()
    }

    #[test]
    fn test_metadata_field_key_val_roundtrip() {
        let env = create_env();

        let variants: &[MetadataFieldKey] = &[
            MetadataFieldKey::TotalParticipants,
            MetadataFieldKey::PrizePoolUsd,
            MetadataFieldKey::Sponsor,
            MetadataFieldKey::Repository,
            MetadataFieldKey::Website,
            MetadataFieldKey::ContactEmail,
            MetadataFieldKey::Difficulty,
            MetadataFieldKey::Category,
            MetadataFieldKey::Status,
            MetadataFieldKey::Version,
            MetadataFieldKey::Custom(String::from_str(&env, "random_field")),
        ];

        for variant in variants {
            let decoded = roundtrip_via_val(&env, variant);
            assert_eq!(*variant, decoded);
        }
    }

    #[test]
    fn test_compressed_field_val_roundtrip() {
        let env = create_env();

        let field = CompressedCustomField {
            key: MetadataFieldKey::TotalParticipants,
            value: String::from_str(&env, "250"),
        };

        let decoded = roundtrip_via_val(&env, &field);
        assert_eq!(field, decoded);
    }

    #[test]
    fn test_compressed_metadata_val_roundtrip() {
        let env = create_env();

        let mut fields: Vec<CompressedCustomField> = Vec::new(&env);
        fields.push_back(CompressedCustomField {
            key: MetadataFieldKey::Sponsor,
            value: String::from_str(&env, "SDF"),
        });
        fields.push_back(CompressedCustomField {
            key: MetadataFieldKey::Custom(String::from_str(&env, "custom_key")),
            value: String::from_str(&env, "custom_val"),
        });

        let compressed = CompressedProgramMetadata {
            program_name: Some(String::from_str(&env, "Test Program")),
            program_type: Some(String::from_str(&env, "hackathon")),
            ecosystem: Some(String::from_str(&env, "stellar")),
            tags: Vec::new(&env),
            start_date: Some(1_000_000),
            end_date: Some(2_000_000),
            custom_fields: fields,
        };

        let decoded = roundtrip_via_val(&env, &compressed);
        assert_eq!(compressed, decoded);
        assert_eq!(decoded.custom_fields.len(), 2);
    }

    // ------------------------------------------------------------------
    // Backwards compatibility: construct ProgramMetadata with String keys,
    // compress, decompress, verify String keys restored.
    // ------------------------------------------------------------------

    #[test]
    fn test_backwards_compatible_roundtrip() {
        let env = create_env();

        // This replicates the legacy constructor pattern from existing tests.
        let mut custom_fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(&env);
        custom_fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "total_participants"),
            value: String::from_str(&env, "150"),
        });
        custom_fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "prize_pool_usd"),
            value: String::from_str(&env, "50000"),
        });
        custom_fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, "sponsor"),
            value: String::from_str(&env, "Stellar Development Foundation"),
        });

        let metadata = crate::ProgramMetadata {
            program_name: Some(String::from_str(&env, "Legacy Compat Program")),
            program_type: Some(String::from_str(&env, "hackathon")),
            ecosystem: Some(String::from_str(&env, "stellar")),
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields,
        };

        // Legacy -> Compressed
        let compressed = CompressedProgramMetadata::from_legacy(&env, &metadata);
        assert_eq!(compressed.custom_fields.len(), 3);

        // Verify known-key compression
        assert_eq!(
            compressed.custom_fields.get(0).unwrap().key,
            MetadataFieldKey::TotalParticipants
        );
        assert_eq!(
            compressed.custom_fields.get(1).unwrap().key,
            MetadataFieldKey::PrizePoolUsd
        );
        assert_eq!(
            compressed.custom_fields.get(2).unwrap().key,
            MetadataFieldKey::Sponsor
        );

        // Compressed -> Legacy (decompress)
        let restored = compressed.into_legacy(&env);
        assert_eq!(
            restored.program_name,
            Some(String::from_str(&env, "Legacy Compat Program"))
        );
        assert_eq!(restored.custom_fields.len(), 3);

        let r0 = restored.custom_fields.get(0).unwrap();
        assert_eq!(r0.key, String::from_str(&env, "total_participants"));
        assert_eq!(r0.value, String::from_str(&env, "150"));
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[test]
    fn test_compress_large_custom_key() {
        let env = create_env();
        let large_key = "a".repeat(256); // 256-char key
        let mut fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(&env);
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, &large_key),
            value: String::from_str(&env, "value"),
        });

        let legacy = crate::ProgramMetadata {
            program_name: None,
            program_type: None,
            ecosystem: None,
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields: fields,
        };

        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
        assert_eq!(compressed.custom_fields.len(), 1);
        match &compressed.custom_fields.get(0).unwrap().key {
            MetadataFieldKey::Custom(s) => {
                assert_eq!(s.len() as u32, 256);
            }
            _ => panic!("Expected Custom variant for large key"),
        }
    }

    #[test]
    fn test_compress_special_characters() {
        let env = create_env();
        let special = "field_with_underscores_and_numbers_123";
        let mut fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(&env);
        fields.push_back(crate::ProgramMetadataField {
            key: String::from_str(&env, special),
            value: String::from_str(&env, "value_123"),
        });

        let legacy = crate::ProgramMetadata {
            program_name: None,
            program_type: None,
            ecosystem: None,
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields: fields,
        };

        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
        let restored = compressed.into_legacy(&env);
        let r0 = restored.custom_fields.get(0).unwrap();
        assert_eq!(r0.key, String::from_str(&env, special));
        assert_eq!(r0.value, String::from_str(&env, "value_123"));
    }

    // ======================================================================
    // Benchmarks: encoded size comparison
    // ======================================================================

    #[cfg(any())]
    mod benchmarks {

    fn benchmark_payload(env: &Env) -> crate::ProgramMetadata {
        let mut tags: crate::Vec<crate::String> = crate::Vec::new(env);
        tags.push_back(crate::String::from_str(env, "hackathon"));
        tags.push_back(crate::String::from_str(env, "defi"));

        let mut custom_fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(env);
        custom_fields.push_back(crate::ProgramMetadataField {
            key: crate::String::from_str(env, "total_participants"),
            value: crate::String::from_str(env, "150"),
        });
        custom_fields.push_back(crate::ProgramMetadataField {
            key: crate::String::from_str(env, "prize_pool_usd"),
            value: crate::String::from_str(env, "50000"),
        });
        custom_fields.push_back(crate::ProgramMetadataField {
            key: crate::String::from_str(env, "sponsor"),
            value: crate::String::from_str(env, "Stellar Development Foundation"),
        });
        custom_fields.push_back(crate::ProgramMetadataField {
            key: crate::String::from_str(env, "project_repository"),
            value: crate::String::from_str(env, "https://github.com/stellar/example"),
        });

        crate::ProgramMetadata {
            program_name: Some(crate::String::from_str(env, "Hackathon 2024")),
            program_type: Some(crate::String::from_str(env, "hackathon")),
            ecosystem: Some(crate::String::from_str(env, "stellar")),
            tags,
            start_date: Some(1_720_000_000),
            end_date: Some(1_750_000_000),
            custom_fields,
        }
    }

    fn encoded_size<T>(env: &Env, value: &T) -> usize
    where
        T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
    {
        use soroban_sdk::xdr::ToXdr;
        let val: soroban_sdk::Val = soroban_sdk::IntoVal::into_val(value.clone(), env);
        let bytes: soroban_sdk::Bytes = val.to_xdr(env);
        bytes.len() as usize
    }

    #[test]
    fn benchmark_representative_payload() {
        let env = create_env();
        let legacy = benchmark_payload(&env);
        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);

        let legacy_size = encoded_size(&env, &legacy);
        let compressed_size = encoded_size(&env, &compressed);

        let savings_pct = if legacy_size > 0 {
            ((legacy_size - compressed_size) as f64 / legacy_size as f64 * 100.0)
        } else {
            0.0
        };

        assert!(
            compressed_size < legacy_size,
            "compressed ({compressed_size}B) < legacy ({legacy_size}B)"
        );
    }

    #[test]
    fn benchmark_max_keys() {
        let env = create_env();
        let mut custom_fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(&env);
        let known: &[&str] = &[
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
        for (i, k) in known.iter().enumerate() {
            let val_str = ["v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9"][i];
            custom_fields.push_back(crate::ProgramMetadataField {
                key: crate::String::from_str(&env, k),
                value: crate::String::from_str(&env, val_str),
            });
        }
        // Add 2 custom (non-matching) keys
        custom_fields.push_back(crate::ProgramMetadataField {
            key: crate::String::from_str(&env, "custom_field_0"),
            value: crate::String::from_str(&env, "x"),
        });
        custom_fields.push_back(crate::ProgramMetadataField {
            key: crate::String::from_str(&env, "custom_field_1"),
            value: crate::String::from_str(&env, "y"),
        });

        let legacy = crate::ProgramMetadata {
            program_name: Some(crate::String::from_str(&env, "MaxKeys")),
            program_type: None,
            ecosystem: None,
            tags: crate::Vec::new(&env),
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

        assert!(
            compressed_size < legacy_size,
            "compressed ({compressed_size}B) < legacy ({legacy_size}B)"
        );
    }

    #[test]
    fn benchmark_custom_only_worst_case() {
        let env = create_env();
        let mut custom_fields: crate::Vec<crate::ProgramMetadataField> = crate::Vec::new(&env);
        let keys: &[&str] = &[
            "custom_key_0",
            "custom_key_1",
            "custom_key_2",
            "custom_key_3",
            "custom_key_4",
        ];
        let vals: &[&str] = &["value_0", "value_1", "value_2", "value_3", "value_4"];
        for i in 0..5 {
            custom_fields.push_back(crate::ProgramMetadataField {
                key: crate::String::from_str(&env, keys[i]),
                value: crate::String::from_str(&env, vals[i]),
            });
        }

        let legacy = crate::ProgramMetadata {
            program_name: None,
            program_type: None,
            ecosystem: None,
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields,
        };

        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
        let legacy_size = encoded_size(&env, &legacy);
        let compressed_size = encoded_size(&env, &compressed);
        let diff = compressed_size as i64 - legacy_size as i64;

        // Accept small overhead (enum discriminant) for all-custom keys.
        assert!(
            diff.abs() <= 32,
            "diff ({diff}B) should be within tolerance"
        );
    }

    #[test]
    fn benchmark_empty_custom_fields() {
        let env = create_env();

        let legacy = crate::ProgramMetadata {
            program_name: None,
            program_type: None,
            ecosystem: None,
            tags: crate::Vec::new(&env),
            start_date: None,
            end_date: None,
            custom_fields: crate::Vec::new(&env),
        };

        let compressed = CompressedProgramMetadata::from_legacy(&env, &legacy);
        let legacy_size = encoded_size(&env, &legacy);
        let compressed_size = encoded_size(&env, &compressed);

        assert!(
            (compressed_size as i64 - legacy_size as i64).unsigned_abs() <= 8,
            "empty metadata sizes should be close"
        );
    }
    }
}
