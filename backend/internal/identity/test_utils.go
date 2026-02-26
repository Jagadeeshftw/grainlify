package identity

import (
	"crypto/ed25519"
	"crypto/rand"
	"time"
)

// TestKeyPair represents a test key pair for signing claims
type TestKeyPair struct {
	PublicKey  ed25519.PublicKey
	PrivateKey ed25519.PrivateKey
}

// GenerateTestKeyPair generates a new Ed25519 key pair for testing
func GenerateTestKeyPair() (*TestKeyPair, error) {
	publicKey, privateKey, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	return &TestKeyPair{
		PublicKey:  publicKey,
		PrivateKey: privateKey,
	}, nil
}

// GenerateTestClaim creates a test claim with valid signature
func GenerateTestClaim(
	address string,
	tier IdentityTier,
	riskScore uint32,
	keyPair *TestKeyPair,
) (*IdentityClaim, []byte, error) {
	// Create claim with 1 hour validity
	claim, err := CreateClaim(address, tier, riskScore, time.Hour)
	if err != nil {
		return nil, nil, err
	}

	// Set issuer (in real implementation, this would be the issuer's address)
	claim.Issuer = "test-issuer"

	// Sign the claim
	signature, err := SignClaim(claim, keyPair.PrivateKey)
	if err != nil {
		return nil, nil, err
	}

	return claim, signature, nil
}

// GenerateTestClaimWithExpiry creates a test claim with custom expiry
func GenerateTestClaimWithExpiry(
	address string,
	tier IdentityTier,
	riskScore uint32,
	expiry uint64,
	keyPair *TestKeyPair,
) (*IdentityClaim, []byte, error) {
	claim := &IdentityClaim{
		Address:   address,
		Tier:      tier,
		RiskScore: riskScore,
		Expiry:    expiry,
		Issuer:    "test-issuer",
	}

	// Sign the claim
	signature, err := SignClaim(claim, keyPair.PrivateKey)
	if err != nil {
		return nil, nil, err
	}

	return claim, signature, nil
}
