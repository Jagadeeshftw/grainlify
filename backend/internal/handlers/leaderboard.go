package handlers

import (
	"fmt"
	"log/slog"

	"github.com/gofiber/fiber/v2"

	"github.com/jagadeesh/grainlify/backend/internal/db"
)

type LeaderboardHandler struct {
	db *db.DB
}

func NewLeaderboardHandler(d *db.DB) *LeaderboardHandler {
	return &LeaderboardHandler{db: d}
}

// Leaderboard returns top contributors ranked by contributions in verified projects
func (h *LeaderboardHandler) Leaderboard() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		// Get limit and offset from query params (default 10, max 100)
		limit := c.QueryInt("limit", 10)
		if limit < 1 {
			limit = 10
		}
		if limit > 100 {
			limit = 100
		}
		offset := c.QueryInt("offset", 0)
		if offset < 0 {
			offset = 0
		}

		// Query top contributors by contribution count in verified projects
		// This query:
		// 1. Gets all unique author_logins from issues and PRs in verified projects
		// 2. LEFT JOINs with github_accounts to get user info if they signed up
		// 3. Shows ALL contributors, whether they signed up or not
		// 4. Counts their contributions (issues + PRs) in verified projects
		rows, err := h.db.Pool.Query(c.Context(), `
WITH all_contributors AS (
  -- Get all unique contributors from issues in verified projects
  SELECT DISTINCT i.author_login as login
  FROM github_issues i
  INNER JOIN projects p ON i.project_id = p.id
  WHERE i.author_login IS NOT NULL 
    AND i.author_login != ''
    AND p.status = 'verified'
  
  UNION
  
  -- Get all unique contributors from PRs in verified projects
  SELECT DISTINCT pr.author_login as login
  FROM github_pull_requests pr
  INNER JOIN projects p ON pr.project_id = p.id
  WHERE pr.author_login IS NOT NULL 
    AND pr.author_login != ''
    AND p.status = 'verified'
)
SELECT 
  ac.login as username,
  COALESCE(ga.avatar_url, '') as avatar_url,
  COALESCE(u.id::text, '') as user_id,
  u.first_name,
  u.last_name,
  (
    SELECT COUNT(*) 
    FROM github_issues i
    INNER JOIN projects p ON i.project_id = p.id
    WHERE LOWER(i.author_login) = LOWER(ac.login) AND p.status = 'verified'
  ) +
  (
    SELECT COUNT(*) 
    FROM github_pull_requests pr
    INNER JOIN projects p ON pr.project_id = p.id
    WHERE LOWER(pr.author_login) = LOWER(ac.login) AND p.status = 'verified'
  ) as contribution_count,
  COALESCE(
    (
      SELECT ARRAY_AGG(DISTINCT e.name)
      FROM (
        SELECT DISTINCT p.ecosystem_id
        FROM github_issues i
        INNER JOIN projects p ON i.project_id = p.id
        WHERE LOWER(i.author_login) = LOWER(ac.login) AND p.status = 'verified'
        UNION
        SELECT DISTINCT p.ecosystem_id
        FROM github_pull_requests pr
        INNER JOIN projects p ON pr.project_id = p.id
        WHERE LOWER(pr.author_login) = LOWER(ac.login) AND p.status = 'verified'
      ) contrib_ecosystems
      INNER JOIN ecosystems e ON contrib_ecosystems.ecosystem_id = e.id
      WHERE e.status = 'active'
    ),
    ARRAY[]::TEXT[]
  ) as ecosystems
FROM all_contributors ac
LEFT JOIN github_accounts ga ON LOWER(ga.login) = LOWER(ac.login)
LEFT JOIN users u ON ga.user_id = u.id
WHERE (
  SELECT COUNT(*) 
  FROM github_issues i
  INNER JOIN projects p ON i.project_id = p.id
  WHERE LOWER(i.author_login) = LOWER(ac.login) AND p.status = 'verified'
) +
(
  SELECT COUNT(*) 
  FROM github_pull_requests pr
  INNER JOIN projects p ON pr.project_id = p.id
  WHERE LOWER(pr.author_login) = LOWER(ac.login) AND p.status = 'verified'
) > 0
ORDER BY contribution_count DESC, ac.login ASC
LIMIT $1 OFFSET $2
`, limit, offset)
		if err != nil {
			slog.Error("failed to fetch leaderboard",
				"error", err,
			)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "leaderboard_fetch_failed"})
		}
		defer rows.Close()

		var leaderboard []fiber.Map
		rank := offset + 1 // Start rank from offset + 1 for pagination
		for rows.Next() {
			var username string
			var avatarURL *string
			var userID string
			var contributionCount int
			var ecosystems []string
			var firstName, lastName *string

			if err := rows.Scan(&username, &avatarURL, &userID, &contributionCount, &ecosystems, &firstName, &lastName); err != nil {
				slog.Error("failed to scan leaderboard row",
					"error", err,
				)
				continue
			}

			// Default avatar if not set - use GitHub avatar URL as fallback
			avatar := ""
			if avatarURL != nil && *avatarURL != "" {
				avatar = *avatarURL
			} else {
				// Fallback to GitHub avatar URL if not in database
				avatar = fmt.Sprintf("https://github.com/%s.png?size=200", username)
			}

			// Ensure ecosystems is not nil
			if ecosystems == nil {
				ecosystems = []string{}
			}

			// Calculate rank tier based on position
			rankTier := GetRankTier(rank)

			leaderboard = append(leaderboard, fiber.Map{
				"rank":           rank,
				"rank_tier":      string(rankTier),
				"rank_tier_name": GetRankTierDisplayName(rankTier),
				"username":       username,
				"avatar":         avatar,
				"user_id":        userID,
				"contributions":  contributionCount,
				"ecosystems":     ecosystems,
				"first_name":     firstName,
				"last_name":      lastName,
				// For now, set trend to 'same' and score to contribution count
				// These can be enhanced later with historical data
				"score":      contributionCount,
				"trend":      "same",
				"trendValue": 0,
			})
			rank++
		}

		// Always return an array, even if empty
		if leaderboard == nil {
			leaderboard = []fiber.Map{}
		}

		return c.Status(fiber.StatusOK).JSON(leaderboard)
	}
}
