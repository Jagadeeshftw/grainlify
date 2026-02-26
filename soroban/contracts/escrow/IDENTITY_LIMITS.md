# Identity-Aware Limits

## Overview

This document describes the identity-aware transaction limits feature for the Grainlify escrow contract. This feature enables regulatory compliance and risk management by enforcing different transaction limits based on user verification levels, without storing sensitive personal information on-chain.

## Architecture

The system uses cryptographically signed identity claims issued by trusted KYC providers. These claims associate blockchain addresses with:
- **Identity Tiers**: Unverified, Basic, Verified, Premium
- **Risk Scores**: 0-100 numerical risk assessment
- **Expiry Timestamps**: Claim validity period

## Identity Tiers

### Tier Levels

1. **Unverified (Tier 0)**
   - Default tier for all addresses
   - Lowest transaction limits
   - No KYC verification required
   - Default limit: 100 tokens

2. **Basic (Tier 1)**
   - Basic identity verification
   - Moderate transaction limits
   - Default limit: 1,000 tokens

3. **Verified (Tier 2)**
   - Full identity verification
   - High transaction limits
   - Default limit: 10,000 tokens

4. **Premium (Tier 3)**
   - Enhanced verification with additional checks
   - Highest transaction limits
   - Default limit: 100,000 tokens

## Risk Scores

Risk scores (0-100) provide additional granularity for limit enforcement:

- **Low Risk (0-69)**: Standard tier-based limits apply
- **High Risk (70-100)**: Reduced limits based on risk multiplier

### Risk-Adjusted Limits

When a risk score exceeds the high-risk threshold (default: 70), the effective limit is calculated as:

```
effective_limit = tier_limit * (risk_multiplier / 100)
```

Default risk multiplier: 50% (high-risk users get 50% of their tier limit)

## Identity Claims

### Claim Structure

```rust
pub struct IdentityClaim {
    pub address: Address,      // User's blockchain address
    pub tier: IdentityTier,    // Identity verification tier
    pub risk_score: u32,       // Risk assessment (0-100)
    pub expiry: u64,           // Unix timestamp
    pub issuer: Address,       // Issuer's public key
}
```

### Claim Lifecycle

1. **Creation**: Off-chain KYC provider creates claim after verification
2. **Signing**: Claim is signed with issuer's Ed25519 private key
3. **Submission**: User submits claim to contract with signature
4. **Verification**: Contract verifies signature and issuer authorization
5. **Storage**: Valid claims are stored on-chain
6. **Expiry**: Expired claims revert address to unverified tier

## Contract Functions

### Admin Functions

#### `set_authorized_issuer(issuer: Address, authorized: bool)`
Authorize or revoke a claim issuer. Only callable by contract admin.

**Parameters:**
- `issuer`: Address of the claim issuer
- `authorized`: true to authorize, false to revoke

**Events:**
- Emits issuer management event with action (add/remove)

#### `set_tier_limits(unverified, basic, verified, premium: i128)`
Configure transaction limits for each tier. Only callable by contract admin.

**Parameters:**
- `unverified`: Limit for unverified tier (in stroops)
- `basic`: Limit for basic tier
- `verified`: Limit for verified tier
- `premium`: Limit for premium tier

#### `set_risk_thresholds(high_risk_threshold: u32, high_risk_multiplier: u32)`
Configure risk-based limit adjustments. Only callable by contract admin.

**Parameters:**
- `high_risk_threshold`: Risk score threshold for high-risk classification (0-100)
- `high_risk_multiplier`: Percentage multiplier for high-risk limits (0-100)

### User Functions

#### `submit_identity_claim(claim: IdentityClaim, signature: BytesN<64>, issuer_pubkey: BytesN<32>)`
Submit an identity claim for verification and storage.

**Parameters:**
- `claim`: The identity claim structure
- `signature`: Ed25519 signature from authorized issuer
- `issuer_pubkey`: Public key of the issuer (32 bytes)

**Validation:**
- Claim must be signed by authorized issuer
- Claim must not be expired
- Risk score must be 0-100
- Signature must be valid

**Events:**
- Success: Emits claim event with tier, risk score, and expiry
- Failure: Emits rejection event with reason

**Errors:**
- `InvalidSignature`: Signature verification failed
- `ClaimExpired`: Claim expiry timestamp has passed
- `UnauthorizedIssuer`: Issuer is not authorized
- `InvalidRiskScore`: Risk score exceeds 100

#### `get_address_identity(address: Address) -> AddressIdentity`
Query the current identity data for an address.

**Returns:**
- `AddressIdentity` with tier, risk score, expiry, and last updated timestamp
- Returns default unverified tier if no claim exists or claim is expired

#### `get_effective_limit(address: Address) -> i128`
Query the effective transaction limit for an address.

**Returns:**
- Transaction limit in stroops, calculated from tier and risk score
- Returns unverified limit if no claim exists

#### `is_claim_valid(address: Address) -> bool`
Check if an address has a valid (non-expired) claim.

**Returns:**
- `true` if claim exists and is not expired
- `false` if no claim or claim is expired

## Limit Enforcement

Transaction limits are enforced in:
- `lock_funds`: Depositor must be within their limit
- `release_funds`: Contributor must be within their limit

### Enforcement Logic

