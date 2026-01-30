package handlers

import (
	"fmt"
	"log/slog"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	"github.com/jagadeesh/grainlify/backend/internal/auth"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/github"
)

type UserProfileHandler struct {
	cfg config.Config
	db  *db.DB
}

func NewUserProfileHandler(cfg config.Config, d *db.DB) *UserProfileHandler {
	return &UserProfileHandler{cfg: cfg, db: d}
}

// Profile returns the user's profile statistics including:
// - Total contribution count (only for verified projects in our system)
// - Most active languages (based on contributions)
// - Most active ecosystems (based on contributions)
func (h *UserProfileHandler) Profile() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		// Get user ID from JWT
		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		// Get user's GitHub login from github_accounts
		var githubLogin *string
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubLogin)

		// Get user profile fields (bio, website, social links) from users table
		var bio, website, telegram, linkedin, whatsapp, twitter, discord *string
		_ = h.db.Pool.QueryRow(c.Context(), `
SELECT bio, website, telegram, linkedin, whatsapp, twitter, discord
FROM users
WHERE id = $1
`, userID).Scan(&bio, &website, &telegram, &linkedin, &whatsapp, &twitter, &discord)
		if err != nil {
			// User doesn't have GitHub account linked
			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"contributions_count": 0,
				"languages":           []fiber.Map{},
				"ecosystems":          []fiber.Map{},
			})
		}

		if githubLogin == nil || *githubLogin == "" {
			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"contributions_count": 0,
				"languages":           []fiber.Map{},
				"ecosystems":          []fiber.Map{},
			})
		}

		// Count total contributions (issues + PRs) for verified projects only
		var contributionsCount int
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT 
  (SELECT COUNT(*) FROM github_issues i
   INNER JOIN projects p ON i.project_id = p.id
   WHERE i.author_login = $1 AND p.status = 'verified')
  +
  (SELECT COUNT(*) FROM github_pull_requests pr
   INNER JOIN projects p ON pr.project_id = p.id
   WHERE pr.author_login = $1 AND p.status = 'verified')
`, *githubLogin).Scan(&contributionsCount)
		if err != nil {
			slog.Error("failed to count contributions", "error", err, "user_id", userID, "github_login", *githubLogin)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "contribution_count_failed"})
		}

		// Get most active languages (top 10)
		// Count contributions per language, only for verified projects
		langRows, err := h.db.Pool.Query(c.Context(), `
SELECT 
  p.language,
  COUNT(*) as contribution_count
FROM (
  SELECT project_id FROM github_issues WHERE author_login = $1
  UNION ALL
  SELECT project_id FROM github_pull_requests WHERE author_login = $1
) contributions
INNER JOIN projects p ON contributions.project_id = p.id
WHERE p.status = 'verified' AND p.language IS NOT NULL
GROUP BY p.language
ORDER BY contribution_count DESC, p.language ASC
LIMIT 10
`, *githubLogin)
		if err != nil {
			slog.Error("failed to fetch languages", "error", err, "user_id", userID, "github_login", *githubLogin)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "languages_fetch_failed"})
		}
		defer langRows.Close()

		var languages []fiber.Map
		for langRows.Next() {
			var lang string
			var count int
			if err := langRows.Scan(&lang, &count); err != nil {
				slog.Error("failed to scan language row", "error", err)
				continue
			}
			languages = append(languages, fiber.Map{
				"language":           lang,
				"contribution_count": count,
			})
		}

		// Get most active ecosystems (top 10)
		// Count contributions per ecosystem, only for verified projects
		ecoRows, err := h.db.Pool.Query(c.Context(), `
SELECT 
  e.name as ecosystem_name,
  COUNT(*) as contribution_count
FROM (
  SELECT project_id FROM github_issues WHERE author_login = $1
  UNION ALL
  SELECT project_id FROM github_pull_requests WHERE author_login = $1
) contributions
INNER JOIN projects p ON contributions.project_id = p.id
INNER JOIN ecosystems e ON p.ecosystem_id = e.id
WHERE p.status = 'verified' AND e.status = 'active'
GROUP BY e.id, e.name
ORDER BY contribution_count DESC, e.name ASC
LIMIT 10
`, *githubLogin)
		if err != nil {
			slog.Error("failed to fetch ecosystems", "error", err, "user_id", userID, "github_login", *githubLogin)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "ecosystems_fetch_failed"})
		}
		defer ecoRows.Close()

		var ecosystems []fiber.Map
		for ecoRows.Next() {
			var ecoName string
			var count int
			if err := ecoRows.Scan(&ecoName, &count); err != nil {
				slog.Error("failed to scan ecosystem row", "error", err)
				continue
			}
			ecosystems = append(ecosystems, fiber.Map{
				"ecosystem_name":     ecoName,
				"contribution_count": count,
			})
		}

		// Get user's rank position in leaderboard
		// Use a more efficient query with CTE
		var rankPosition *int
		err = h.db.Pool.QueryRow(c.Context(), `
