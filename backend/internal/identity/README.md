# Identity Claims Package

This package provides off-chain utilities for creating, signing, and verifying identity claims for the Grainlify escrow system's identity-aware limits feature.

## Overview

The identity claims system enables KYC providers to issue cryptographically signed claims that associate blockchain addresses with identity tiers and risk scores. These claims are then submitted to the on-chain escrow contract for verification and limit enforcement.

## Installation

This package is part of the Grainlify backend and uses Go's standard library along with `crypto/ed25519` for signature operations.

```go
import "github.com/Jagadeeshftw/grainlify/backend/internal/identity"
```

## Types

### IdentityTier

```go
type IdentityTier uint32

const (
    TierUnverified IdentityTier = 0  // Default tier, lowest limits
    TierBasic      IdentityTier = 1  // Basic verification
    TierVerified   IdentityTier = 2  // Full verification
    TierPremium    IdentityTier = 3  // Enhanced verification
)
```

### IdentityClaim

```go
type IdentityClaim struct {
    Address   string       // Stellar address
    Tier      IdentityTier // Identity tier level
    RiskScore uint32       // Risk score (0-100)
    Expiry    uint64       // Unix timestamp
    Issuer    string       // Issuer public key address
}
```

## Functions

### CreateClaim

Creates a new identity claim with the given parameters.

```go
func CreateClaim(
    address string,
    tier IdentityTier,
    riskScore uint32,
    validityDuration time.Duration,
) (*IdentityClaim, error)
```

**Parameters:**
- `address`: The Stellar address for the claim
- `tier`: The identity tier level
- `riskScore`: Risk assessment score (0-100)
- `validityDuration`: How long the claim should be valid

**Returns:**
- `*IdentityClaim`: The created claim
- `error`: Error if validation fails

**Example:**
```go
claim, err := identity.CreateClaim(
    "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    identity.TierVerified,
    25,
    30 * 24 * time.Hour, // 30 days
)
if err != nil {
    log.Fatal(err)
}
```

### SerializeClaim

Serializes a claim for signing using deterministic encoding.

```go
func SerializeClaim(claim *IdentityClaim) ([]byte, error)
```

**Parameters:**
- `claim`: The claim to serialize

**Returns:**
- `[]byte`: Serialized claim bytes
- `error`: Error if serialization fails

**Note:** The serialization format matches the on-chain contract to ensure signature compatibility.

### SignClaim

Signs a claim with the issuer's Ed25519 private key.

```go
func SignClaim(claim *IdentityClaim, privateKey ed25519.PrivateKey) ([]byte, error)
```

**Parameters:**
- `claim`: The claim to sign
- `privateKey`: The issuer's Ed25519 private key

**Returns:**
- `[]byte`: 64-byte Ed25519 signature
- `error`: Error if signing fails

**Example:**
```go
signature, err := identity.SignClaim(claim, issuerPrivateKey)
if err != nil {
    log.Fatal(err)
}
```

### VerifyClaim

Verifies a claim signature using the issuer's public key.

```go
func VerifyClaim(claim *IdentityClaim, signature []byte, publicKey ed25519.PublicKey) error
```

**Parameters:**
- `claim`: The claim to verify
- `signature`: The signature to verify
- `publicKey`: The issuer's Ed25519 public key

**Returns:**
- `error`: Error if verification fails, nil if valid

**Example:**
```go
err := identity.VerifyClaim(claim, signature, issuerPublicKey)
if err != nil {
    log.Printf("Invalid signature: %v", err)
}
```

### Claim Methods

#### IsExpired

```go
func (c *IdentityClaim) IsExpired() bool
```

Checks if a claim has expired based on the current time.

#### Validate

```go
func (c *IdentityClaim) Validate() error
```

Validates a claim's fields (risk score, expiry, address, issuer).

## Test Utilities

### GenerateTestKeyPair

Generates a new Ed25519 key pair for testing.

```go
func GenerateTestKeyPair() (*TestKeyPair, error)
```

**Returns:**
- `*TestKeyPair`: Contains PublicKey and PrivateKey
- `error`: Error if generation fails

### GenerateTestClaim

Creates a test claim with a valid signature.

```go
func GenerateTestClaim(
    address string,
    tier IdentityTier,
    riskScore uint32,
    keyPair *TestKeyPair,
) (*IdentityClaim, []byte, error)
```

**Returns:**
- `*IdentityClaim`: The generated claim
- `[]byte`: The signature
- `error`: Error if generation fails

### GenerateTestClaimWithExpiry

Creates a test claim with a custom expiry timestamp.

```go
func GenerateTestClaimWithExpiry(
    address string,
    tier IdentityTier,
    riskScore uint32,
    expiry uint64,
    keyPair *TestKeyPair,
) (*IdentityClaim, []byte, error)
```

## Usage Examples

### Complete Workflow

