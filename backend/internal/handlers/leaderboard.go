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

			if err := rows.Scan(&username, &avatarURL, &userID, &contributionCount, &ecosystems); err != nil {
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

// ProjectsLeaderboard returns top projects ranked by contributor count in verified projects
func (h *LeaderboardHandler) ProjectsLeaderboard() fiber.Handler {
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

		// Get ecosystem filter (optional)
		ecosystemSlug := c.Query("ecosystem", "")

		// Build query with optional ecosystem filter
		query := `
SELECT 
  p.id,
  p.github_full_name,
  (
    SELECT COUNT(DISTINCT a.author_login)
    FROM (
      SELECT author_login FROM github_issues WHERE project_id = p.id AND author_login IS NOT NULL AND author_login != ''
      UNION
      SELECT author_login FROM github_pull_requests WHERE project_id = p.id AND author_login IS NOT NULL AND author_login != ''
    ) a
  ) AS contributors_count,
  COALESCE(
    (
      SELECT ARRAY_AGG(DISTINCT e.name)
      FROM ecosystems e
      WHERE e.id = p.ecosystem_id AND e.status = 'active'
    ),
    ARRAY[]::TEXT[]
  ) as ecosystems,
  COALESCE(e.slug, '') as ecosystem_slug
FROM projects p
LEFT JOIN ecosystems e ON p.ecosystem_id = e.id
WHERE p.status = 'verified' 
  AND p.deleted_at IS NULL
  AND (
    SELECT COUNT(DISTINCT a.author_login)
    FROM (
      SELECT author_login FROM github_issues WHERE project_id = p.id AND author_login IS NOT NULL AND author_login != ''
      UNION
      SELECT author_login FROM github_pull_requests WHERE project_id = p.id AND author_login IS NOT NULL AND author_login != ''
    ) a
  ) > 0
`
		args := []interface{}{}
		argIndex := 1

		// Add ecosystem filter if provided
		if ecosystemSlug != "" {
			query += fmt.Sprintf(" AND LOWER(e.slug) = LOWER($%d)", argIndex)
			args = append(args, ecosystemSlug)
			argIndex++
		}

		query += `
ORDER BY contributors_count DESC, p.github_full_name ASC
`

		// Add limit and offset
		query += fmt.Sprintf(" LIMIT $%d OFFSET $%d", argIndex, argIndex+1)
		args = append(args, limit, offset)

		rows, err := h.db.Pool.Query(c.Context(), query, args...)
		if err != nil {
			slog.Error("failed to fetch project leaderboard",
				"error", err,
			)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "project_leaderboard_fetch_failed"})
		}
		defer rows.Close()

		var leaderboard []fiber.Map
		rank := offset + 1 // Start rank from offset + 1 for pagination
		for rows.Next() {
			var id string
			var fullName string
			var contributorsCount int
			var ecosystems []string
			var ecosystemSlug string

			if err := rows.Scan(&id, &fullName, &contributorsCount, &ecosystems, &ecosystemSlug); err != nil {
				slog.Error("failed to scan project leaderboard row",
					"error", err,
				)
				continue
			}

			// Ensure ecosystems is not nil
			if ecosystems == nil {
				ecosystems = []string{}
			}

			// Extract project name from github_full_name (owner/repo -> repo)
			projectName := fullName
			if idx := len(fullName) - 1; idx >= 0 {
				if slashIdx := len(fullName) - 1; slashIdx >= 0 {
					for i := len(fullName) - 1; i >= 0; i-- {
						if fullName[i] == '/' {
							projectName = fullName[i+1:]
							break
						}
					}
				}
			}

			// Generate a simple logo/icon based on project name (first letter or emoji)
			// In a real implementation, you might want to fetch the actual repo avatar from GitHub
			logo := "📦" // Default icon
			if len(projectName) > 0 {
				firstChar := projectName[0]
				// Use emoji based on first letter (simple mapping)
				emojiMap := map[byte]string{
					'a': "🅰", 'b': "🅱", 'c': "©", 'd': "♦", 'e': "⚡",
					'f': "⚡", 'g': "🎮", 'h': "🏠", 'i': "ℹ", 'j': "🎯",
					'k': "🔑", 'l': "🔗", 'm': "📱", 'n': "🔢", 'o': "⭕",
					'p': "📦", 'q': "❓", 'r': "🔴", 's': "⭐", 't': "🔧",
					'u': "⬆", 'v': "✅", 'w': "🌐", 'x': "❌", 'y': "⚛",
					'z': "⚡",
				}
				lowerChar := firstChar
				if lowerChar >= 'A' && lowerChar <= 'Z' {
					lowerChar = lowerChar + ('a' - 'A')
				}
				if emoji, ok := emojiMap[lowerChar]; ok {
					logo = emoji
				}
			}

			// Calculate activity level based on contributor count
			activity := "Low"
			if contributorsCount >= 200 {
				activity = "Very High"
			} else if contributorsCount >= 150 {
				activity = "High"
			} else if contributorsCount >= 100 {
				activity = "Medium"
			}

			// Score is based on contributor count (can be enhanced with other metrics)
			score := contributorsCount * 10 // Multiply by 10 to get a more meaningful score

			leaderboard = append(leaderboard, fiber.Map{
				"rank":        rank,
				"name":        projectName,
				"full_name":   fullName,
				"logo":        logo,
				"score":       score,
				"trend":       "same", // For now, set to 'same' (can be enhanced with historical data)
				"trendValue":  0,
				"contributors": contributorsCount,
				"ecosystems":   ecosystems,
				"activity":    activity,
				"project_id":  id,
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