WITH contribution_counts AS (
  SELECT 
    ga.login,
    (
      SELECT COUNT(*) 
      FROM github_issues i
      INNER JOIN projects p ON i.project_id = p.id
      WHERE i.author_login = ga.login AND p.status = 'verified'
    ) +
    (
      SELECT COUNT(*) 
      FROM github_pull_requests pr
      INNER JOIN projects p ON pr.project_id = p.id
      WHERE pr.author_login = ga.login AND p.status = 'verified'
    ) as contribution_count
  FROM github_accounts ga
  INNER JOIN users u ON ga.user_id = u.id
  WHERE (
    SELECT COUNT(*) 
    FROM github_issues i
    INNER JOIN projects p ON i.project_id = p.id
    WHERE i.author_login = ga.login AND p.status = 'verified'
  ) +
  (
    SELECT COUNT(*) 
    FROM github_pull_requests pr
    INNER JOIN projects p ON pr.project_id = p.id
    WHERE pr.author_login = ga.login AND p.status = 'verified'
  ) > 0
),
ranked_users AS (
  SELECT 
    login,
    ROW_NUMBER() OVER (
      ORDER BY contribution_count DESC, login ASC
    ) as rank_position
  FROM contribution_counts
)
SELECT rank_position
FROM ranked_users
WHERE login = $1
`, *githubLogin).Scan(&rankPosition)

		// Calculate rank tier
		var rankTier RankTier
		var rankTierName string
		var rankTierColor string
		if rankPosition != nil && *rankPosition > 0 {
			rankTier = GetRankTier(*rankPosition)
			rankTierName = GetRankTierDisplayName(rankTier)
			rankTierColor = GetRankTierColor(rankTier)
		} else {
			// User has no contributions or not ranked
			rankTier = RankBronze
			rankTierName = GetRankTierDisplayName(rankTier)
			rankTierColor = GetRankTierColor(rankTier)
		}

		// Count distinct projects user has contributed to (via issues or PRs)
		var projectsContributedToCount int
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT COUNT(DISTINCT project_id)
FROM (
  SELECT project_id FROM github_issues WHERE author_login = $1
  UNION
  SELECT project_id FROM github_pull_requests WHERE author_login = $1
) contributions
INNER JOIN projects p ON contributions.project_id = p.id
WHERE p.status = 'verified'
`, *githubLogin).Scan(&projectsContributedToCount)
		if err != nil {
			slog.Warn("failed to count projects contributed to", "error", err, "user_id", userID, "github_login", *githubLogin)
			projectsContributedToCount = 0
		}

		// Count projects where user is a maintainer/lead
		// This checks if the user is the owner of the project (via github_full_name owner match)
		var projectsLedCount int
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT COUNT(DISTINCT p.id)
FROM projects p
WHERE p.status = 'verified' 
  AND p.deleted_at IS NULL
  AND SPLIT_PART(p.github_full_name, '/', 1) = $1
`, *githubLogin).Scan(&projectsLedCount)
		if err != nil {
			slog.Warn("failed to count projects led", "error", err, "user_id", userID, "github_login", *githubLogin)
			projectsLedCount = 0
		}

		response := fiber.Map{
			"contributions_count":           contributionsCount,
			"projects_contributed_to_count": projectsContributedToCount,
			"projects_led_count":            projectsLedCount,
			"rewards_count":                 0, // TODO: Implement rewards system
			"languages":                     languages,
			"ecosystems":                    ecosystems,
			"rank": fiber.Map{
				"position":   rankPosition,
				"tier":       string(rankTier),
				"tier_name":  rankTierName,
				"tier_color": rankTierColor,
			},
		}

		// Add bio, website, and social links if available
		if bio != nil && *bio != "" {
			response["bio"] = *bio
		}
		if website != nil && *website != "" {
			response["website"] = *website
		}
		if telegram != nil && *telegram != "" {
			response["telegram"] = *telegram
		}
		if linkedin != nil && *linkedin != "" {
			response["linkedin"] = *linkedin
		}
		if whatsapp != nil && *whatsapp != "" {
			response["whatsapp"] = *whatsapp
		}
		if twitter != nil && *twitter != "" {
			response["twitter"] = *twitter
		}
		if discord != nil && *discord != "" {
			response["discord"] = *discord
		}

		return c.Status(fiber.StatusOK).JSON(response)
	}
}

// ContributionCalendar returns daily contribution counts for the last year (365 days)
// Used for rendering a GitHub-style contribution heatmap/calendar
// Returns data in format: {"date": "2024-01-15", "count": 5, "level": 3}
// where level is 0-4 (0 = no contributions, 4 = highest activity)
// Accepts optional user_id or login query parameters for viewing other users' profiles
func (h *UserProfileHandler) ContributionCalendar() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		var githubLogin *string
		var err error

		// Check if user_id or login is provided in query params (for viewing other users)
		userIDParam := c.Query("user_id")
		loginParam := c.Query("login")

		if userIDParam != "" {
			// Fetch by user_id
			parsedUserID, err := uuid.Parse(userIDParam)
			if err != nil {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_user_id"})
			}
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, parsedUserID).Scan(&githubLogin)
		} else if loginParam != "" {
			// Fetch by login
			githubLogin = &loginParam
		} else {
			// Get user ID from JWT (own profile)
			sub, _ := c.Locals(auth.LocalUserID).(string)
			userID, err := uuid.Parse(sub)
			if err != nil {
				return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
			}
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubLogin)
		}

		if githubLogin == nil || *githubLogin == "" {
			// Return empty calendar if no GitHub account
			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"calendar": []fiber.Map{},
				"total":    0,
			})
		}

		// Calculate date range: last 365 days from today
		now := time.Now().UTC()
		startDate := now.AddDate(0, 0, -365)

		// Query daily contribution counts (issues + PRs) for verified projects
		// Use DATE_TRUNC to group by day
		rows, err := h.db.Pool.Query(c.Context(), `