```go
package main

import (
    "crypto/ed25519"
    "crypto/rand"
    "log"
    "time"
    
    "github.com/Jagadeeshftw/grainlify/backend/internal/identity"
)

func main() {
    // Generate issuer key pair
    publicKey, privateKey, err := ed25519.GenerateKey(rand.Reader)
    if err != nil {
        log.Fatal(err)
    }
    
    // Create a claim
    claim, err := identity.CreateClaim(
        "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
        identity.TierVerified,
        25,
        30 * 24 * time.Hour,
    )
    if err != nil {
        log.Fatal(err)
    }
    
    // Set issuer
    claim.Issuer = "GISSUERXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
    
    // Sign the claim
    signature, err := identity.SignClaim(claim, privateKey)
    if err != nil {
        log.Fatal(err)
    }
    
    // Verify the claim
    err = identity.VerifyClaim(claim, signature, publicKey)
    if err != nil {
        log.Fatal("Invalid signature:", err)
    }
    
    log.Println("Claim created and verified successfully")
    log.Printf("Tier: %d, Risk Score: %d, Expiry: %d", claim.Tier, claim.RiskScore, claim.Expiry)
}
```

### Testing Example

```go
package main

import (
    "testing"
    
    "github.com/Jagadeeshftw/grainlify/backend/internal/identity"
)

func TestClaimCreation(t *testing.T) {
    // Generate test key pair
    keyPair, err := identity.GenerateTestKeyPair()
    if err != nil {
        t.Fatal(err)
    }
    
    // Generate test claim
    claim, signature, err := identity.GenerateTestClaim(
        "GTEST",
        identity.TierBasic,
        50,
        keyPair,
    )
    if err != nil {
        t.Fatal(err)
    }
    
    // Verify the claim
    err = identity.VerifyClaim(claim, signature, keyPair.PublicKey)
    if err != nil {
        t.Errorf("Claim verification failed: %v", err)
    }
}
```

### Integration with KYC Provider

```go
func HandleKYCWebhook(w http.ResponseWriter, r *http.Request) {
    // Parse KYC verification result
    var kycResult KYCResult
    if err := json.NewDecoder(r.Body).Decode(&kycResult); err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }
    
    // Determine tier based on KYC level
    var tier identity.IdentityTier
    switch kycResult.VerificationLevel {
    case "basic":
        tier = identity.TierBasic
    case "verified":
        tier = identity.TierVerified
    case "premium":
        tier = identity.TierPremium
    default:
        tier = identity.TierUnverified
    }
    
    // Calculate risk score (example logic)
    riskScore := calculateRiskScore(kycResult)
    
    // Create claim
    claim, err := identity.CreateClaim(
        kycResult.StellarAddress,
        tier,
        riskScore,
        90 * 24 * time.Hour, // 90 days validity
    )
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    
    // Set issuer
    claim.Issuer = config.IssuerAddress
    
    // Sign claim
    signature, err := identity.SignClaim(claim, config.IssuerPrivateKey)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    
    // Return claim and signature to user
    response := ClaimResponse{
        Claim:     claim,
        Signature: signature,
    }
    
    json.NewEncoder(w).Encode(response)
}
```

## Serialization Format

The claim serialization format is deterministic and matches the on-chain contract:

1. **Address bytes**: Variable length, raw bytes
2. **Tier**: 4 bytes, big-endian uint32
3. **Risk Score**: 4 bytes, big-endian uint32
4. **Expiry**: 8 bytes, big-endian uint64
5. **Issuer bytes**: Variable length, raw bytes

This ensures that signatures created off-chain can be verified on-chain.

## Security Considerations

### Private Key Management

- Store issuer private keys securely (HSM, key vault, etc.)
- Never expose private keys in logs or responses
- Rotate keys periodically
- Use separate keys for different environments

### Claim Validation

- Always validate risk scores (0-100)
- Ensure expiry is in the future
- Verify addresses are valid Stellar addresses
- Check claim hasn't been tampered with after signing

### Signature Verification

- Always verify signatures before trusting claims
- Use constant-time comparison for signature verification
- Validate public keys match authorized issuers

## Error Handling

Common errors and their meanings:

- `"risk score must be between 0 and 100"`: Invalid risk score provided
- `"expiry must be in the future"`: Claim expiry is not in the future
- `"invalid signature"`: Signature verification failed
- `"claim has expired"`: Claim expiry timestamp has passed
- `"address cannot be empty"`: Address field is empty
- `"issuer cannot be empty"`: Issuer field is empty

## Testing

Run tests with:

```bash
cd backend
go test ./internal/identity/...
```

## Contributing

When contributing to this package:

1. Maintain serialization format compatibility with on-chain contract
2. Add tests for new functionality
3. Document all public functions
4. Follow Go best practices and conventions
5. Ensure backward compatibility

## License

This package is part of the Grainlify project. See the main repository for license information.
