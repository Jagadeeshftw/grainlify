package auth

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"errors"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type User struct {
	ID   uuid.UUID `json:"id"`
	Role string    `json:"role"`
}

type Wallet struct {
	WalletType WalletType `json:"wallet_type"`
	Address    string     `json:"address"`
	PublicKey  string     `json:"public_key,omitempty"`
}

type Nonce struct {
	Nonce     string    `json:"nonce"`
	ExpiresAt time.Time `json:"expires_at"`
}

func CreateNonce(ctx context.Context, pool *pgxpool.Pool, walletType WalletType, address string, ttl time.Duration) (Nonce, error) {
	if pool == nil {
		return Nonce{}, fmt.Errorf("db not configured")
	}
	if ttl <= 0 {
		ttl = 10 * time.Minute
	}

	nonce := randomNonce(32)
	expiresAt := time.Now().UTC().Add(ttl)

	_, err := pool.Exec(ctx, `
INSERT INTO auth_nonces (wallet_type, address, nonce, expires_at)
VALUES ($1, $2, $3, $4)
`, string(walletType), address, nonce, expiresAt)
	if err != nil {
		return Nonce{}, err
	}

	return Nonce{Nonce: nonce, ExpiresAt: expiresAt}, nil
}

type VerifyResult struct {
	User   User   `json:"user"`
	Wallet Wallet `json:"wallet"`
}

func ConsumeNonceAndUpsertUser(ctx context.Context, pool *pgxpool.Pool, walletType WalletType, address string, nonce string, publicKey string) (VerifyResult, error) {
	if pool == nil {
		return VerifyResult{}, fmt.Errorf("db not configured")
	}

	tx, err := pool.BeginTx(ctx, pgx.TxOptions{})
	if err != nil {
		return VerifyResult{}, err
	}
	defer func() { _ = tx.Rollback(ctx) }()

	var nonceID uuid.UUID
	err = tx.QueryRow(ctx, `
SELECT id
FROM auth_nonces
WHERE wallet_type = $1
  AND address = $2
  AND nonce = $3
  AND used_at IS NULL
  AND expires_at > now()
FOR UPDATE
`, string(walletType), address, nonce).Scan(&nonceID)
	if errors.Is(err, pgx.ErrNoRows) {
		return VerifyResult{}, fmt.Errorf("invalid_or_expired_nonce")
	}
	if err != nil {
		return VerifyResult{}, err
	}

	if _, err := tx.Exec(ctx, `UPDATE auth_nonces SET used_at = now() WHERE id = $1`, nonceID); err != nil {
		return VerifyResult{}, err
	}

	var userID uuid.UUID
	var role string
	err = tx.QueryRow(ctx, `
SELECT u.id, u.role
FROM wallets w
JOIN users u ON u.id = w.user_id
WHERE w.wallet_type = $1 AND w.address = $2
`, string(walletType), address).Scan(&userID, &role)
	if errors.Is(err, pgx.ErrNoRows) {
		// New user + wallet.
		err = tx.QueryRow(ctx, `INSERT INTO users DEFAULT VALUES RETURNING id, role`).Scan(&userID, &role)
		if err != nil {
			return VerifyResult{}, err
		}

		_, err = tx.Exec(ctx, `
INSERT INTO wallets (user_id, wallet_type, address, public_key)
VALUES ($1, $2, $3, $4)
`, userID, string(walletType), address, nullIfEmpty(publicKey))
		if err != nil {
			return VerifyResult{}, err
		}
	} else if err != nil {
		return VerifyResult{}, err
	} else {
		// Existing wallet: update public key if provided and missing.
		if publicKey != "" {
			_, _ = tx.Exec(ctx, `
UPDATE wallets
SET public_key = COALESCE(public_key, $3)
WHERE wallet_type = $1 AND address = $2
`, string(walletType), address, publicKey)
		}
	}

	if err := tx.Commit(ctx); err != nil {
		return VerifyResult{}, err
	}

	return VerifyResult{
		User: User{ID: userID, Role: role},
		Wallet: Wallet{
			WalletType: walletType,
			Address:    address,
			PublicKey:  publicKey,
		},
	}, nil
}

func randomNonce(n int) string {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		// Should never happen, but keep it deterministic-ish if entropy fails.
		return uuid.NewString()
	}
	return base64.RawURLEncoding.EncodeToString(b)
}

func nullIfEmpty(s string) any {
	if s == "" {
		return nil
	}
	return s
}





















