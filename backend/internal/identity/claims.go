package identity

import (
	"crypto/ed25519"
	"encoding/binary"
	"errors"
	"time"
)

// IdentityTier represents the KYC verification level
type IdentityTier uint32

const (
	TierUnverified IdentityTier = 0
	TierBasic      IdentityTier = 1
	TierVerified   IdentityTier = 2
	TierPremium    IdentityTier = 3
)

// IdentityClaim represents an off-chain identity claim
type IdentityClaim struct {
	Address   string       // Stellar address
	Tier      IdentityTier // Identity tier level
	RiskScore uint32       // Risk score (0-100)
	Expiry    uint64       // Unix timestamp
	Issuer    string       // Issuer public key address
}

// CreateClaim creates a new identity claim with the given parameters
func CreateClaim(
	address string,
	tier IdentityTier,
	riskScore uint32,
	validityDuration time.Duration,
) (*IdentityClaim, error) {
	// Validate risk score
	if riskScore > 100 {
		return nil, errors.New("risk score must be between 0 and 100")
	}

	// Calculate expiry timestamp
	expiry := uint64(time.Now().Add(validityDuration).Unix())

	// Ensure expiry is in the future
	if expiry <= uint64(time.Now().Unix()) {
		return nil, errors.New("expiry must be in the future")
	}

	claim := &IdentityClaim{
		Address:   address,
		Tier:      tier,
		RiskScore: riskScore,
		Expiry:    expiry,
	}

	return claim, nil
}

// SerializeClaim serializes a claim for signing
// Uses the same deterministic format as the on-chain contract
func SerializeClaim(claim *IdentityClaim) ([]byte, error) {
	// Estimate buffer size
	// Address (variable) + Tier (4) + RiskScore (4) + Expiry (8) + Issuer (variable)
	buf := make([]byte, 0, 256)

	// Serialize address (as bytes)
	addressBytes := []byte(claim.Address)
	buf = append(buf, addressBytes...)

	// Serialize tier (4 bytes, big-endian)
	tierBytes := make([]byte, 4)
	binary.BigEndian.PutUint32(tierBytes, uint32(claim.Tier))
	buf = append(buf, tierBytes...)

	// Serialize risk score (4 bytes, big-endian)
	riskBytes := make([]byte, 4)
	binary.BigEndian.PutUint32(riskBytes, claim.RiskScore)
	buf = append(buf, riskBytes...)

	// Serialize expiry (8 bytes, big-endian)
	expiryBytes := make([]byte, 8)
	binary.BigEndian.PutUint64(expiryBytes, claim.Expiry)
	buf = append(buf, expiryBytes...)

	// Serialize issuer (as bytes)
	issuerBytes := []byte(claim.Issuer)
	buf = append(buf, issuerBytes...)

	return buf, nil
}

// SignClaim signs a claim with the issuer's private key
func SignClaim(claim *IdentityClaim, privateKey ed25519.PrivateKey) ([]byte, error) {
	// Serialize the claim
	message, err := SerializeClaim(claim)
	if err != nil {
		return nil, err
	}

	// Sign the message
	signature := ed25519.Sign(privateKey, message)

	return signature, nil
}

// VerifyClaim verifies a claim signature using the issuer's public key
func VerifyClaim(claim *IdentityClaim, signature []byte, publicKey ed25519.PublicKey) error {
	// Serialize the claim
	message, err := SerializeClaim(claim)
	if err != nil {
		return err
	}

	// Verify the signature
	if !ed25519.Verify(publicKey, message, signature) {
		return errors.New("invalid signature")
	}

	return nil
}

// IsExpired checks if a claim has expired
func (c *IdentityClaim) IsExpired() bool {
	now := uint64(time.Now().Unix())
	return now >= c.Expiry
}

// Validate checks if a claim is valid
func (c *IdentityClaim) Validate() error {
	if c.RiskScore > 100 {
		return errors.New("risk score must be between 0 and 100")
	}

	if c.IsExpired() {
		return errors.New("claim has expired")
	}

	if c.Address == "" {
		return errors.New("address cannot be empty")
	}

	if c.Issuer == "" {
		return errors.New("issuer cannot be empty")
	}

	return nil
}
