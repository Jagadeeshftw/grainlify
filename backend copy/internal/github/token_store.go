package github

import (
	"context"
	"errors"
	"fmt"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"

	"github.com/jagadeesh/grainlify/backend/internal/cryptox"
)

type LinkedAccount struct {
	GitHubUserID int64
	Login        string
	AccessToken  string
}

func GetLinkedAccount(ctx context.Context, pool *pgxpool.Pool, userID uuid.UUID, tokenEncKeyB64 string) (LinkedAccount, error) {
	if pool == nil {
		return LinkedAccount{}, fmt.Errorf("db not configured")
	}

	var githubUserID int64
	var login string
	var encToken []byte
	err := pool.QueryRow(ctx, `
SELECT github_user_id, login, access_token
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubUserID, &login, &encToken)
	if errors.Is(err, pgx.ErrNoRows) {
		return LinkedAccount{}, fmt.Errorf("github_not_linked")
	}
	if err != nil {
		return LinkedAccount{}, err
	}

	key, err := cryptox.KeyFromB64(tokenEncKeyB64)
	if err != nil {
		return LinkedAccount{}, err
	}
	tokenBytes, err := cryptox.DecryptAESGCM(key, encToken)
	if err != nil {
		return LinkedAccount{}, fmt.Errorf("decrypt github token failed")
	}

	return LinkedAccount{
		GitHubUserID: githubUserID,
		Login:        login,
		AccessToken:  string(tokenBytes),
	}, nil
}





