SELECT 
  DATE(contribution_date) as date,
  COUNT(*) as count
FROM (
  SELECT created_at_github as contribution_date
  FROM github_issues i
  INNER JOIN projects p ON i.project_id = p.id
  WHERE i.author_login = $1 
    AND i.created_at_github >= $2 
    AND i.created_at_github <= $3
    AND p.status = 'verified'
  
  UNION ALL
  
  SELECT created_at_github as contribution_date
  FROM github_pull_requests pr
  INNER JOIN projects p ON pr.project_id = p.id
  WHERE pr.author_login = $1 
    AND pr.created_at_github >= $2 
    AND pr.created_at_github <= $3
    AND p.status = 'verified'
) contributions
GROUP BY DATE(contribution_date)
ORDER BY date ASC
`, *githubLogin, startDate, now)
		if err != nil {
			slog.Error("failed to fetch contribution calendar", "error", err, "github_login", *githubLogin)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "calendar_fetch_failed"})
		}
		defer rows.Close()

		// Build a map of date -> count for quick lookup
		dateCounts := make(map[string]int)
		totalContributions := 0
		for rows.Next() {
			var date time.Time
			var count int
			if err := rows.Scan(&date, &count); err != nil {
				slog.Error("failed to scan calendar row", "error", err)
				continue
			}
			dateStr := date.Format("2006-01-02")
			dateCounts[dateStr] = count
			totalContributions += count
		}

		// Find max count for color level calculation
		maxCount := 0
		for _, count := range dateCounts {
			if count > maxCount {
				maxCount = count
			}
		}

		// Generate calendar data for all 365 days
		// Color levels: 0 = none, 1 = low, 2 = medium, 3 = high, 4 = very high
		// Using GitHub's algorithm: levels are based on quartiles
		var calendar []fiber.Map
		currentDate := startDate
		for currentDate.Before(now) || currentDate.Equal(now.Truncate(24*time.Hour)) {
			dateStr := currentDate.Format("2006-01-02")
			count := dateCounts[dateStr]

			// Calculate level (0-4) based on count
			level := calculateContributionLevel(count, maxCount)

			calendar = append(calendar, fiber.Map{
				"date":  dateStr,
				"count": count,
				"level": level,
			})

			currentDate = currentDate.AddDate(0, 0, 1)
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"calendar": calendar,
			"total":    totalContributions,
		})
	}
}

// ContributionActivity returns a paginated list of individual contributions (issues and PRs)
// Grouped by month, showing contribution type, project, title, and date
// Accepts optional user_id or login query parameters for viewing other users' profiles
func (h *UserProfileHandler) ContributionActivity() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		// Get pagination parameters
		limit := c.QueryInt("limit", 50)
		if limit > 100 {
			limit = 100 // Cap at 100 for performance
		}
		offset := c.QueryInt("offset", 0)

		var githubLogin *string
		var err error

		// Check if user_id or login is provided in query params (for viewing other users)
		userIDParam := c.Query("user_id")
		loginParam := c.Query("login")

		if userIDParam != "" {
			// Fetch by user_id
			parsedUserID, err := uuid.Parse(userIDParam)
			if err != nil {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_user_id"})
			}
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, parsedUserID).Scan(&githubLogin)
		} else if loginParam != "" {
			// Fetch by login
			githubLogin = &loginParam
		} else {
			// Get user ID from JWT (own profile)
			sub, _ := c.Locals(auth.LocalUserID).(string)
			userID, err := uuid.Parse(sub)
			if err != nil {
				return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
			}
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubLogin)
		}

		if githubLogin == nil || *githubLogin == "" {
			return c.Status(fiber.StatusOK).JSON(fiber.Map{
				"activities": []fiber.Map{},
				"total":      0,
				"limit":      limit,
				"offset":     offset,
			})
		}

		// Query contributions (issues and PRs) for verified projects
		// Order by date descending (most recent first)
		rows, err := h.db.Pool.Query(c.Context(), `
