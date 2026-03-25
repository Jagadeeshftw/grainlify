# Metadata Constraints

This document defines on-chain validation rules for escrow metadata stored by the
bounty escrow contract.

## Bounty Type (Tag)

The `bounty_type` field is treated as a short human-readable tag. It is validated
when metadata is updated.

Constraints:
- Must be non-empty.
- Maximum length is 50 characters.

Notes:
- Character-set validation is intentionally conservative today and may be
  tightened in a future SDK update. Consumers should avoid control characters
  and prefer simple, human-readable labels.

## Reference Hash

`reference_hash` is optional and currently not validated on-chain. Off-chain
systems should define and enforce any expected format or length.