```rust
fn enforce_transaction_limit(address: &Address, amount: i128) -> Result<(), Error> {
    let effective_limit = get_effective_limit(address);
    
    if amount > effective_limit {
        return Err(Error::TransactionExceedsLimit);
    }
    
    Ok(())
}
```

### Events

Limit enforcement emits events:
- **Pass**: Transaction within limit
- **Exceed**: Transaction exceeds limit (transaction rejected)

## Off-Chain Integration

### Go Helper Package

The `backend/internal/identity` package provides off-chain claim management:

```go
// Create a new claim
claim, err := identity.CreateClaim(
    address,
    identity.TierVerified,
    25, // risk score
    time.Hour * 24 * 30, // 30 days validity
)

// Sign the claim
signature, err := identity.SignClaim(claim, privateKey)

// Verify the claim
err = identity.VerifyClaim(claim, signature, publicKey)
```

### Claim Serialization

Claims are serialized deterministically for signature verification:

1. Address bytes (XDR encoded)
2. Tier (4 bytes, big-endian)
3. Risk score (4 bytes, big-endian)
4. Expiry (8 bytes, big-endian)
5. Issuer bytes (XDR encoded)

Both on-chain (Rust) and off-chain (Go) implementations use the same serialization format to ensure signature compatibility.

## Security Considerations

### Trust Model

1. **Issuer Trust**: The system trusts authorized issuers to perform proper KYC verification
2. **Signature Security**: Ed25519 signatures provide cryptographic proof of claim authenticity
3. **Admin Trust**: Contract admin is trusted to authorize only legitimate issuers
4. **Time Accuracy**: Ledger timestamps are used for expiry checks

### Attack Vectors

1. **Signature Forgery**: Mitigated by Ed25519 cryptographic signatures
2. **Replay Attacks**: Mitigated by claim expiry timestamps
3. **Claim Tampering**: Any modification invalidates the signature
4. **Unauthorized Issuers**: Only authorized issuers can sign valid claims
5. **Limit Bypass**: Limits enforced on all fund operations

### Privacy

- No personal information stored on-chain
- Only tier and risk score are stored
- Claims can be verified without revealing identity
- Off-chain KYC data remains with provider

## Configuration Examples

### Setting Up Issuers

```rust
// Authorize a KYC provider
client.set_authorized_issuer(&issuer_address, &true);

// Revoke an issuer
client.set_authorized_issuer(&old_issuer_address, &false);
```

### Configuring Limits

```rust
// Set tier limits (in stroops, 7 decimals)
client.set_tier_limits(
    &100_0000000,      // Unverified: 100 tokens
    &1000_0000000,     // Basic: 1,000 tokens
    &10000_0000000,    // Verified: 10,000 tokens
    &100000_0000000,   // Premium: 100,000 tokens
);

// Set risk thresholds
client.set_risk_thresholds(
    &70,  // High risk threshold
    &50,  // 50% multiplier for high-risk users
);
```

### Submitting Claims

```rust
// Create claim off-chain (Go)
claim := &identity.IdentityClaim{
    Address:   user_address,
    Tier:      identity.TierVerified,
    RiskScore: 25,
    Expiry:    uint64(time.Now().Add(30 * 24 * time.Hour).Unix()),
    Issuer:    issuer_address,
}

// Sign claim
signature, _ := identity.SignClaim(claim, issuer_private_key)

// Submit to contract (Rust)
client.submit_identity_claim(&claim, &signature, &issuer_pubkey);
```

## Testing

### Unit Tests

The contract includes comprehensive unit tests:
- Issuer authorization management
- Tier limits configuration
- Risk thresholds configuration
- Default identity queries
- Effective limit calculations
- Limit enforcement in transactions

### Running Tests

```bash
cd soroban
cargo test --manifest-path contracts/escrow/Cargo.toml
```

## Migration Guide

### For Existing Deployments

1. Deploy new identity-aware contract
2. Migrate existing escrows to new contract
3. All addresses start as unverified tier
4. Users submit claims to upgrade tiers
5. Gradually deprecate old contract

### Configuration Updates

- Tier limits can be updated without migration
- Risk thresholds can be adjusted based on operational experience
- Issuers can be added/removed as needed
- No contract redeployment required for configuration changes

## Troubleshooting

### Common Issues

1. **Claim Rejected - Invalid Signature**
   - Verify claim serialization matches on-chain format
   - Ensure issuer private key matches authorized public key
   - Check that claim data hasn't been modified after signing

2. **Claim Rejected - Expired**
   - Check claim expiry timestamp
   - Ensure expiry is in the future when creating claim
   - Verify ledger timestamp is accurate

3. **Claim Rejected - Unauthorized Issuer**
   - Verify issuer is authorized via `set_authorized_issuer`
   - Check issuer address matches claim issuer field

4. **Transaction Exceeds Limit**
   - Query effective limit with `get_effective_limit`
   - Check identity tier with `get_address_identity`
   - Verify risk score isn't reducing limit
   - Submit higher-tier claim if eligible

## Future Enhancements

Potential improvements for future versions:
- Multiple concurrent claims per address
- Time-based transaction limits (daily/weekly)
- Graduated limits based on transaction history
- Automated claim renewal
- Multi-signature claim issuance
- Claim revocation mechanism