SELECT 
  'issue' as contribution_type,
  i.id,
  i.number,
  i.title,
  i.url,
  i.created_at_github,
  i.state,
  p.github_full_name as project_name,
  p.id as project_id
FROM github_issues i
INNER JOIN projects p ON i.project_id = p.id
WHERE i.author_login = $1 AND p.status = 'verified' AND i.created_at_github IS NOT NULL

UNION ALL

SELECT 
  'pull_request' as contribution_type,
  pr.id,
  pr.number,
  pr.title,
  pr.url,
  pr.created_at_github,
  pr.state,
  p.github_full_name as project_name,
  p.id as project_id
FROM github_pull_requests pr
INNER JOIN projects p ON pr.project_id = p.id
WHERE pr.author_login = $1 AND p.status = 'verified' AND pr.created_at_github IS NOT NULL

ORDER BY created_at_github DESC
LIMIT $2 OFFSET $3
`, *githubLogin, limit, offset)
		if err != nil {
			slog.Error("failed to fetch contribution activity", "error", err, "github_login", *githubLogin)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "activity_fetch_failed"})
		}
		defer rows.Close()

		var activities []fiber.Map
		for rows.Next() {
			var contribType string
			var id uuid.UUID
			var number int
			var title, url, state, projectName string
			var projectID uuid.UUID
			var createdAt *time.Time

			if err := rows.Scan(&contribType, &id, &number, &title, &url, &createdAt, &state, &projectName, &projectID); err != nil {
				slog.Error("failed to scan activity row", "error", err)
				continue
			}

			// Format date for display
			var dateStr string
			var monthYear string
			if createdAt != nil {
				dateStr = createdAt.Format("2006-01-02")
				monthYear = createdAt.Format("January 2006")
			}

			activities = append(activities, fiber.Map{
				"type":         contribType,
				"id":           id.String(),
				"number":       number,
				"title":        title,
				"url":          url,
				"state":        state,
				"date":         dateStr,
				"month_year":   monthYear,
				"project_name": projectName,
				"project_id":   projectID.String(),
			})
		}

		// Get total count for pagination
		var total int
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT 
  (SELECT COUNT(*) FROM github_issues i
   INNER JOIN projects p ON i.project_id = p.id
   WHERE i.author_login = $1 AND p.status = 'verified' AND i.created_at_github IS NOT NULL)
  +
  (SELECT COUNT(*) FROM github_pull_requests pr
   INNER JOIN projects p ON pr.project_id = p.id
   WHERE pr.author_login = $1 AND p.status = 'verified' AND pr.created_at_github IS NOT NULL)
`, *githubLogin).Scan(&total)
		if err != nil {
			slog.Error("failed to count total activities", "error", err)
			total = len(activities) // Fallback
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"activities": activities,
			"total":      total,
			"limit":      limit,
			"offset":     offset,
		})
	}
}

