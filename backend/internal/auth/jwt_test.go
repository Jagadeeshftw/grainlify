package auth

import (
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
)

func TestIssueJWT_EmptySecret(t *testing.T) {
	_, err := IssueJWT("", uuid.New(), "admin", "", "", 15*time.Minute)
	if err == nil {
		t.Error("expected error for empty secret, got nil")
	}
}

func TestParseJWT_EmptySecret(t *testing.T) {
	_, err := ParseJWT("", "some.token.string")
	if err == nil {
		t.Error("expected error for empty secret, got nil")
	}
}

func TestParseJWT_ExpiredToken(t *testing.T) {
	secret := "test-secret-32-bytes-long-for-hs256!!!"
	userID := uuid.New()

	// Manually create expired JWT (bypass IssueJWT which defaults negative TTL to 15min)
	claims := &Claims{
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   userID.String(),
			IssuedAt:  jwt.NewNumericDate(time.Now().Add(-2 * time.Hour)),
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(-1 * time.Hour)),
		},
		Role: "admin",
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	tokenStr, err := token.SignedString([]byte(secret))
	if err != nil {
		t.Fatalf("failed to sign expired JWT: %v", err)
	}

	_, err = ParseJWT(secret, tokenStr)
	if err == nil {
		t.Error("expected error for expired token, got nil")
	}
}

func TestParseJWT_NotYetValidToken(t *testing.T) {
	secret := "test-secret-32-bytes-long-for-hs256!!!"
	userID := uuid.New()

	claims := &Claims{
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   userID.String(),
			IssuedAt:  jwt.NewNumericDate(time.Now().Add(1 * time.Hour)),
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(2 * time.Hour)),
			NotBefore: jwt.NewNumericDate(time.Now().Add(30 * time.Minute)),
		},
		Role: "admin",
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	tokenStr, err := token.SignedString([]byte(secret))
	if err != nil {
		t.Fatalf("failed to sign future JWT: %v", err)
	}

	_, err = ParseJWT(secret, tokenStr)
	if err == nil {
		t.Error("expected error for not-yet-valid token (nbf in future), got nil")
	}
}

func TestParseJWT_WrongSigningMethod(t *testing.T) {
	secret := "test-secret-32-bytes-long-for-hs256!!!"
	userID := uuid.New()

	claims := &Claims{
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   userID.String(),
			IssuedAt:  jwt.NewNumericDate(time.Now()),
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(15 * time.Minute)),
		},
		Role: "admin",
	}

	// Create token with HS256 but modify header alg to ES256
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	token.Header["alg"] = "ES256"
	tokenStr, err := token.SignedString([]byte(secret))
	if err != nil {
		t.Fatalf("failed to sign token with spoofed header: %v", err)
	}

	_, err = ParseJWT(secret, tokenStr)
	if err == nil {
		t.Error("expected error for wrong signing method, got nil")
	}
}

func TestParseJWT_MalformedToken(t *testing.T) {
	secret := "test-secret-32-bytes-long-for-hs256!!!"
	_, err := ParseJWT(secret, "not-a-valid-jwt")
	if err == nil {
		t.Error("expected error for malformed token, got nil")
	}
}

func TestParseJWT_ValidToken(t *testing.T) {
	secret := "test-secret-32-bytes-long-for-hs256!!!"
	userID := uuid.New()
	token, err := IssueJWT(secret, userID, "maintainer", "evm", "0x1234", 15*time.Minute)
	if err != nil {
		t.Fatalf("failed to issue valid JWT: %v", err)
	}

	claims, err := ParseJWT(secret, token)
	if err != nil {
		t.Fatalf("unexpected error parsing valid JWT: %v", err)
	}

	if claims.Subject != userID.String() {
		t.Errorf("subject = %s; want %s", claims.Subject, userID.String())
	}
	if claims.Role != "maintainer" {
		t.Errorf("role = %s; want %s", claims.Role, "maintainer")
	}
	if claims.WalletType != "evm" {
		t.Errorf("wallet_type = %s; want %s", claims.WalletType, "evm")
	}
	if claims.Address != "0x1234" {
		t.Errorf("address = %s; want %s", claims.Address, "0x1234")
	}
}

func TestIssueJWT_NegativeTTLUsesDefault(t *testing.T) {
	secret := "test-secret-32-bytes-long-for-hs256!!!"
	userID := uuid.New()
	token, err := IssueJWT(secret, userID, "admin", "", "", -5*time.Minute)
	if err != nil {
		t.Fatalf("failed to issue JWT with negative TTL: %v", err)
	}

	claims, err := ParseJWT(secret, token)
	if err != nil {
		t.Fatalf("unexpected error parsing JWT: %v", err)
	}

	// Token should still be valid (TTL defaulted to 15 min)
	if claims.Subject != userID.String() {
		t.Errorf("subject = %s; want %s", claims.Subject, userID.String())
	}
}

func TestIssueJWT_ZeroTTLUsesDefault(t *testing.T) {
	secret := "test-secret-32-bytes-long-for-hs256!!!"
	userID := uuid.New()
	token, err := IssueJWT(secret, userID, "admin", "", "", 0)
	if err != nil {
		t.Fatalf("failed to issue JWT with zero TTL: %v", err)
	}

	claims, err := ParseJWT(secret, token)
	if err != nil {
		t.Fatalf("unexpected error parsing JWT: %v", err)
	}

	if claims.Subject != userID.String() {
		t.Errorf("subject = %s; want %s", claims.Subject, userID.String())
	}
}