// ProjectsContributed returns projects a user has contributed to (via issues or PRs)
// Accepts optional user_id or login query parameters for viewing other users' profiles
func (h *UserProfileHandler) ProjectsContributed() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		var githubLogin *string
		var err error

		// Check if user_id or login is provided in query params (for viewing other users)
		userIDParam := c.Query("user_id")
		loginParam := c.Query("login")

		if userIDParam != "" {
			// Fetch by user_id
			parsedUserID, parseErr := uuid.Parse(userIDParam)
			if parseErr != nil {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_user_id"})
			}
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, parsedUserID).Scan(&githubLogin)
		} else if loginParam != "" {
			// Fetch by login
			githubLogin = &loginParam
			err = nil // No error when using login directly
		} else {
			// Get user ID from JWT (own profile)
			sub, _ := c.Locals(auth.LocalUserID).(string)
			userID, parseErr := uuid.Parse(sub)
			if parseErr != nil {
				return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
			}
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, userID).Scan(&githubLogin)
		}

		if err != nil || githubLogin == nil || *githubLogin == "" {
			slog.Warn("no github login found for user",
				"err", err,
				"user_id_param", userIDParam,
				"login_param", loginParam,
			)
			return c.Status(fiber.StatusOK).JSON([]fiber.Map{})
		}
		slog.Warn("ProjectsContributed: resolved github login",
			"github_login", githubLogin,
			"query_user_id", userIDParam,
			"query_login", loginParam,
			"jwt_sub", c.Locals(auth.LocalUserID),
		)
		// Get distinct projects user has contributed to (via issues or PRs) in verified projects
		rows, err := h.db.Pool.Query(c.Context(), `
SELECT DISTINCT
  p.id,
  p.github_full_name,
  p.status,
  e.name AS ecosystem_name,
  p.language,
  p.owner_user_id
FROM (
  SELECT DISTINCT project_id
  FROM github_issues i
  INNER JOIN projects p ON i.project_id = p.id
  WHERE i.author_login = $1 AND p.status = 'verified'
  
  UNION
  
  SELECT DISTINCT project_id
  FROM github_pull_requests pr
  INNER JOIN projects p ON pr.project_id = p.id
  WHERE pr.author_login = $1 AND p.status = 'verified'
) contrib_projects
INNER JOIN projects p ON contrib_projects.project_id = p.id
LEFT JOIN ecosystems e ON p.ecosystem_id = e.id
WHERE p.status = 'verified' AND p.deleted_at IS NULL
ORDER BY p.github_full_name ASC
LIMIT 10
`, *githubLogin)
		if err != nil {
			slog.Error("failed to fetch contributed projects", "error", err, "github_login", *githubLogin)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "projects_fetch_failed"})
		}
		defer rows.Close()

		// Get access token if available (for authenticated user)
		var accessToken string
		if loginParam == "" {
			// It's the authenticated user, try to get access token
			sub, _ := c.Locals(auth.LocalUserID).(string)
			if userID, parseErr := uuid.Parse(sub); parseErr == nil {
				linkedAccount, err := github.GetLinkedAccount(c.Context(), h.db.Pool, userID, h.cfg.TokenEncKeyB64)
				if err == nil {
					accessToken = linkedAccount.AccessToken
				}
			}
		} else if userIDParam != "" {
			// Try to get access token for the specified user
			if parsedUserID, parseErr := uuid.Parse(userIDParam); parseErr == nil {
				linkedAccount, err := github.GetLinkedAccount(c.Context(), h.db.Pool, parsedUserID, h.cfg.TokenEncKeyB64)
				if err == nil {
					accessToken = linkedAccount.AccessToken
				}
			}
		}

		gh := github.NewClient()
		var projects []fiber.Map
		for rows.Next() {
			var id uuid.UUID
			var fullName, status string
			var ecosystemName, language *string
			var ownerUserID *uuid.UUID

			if err := rows.Scan(&id, &fullName, &status, &ecosystemName, &language, &ownerUserID); err != nil {
				slog.Error("failed to scan project row", "error", err)
				continue
			}

			slog.Warn("ProjectsContributed: no github login resolved",
				"err", err,
				"githubLogin", githubLogin,
				"query_user_id", userIDParam,
				"query_login", loginParam,
				"jwt_sub", c.Locals(auth.LocalUserID),
			)

			// Fetch owner avatar from GitHub (works for public repos even without token)
			var ownerAvatarURL *string
			repo, err := gh.GetRepo(c.Context(), accessToken, fullName)
			if err == nil && !repo.Private {
				ownerAvatarURL = &repo.Owner.AvatarURL
			}

			projects = append(projects, fiber.Map{
				"id":               id.String(),
				"github_full_name": fullName,
				"status":           status,
				"ecosystem_name":   ecosystemName,
				"language":         language,
				"owner_avatar_url": ownerAvatarURL,
			})
		}

		return c.Status(fiber.StatusOK).JSON(projects)
	}
}

// PublicProfile returns public profile data for a user by user_id or GitHub login
// This endpoint is public and doesn't require authentication
func (h *UserProfileHandler) PublicProfile() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		// Get identifier from query params (user_id or login)
		userIDParam := c.Query("user_id")
		loginParam := c.Query("login")

		if userIDParam == "" && loginParam == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "missing_identifier"})
		}

		var githubLogin *string
		var userID *uuid.UUID
		var bio, website, telegram, linkedin, whatsapp, twitter, discord *string

		// If user_id is provided, get GitHub login from it
		if userIDParam != "" {
			parsedUserID, err := uuid.Parse(userIDParam)
			if err != nil {
				return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_user_id"})
			}
			userID = &parsedUserID

			err = h.db.Pool.QueryRow(c.Context(), `
SELECT login
FROM github_accounts
WHERE user_id = $1
`, parsedUserID).Scan(&githubLogin)
			if err != nil {
				// User doesn't have GitHub account linked
				return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "user_not_found"})
			}

			// Get profile fields
			_ = h.db.Pool.QueryRow(c.Context(), `
SELECT bio, website, telegram, linkedin, whatsapp, twitter, discord
FROM users
WHERE id = $1
`, parsedUserID).Scan(&bio, &website, &telegram, &linkedin, &whatsapp, &twitter, &discord)
		} else {
			// If login is provided, get user_id from it
			loginParamLower := strings.ToLower(loginParam)
			var foundUserID uuid.UUID
			err := h.db.Pool.QueryRow(c.Context(), `
SELECT ga.user_id
FROM github_accounts ga
WHERE LOWER(ga.login) = $1
`, loginParamLower).Scan(&foundUserID)
			if err != nil {
				// User not found in database, but they might still be a contributor
				// Return basic profile with just the login
				return c.Status(fiber.StatusOK).JSON(fiber.Map{
					"login":               loginParam,
					"user_id":             "",
					"contributions_count": 0,
					"languages":           []fiber.Map{},
					"ecosystems":          []fiber.Map{},
					"bio":                 nil,
					"website":             nil,
					"rank": fiber.Map{
						"position":   nil,
						"tier":       "unranked",
						"tier_name":  "Unranked",
						"tier_color": "#7a6b5a",
					},
				})
			}
			userID = &foundUserID
			githubLogin = &loginParam

			// Get profile fields
			_ = h.db.Pool.QueryRow(c.Context(), `
SELECT bio, website, telegram, linkedin, whatsapp, twitter, discord
FROM users
WHERE id = $1
`, foundUserID).Scan(&bio, &website, &telegram, &linkedin, &whatsapp, &twitter, &discord)
		}

		if githubLogin == nil || *githubLogin == "" {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "user_not_found"})
		}

		// Count total contributions (issues + PRs) for verified projects only
		var contributionsCount int
		err := h.db.Pool.QueryRow(c.Context(), `
SELECT 
  (SELECT COUNT(*) FROM github_issues i
   INNER JOIN projects p ON i.project_id = p.id
   WHERE i.author_login = $1 AND p.status = 'verified')
  +
  (SELECT COUNT(*) FROM github_pull_requests pr
   INNER JOIN projects p ON pr.project_id = p.id
   WHERE pr.author_login = $1 AND p.status = 'verified')
`, *githubLogin).Scan(&contributionsCount)
		if err != nil {
			slog.Error("failed to count contributions", "error", err, "github_login", *githubLogin)
			contributionsCount = 0
		}

		// Get most active languages (top 10)
		langRows, err := h.db.Pool.Query(c.Context(), `
SELECT 
  p.language,
  COUNT(*) as contribution_count
FROM (
  SELECT project_id, language FROM github_issues i
  INNER JOIN projects p ON i.project_id = p.id
  WHERE i.author_login = $1 AND p.status = 'verified' AND p.language IS NOT NULL
  
  UNION ALL
  
  SELECT project_id, language FROM github_pull_requests pr
  INNER JOIN projects p ON pr.project_id = p.id
  WHERE pr.author_login = $1 AND p.status = 'verified' AND p.language IS NOT NULL
) contribs
INNER JOIN projects p ON contribs.project_id = p.id
WHERE p.language IS NOT NULL
GROUP BY p.language
ORDER BY contribution_count DESC
LIMIT 10
`, *githubLogin)
		if err != nil {
			slog.Error("failed to fetch languages", "error", err, "github_login", *githubLogin)
		}
		defer langRows.Close()

		var languages []fiber.Map
		for langRows.Next() {
			var lang string
			var count int
			if err := langRows.Scan(&lang, &count); err != nil {
				continue
			}
			languages = append(languages, fiber.Map{
				"language":           lang,
				"contribution_count": count,
			})
		}

		// Get most active ecosystems (top 10)
		ecoRows, err := h.db.Pool.Query(c.Context(), `
SELECT 
  e.name as ecosystem_name,
  COUNT(*) as contribution_count
FROM (
  SELECT DISTINCT p.ecosystem_id
  FROM github_issues i
  INNER JOIN projects p ON i.project_id = p.id
  WHERE i.author_login = $1 AND p.status = 'verified' AND p.ecosystem_id IS NOT NULL
  
  UNION
  
  SELECT DISTINCT p.ecosystem_id
  FROM github_pull_requests pr
  INNER JOIN projects p ON pr.project_id = p.id
  WHERE pr.author_login = $1 AND p.status = 'verified' AND p.ecosystem_id IS NOT NULL
) contrib_ecosystems
INNER JOIN ecosystems e ON contrib_ecosystems.ecosystem_id = e.id
WHERE e.status = 'active'
GROUP BY e.name
ORDER BY contribution_count DESC
LIMIT 10
`, *githubLogin)
		if err != nil {
			slog.Error("failed to fetch ecosystems", "error", err, "github_login", *githubLogin)
		}
		defer ecoRows.Close()

		var ecosystems []fiber.Map
		for ecoRows.Next() {
			var ecoName string
			var count int
			if err := ecoRows.Scan(&ecoName, &count); err != nil {
				continue
			}
			ecosystems = append(ecosystems, fiber.Map{
				"ecosystem_name":     ecoName,
				"contribution_count": count,
			})
		}

		// Calculate rank position
		var rankPosition *int
		err = h.db.Pool.QueryRow(c.Context(), `
WITH ranked_contributors AS (
  SELECT 
    ac.login,
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
    ) as contribution_count
  FROM (
    SELECT DISTINCT i.author_login as login
    FROM github_issues i
    INNER JOIN projects p ON i.project_id = p.id
    WHERE i.author_login IS NOT NULL AND i.author_login != '' AND p.status = 'verified'
    UNION
    SELECT DISTINCT pr.author_login as login
    FROM github_pull_requests pr
    INNER JOIN projects p ON pr.project_id = p.id
    WHERE pr.author_login IS NOT NULL AND pr.author_login != '' AND p.status = 'verified'
  ) ac
)
SELECT 
  ROW_NUMBER() OVER (ORDER BY contribution_count DESC, login ASC) as rank_position
FROM ranked_contributors
WHERE LOWER(login) = LOWER($1)
`, *githubLogin).Scan(&rankPosition)
		if err != nil {
			// User not in ranking, that's okay
			rankPosition = nil
		}

		// Calculate rank tier
		rankTier := RankTierUnranked
		rankTierName := "Unranked"
		rankTierColor := "#7a6b5a"
		if rankPosition != nil {
			rankTier = GetRankTier(*rankPosition)
			rankTierName = GetRankTierDisplayName(rankTier)
			rankTierColor = GetRankTierColor(rankTier)
		}

		// Get projects contributed to and projects led counts
		var projectsContributedToCount int
		err = h.db.Pool.QueryRow(c.Context(), `
SELECT COUNT(DISTINCT p.id)
FROM (
  SELECT project_id FROM github_issues WHERE author_login = $1
  UNION
  SELECT project_id FROM github_pull_requests WHERE author_login = $1
) contribs
INNER JOIN projects p ON contribs.project_id = p.id
WHERE p.status = 'verified'
`, *githubLogin).Scan(&projectsContributedToCount)
		if err != nil {
			projectsContributedToCount = 0
		}

		var projectsLedCount int
		if userID != nil {
			err = h.db.Pool.QueryRow(c.Context(), `
SELECT COUNT(*)
FROM projects
WHERE owner_user_id = $1 AND status = 'verified' AND deleted_at IS NULL
`, *userID).Scan(&projectsLedCount)
			if err != nil {
				projectsLedCount = 0
			}
		}

		// Get avatar URL - try database first, then GitHub
		var avatarURL *string
		if userID != nil {
			_ = h.db.Pool.QueryRow(c.Context(), `
SELECT COALESCE(u.avatar_url, ga.avatar_url, '')
FROM users u
LEFT JOIN github_accounts ga ON u.id = ga.user_id
WHERE u.id = $1
`, *userID).Scan(&avatarURL)
		}
		// If no avatar in database, use GitHub avatar URL as fallback
		if (avatarURL == nil || *avatarURL == "") && githubLogin != nil {
			ghAvatarURL := fmt.Sprintf("https://github.com/%s.png?size=200", *githubLogin)
			avatarURL = &ghAvatarURL
		}

		response := fiber.Map{
			"login": *githubLogin,
			"user_id": func() string {
				if userID != nil {
					return userID.String()
				}
				return ""
			}(),
			"avatar_url": func() string {
				if avatarURL != nil && *avatarURL != "" {
					return *avatarURL
				}
				return ""
			}(),
			"contributions_count":           contributionsCount,
			"projects_contributed_to_count": projectsContributedToCount,
			"projects_led_count":            projectsLedCount,
			"languages":                     languages,
			"ecosystems":                    ecosystems,
			"rank": fiber.Map{
				"position":   rankPosition,
				"tier":       string(rankTier),
				"tier_name":  rankTierName,
				"tier_color": rankTierColor,
			},
		}

		if bio != nil && *bio != "" {
			response["bio"] = *bio
		}
		if website != nil && *website != "" {
			response["website"] = *website
		}
		if telegram != nil && *telegram != "" {
			response["telegram"] = *telegram
		}
		if linkedin != nil && *linkedin != "" {
			response["linkedin"] = *linkedin
		}
		if whatsapp != nil && *whatsapp != "" {
			response["whatsapp"] = *whatsapp
		}
		if twitter != nil && *twitter != "" {
			response["twitter"] = *twitter
		}
		if discord != nil && *discord != "" {
			response["discord"] = *discord
		}

		return c.Status(fiber.StatusOK).JSON(response)
	}
}

// calculateContributionLevel determines the color level (0-4) based on contribution count
// Uses GitHub's algorithm: levels are based on quartiles of the max count
func calculateContributionLevel(count int, maxCount int) int {
	if count == 0 {
		return 0
	}
	if maxCount == 0 {
		return 0
	}

	// Calculate quartiles
	q1 := maxCount / 4
	q2 := maxCount / 2
	q3 := (maxCount * 3) / 4

	if count <= q1 {
		return 1 // Low
	} else if count <= q2 {
		return 2 // Medium
	} else if count <= q3 {
		return 3 // High
	} else {
		return 4 // Very high
	}
}

// UpdateProfile updates user profile information (first_name, last_name, location, website, bio)
func (h *UserProfileHandler) UpdateProfile() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		// Get user ID from JWT
		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		var req struct {
			FirstName *string `json:"first_name,omitempty"`
			LastName  *string `json:"last_name,omitempty"`
			Location  *string `json:"location,omitempty"`
			Website   *string `json:"website,omitempty"`
			Bio       *string `json:"bio,omitempty"`
			Telegram  *string `json:"telegram,omitempty"`
			LinkedIn  *string `json:"linkedin,omitempty"`
			WhatsApp  *string `json:"whatsapp,omitempty"`
			Twitter   *string `json:"twitter,omitempty"`
			Discord   *string `json:"discord,omitempty"`
		}

		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}

		// Build update query dynamically based on provided fields
		var updates []string
		var args []interface{}
		argPos := 1

		if req.FirstName != nil {
			updates = append(updates, fmt.Sprintf("first_name = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.FirstName))
			argPos++
		}
		if req.LastName != nil {
			updates = append(updates, fmt.Sprintf("last_name = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.LastName))
			argPos++
		}
		if req.Location != nil {
			updates = append(updates, fmt.Sprintf("location = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.Location))
			argPos++
		}
		if req.Website != nil {
			updates = append(updates, fmt.Sprintf("website = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.Website))
			argPos++
		}
		if req.Bio != nil {
			updates = append(updates, fmt.Sprintf("bio = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.Bio))
			argPos++
		}
		if req.Telegram != nil {
			updates = append(updates, fmt.Sprintf("telegram = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.Telegram))
			argPos++
		}
		if req.LinkedIn != nil {
			updates = append(updates, fmt.Sprintf("linkedin = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.LinkedIn))
			argPos++
		}
		if req.WhatsApp != nil {
			updates = append(updates, fmt.Sprintf("whatsapp = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.WhatsApp))
			argPos++
		}
		if req.Twitter != nil {
			updates = append(updates, fmt.Sprintf("twitter = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.Twitter))
			argPos++
		}
		if req.Discord != nil {
			updates = append(updates, fmt.Sprintf("discord = $%d", argPos))
			args = append(args, strings.TrimSpace(*req.Discord))
			argPos++
		}

		if len(updates) == 0 {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "no_fields_to_update"})
		}

		// Always update updated_at
		updates = append(updates, "updated_at = now()")
		args = append(args, userID)

		query := fmt.Sprintf(`
UPDATE users
SET %s
WHERE id = $%d
`, strings.Join(updates, ", "), argPos)

		_, err = h.db.Pool.Exec(c.Context(), query, args...)
		if err != nil {
			slog.Error("failed to update user profile", "error", err, "user_id", userID)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "profile_update_failed"})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{"message": "profile_updated"})
	}
}

// UpdateAvatar updates user avatar URL
func (h *UserProfileHandler) UpdateAvatar() fiber.Handler {
	return func(c *fiber.Ctx) error {
		if h.db == nil || h.db.Pool == nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "db_not_configured"})
		}

		// Get user ID from JWT
		sub, _ := c.Locals(auth.LocalUserID).(string)
		userID, err := uuid.Parse(sub)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "invalid_user"})
		}

		var req struct {
			AvatarURL string `json:"avatar_url"`
		}

		if err := c.BodyParser(&req); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_json"})
		}

		avatarURL := strings.TrimSpace(req.AvatarURL)
		if avatarURL == "" {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "avatar_url_required"})
		}

		// Validate URL format (either http/https URL or data URL)
		if !strings.HasPrefix(avatarURL, "http://") &&
			!strings.HasPrefix(avatarURL, "https://") &&
			!strings.HasPrefix(avatarURL, "data:image/") {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "invalid_avatar_url_format"})
		}

		_, err = h.db.Pool.Exec(c.Context(), `
UPDATE users
SET avatar_url = $1, updated_at = now()
WHERE id = $2
`, avatarURL, userID)
		if err != nil {
			slog.Error("failed to update user avatar", "error", err, "user_id", userID)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "avatar_update_failed"})
		}

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"message":    "avatar_updated",
			"avatar_url": avatarURL,
		})
	}
}
