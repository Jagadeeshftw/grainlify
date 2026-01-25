# Patchwork Backend API Documentation

This document describes all available API endpoints for the Patchwork backend. Use this as a reference when integrating the frontend.


## Authentication

Most endpoints require authentication using a JWT (JSON Web Token). Include the token in the `Authorization` header:

```
Authorization: Bearer <your_jwt_token>
```

### Getting a JWT Token

1. Sign in with GitHub (see [GitHub OAuth](#github-oauth) section)
2. The JWT token is returned in the response
3. Store the token and include it in subsequent requests

---

## Table of Contents

1. [Health & Status](#health--status)
2. [Authentication](#authentication-endpoints)
3. [User Profile](#user-profile)
4. [GitHub OAuth](#github-oauth)
5. [KYC Verification](#kyc-verification)
6. [Projects](#projects)
7. [Public Projects](#public-projects)
8. [Ecosystems](#ecosystems)
9. [Admin](#admin)

---

## Health & Status

### GET /health

Check if the API server is running.

**Authentication:** None required

**Response:**
```json
{
  "ok": true,
  "service": "patchwork-api"
}
```

---

### GET /ready

Check if the API is ready (database connectivity check).

**Authentication:** None required

**Response:**
```json
{
  "ok": true,
  "db": "connected"
}
```

**Error Response (503):**
```json
{
  "ok": false,
  "db": "disconnected"
}
```

---

## Authentication Endpoints

### GET /me

Get current authenticated user information.

**Authentication:** Required (JWT)

**Response:**
```json
{
  "id": "8420cb43-eb78-4aa8-b8fb-9d3ab0e2d7c8",
  "role": "contributor"
}
```

**Error Responses:**
- `401 Unauthorized` - Invalid or missing JWT token
- `503 Service Unavailable` - Database not configured

---

## User Profile

### GET /profile

Get user profile statistics including contribution count, most active languages, and ecosystems.

**Authentication:** Required (JWT)

**Response:**
```json
{
  "contributions_count": 165,
  "languages": [
    {
      "language": "TypeScript",
      "contribution_count": 45
    },
    {
      "language": "Go",
      "contribution_count": 30
    }
  ],
  "ecosystems": [
    {
      "ecosystem_name": "Starknet",
      "contribution_count": 120
    },
    {
      "ecosystem_name": "Ethereum",
      "contribution_count": 45
    }
  ]
}
```

**Notes:**
- Only counts contributions to verified projects in our system
- Returns empty arrays if user has no GitHub account linked
- Languages and ecosystems are limited to top 10

---

### GET /profile/calendar

Get daily contribution counts for the last 365 days (for contribution heatmap/calendar visualization).

**Authentication:** Required (JWT)

**Response:**
```json
{
  "calendar": [
    {
      "date": "2024-01-15",
      "count": 5,
      "level": 3
    },
    {
      "date": "2024-01-16",
      "count": 0,
      "level": 0
    },
    {
      "date": "2024-01-17",
      "count": 12,
      "level": 4
    }
    // ... 365 days total
  ],
  "total": 470
}
```

**Field Descriptions:**
- `date`: ISO date string (YYYY-MM-DD)
- `count`: Number of contributions on that day (issues + PRs)
- `level`: Color level (0-4) for visualization
  - `0` = No contributions (lightest color)
  - `1` = Low activity
  - `2` = Medium activity
  - `3` = High activity
  - `4` = Very high activity (darkest color)

**Notes:**
- Only includes contributions to verified projects
- Returns empty calendar if user has no GitHub account
- Level calculation uses quartiles of max count (similar to GitHub)

---

### GET /profile/activity

Get paginated list of individual contributions (issues and PRs) for the authenticated user.

**Authentication:** Required (JWT)

**Query Parameters:**
- `limit` (optional, default: 50, max: 100) - Number of results per page
- `offset` (optional, default: 0) - Pagination offset

**Example Request:**
```
GET /profile/activity?limit=50&offset=0
```

**Response:**
```json
{
  "activities": [
    {
      "type": "pull_request",
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "number": 2341,
      "title": "Implement caching layer",
      "url": "https://github.com/owner/repo/pull/2341",
      "date": "2025-11-15",
      "month_year": "November 2025",
      "project_name": "owner/repo",
      "project_id": "79caaf9a-f1e6-4da0-be79-52c5bee169e1"
    },
    {
      "type": "issue",
      "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "number": 8923,
      "title": "Add unit tests for API endpoints",
      "url": "https://github.com/owner/repo/issues/8923",
      "date": "2025-11-22",
      "month_year": "November 2025",
      "project_name": "owner/repo",
      "project_id": "79caaf9a-f1e6-4da0-be79-52c5bee169e1"
    }
  ],
  "total": 165,
  "limit": 50,
  "offset": 0
}
```

**Field Descriptions:**
- `type`: Either `"issue"` or `"pull_request"`
- `id`: Contribution UUID
- `number`: GitHub issue/PR number
- `title`: Contribution title
- `url`: GitHub URL
- `date`: ISO date string (YYYY-MM-DD)
- `month_year`: Formatted month/year (e.g., "November 2025")
- `project_name`: Repository full name (owner/repo)
- `project_id`: Project UUID

**Notes:**
- Ordered by date descending (most recent first)
- Only includes contributions to verified projects
- Returns empty array if user has no GitHub account

---

## GitHub OAuth

### GET /auth/github/login/start

Start GitHub OAuth login/signup flow. Redirects user to GitHub for authorization.

**Authentication:** None required

**Response:** HTTP 302 redirect to GitHub OAuth page

**Flow:**
1. User is redirected to GitHub
2. User authorizes the application
3. GitHub redirects back to `/auth/github/callback`
4. Backend processes OAuth and redirects to frontend with JWT token

---

### GET /auth/github/callback

GitHub OAuth callback endpoint (handled automatically by backend).

**Authentication:** None required

**Note:** This endpoint is called by GitHub after user authorization. The backend will redirect to the frontend with the JWT token.

---

### POST /auth/github/start

Link GitHub account to existing authenticated user (legacy endpoint, still available).

**Authentication:** Required (JWT)

**Response:** HTTP 302 redirect to GitHub OAuth page

---

### GET /auth/github/status

Check GitHub account linking status for the authenticated user.

**Authentication:** Required (JWT)

**Response:**
```json
{
  "linked": true,
  "github": {
    "id": 92681651,
    "login": "Jagadeeshftw"
  }
}
```

**If not linked:**
```json
{
  "linked": false
}
```

---

## KYC Verification

### POST /auth/kyc/start

Start a new KYC verification session using Didit.

**Authentication:** Required (JWT)

**Request Body:** None

**Response:**
```json
{
  "session_id": "871e9803-178d-4290-a36b-bd2f09901e57",
  "url": "https://verify.didit.me/session/OcTUSqkMkW7Q"
}
```

**Error Responses:**
- `409 Conflict` - User already has an active KYC session
  ```json
  {
    "error": "kyc_session_exists",
    "message": "You already have an active KYC verification session (status: pending). Please complete it or contact admin to delete it.",
    "session_id": "871e9803-178d-4290-a36b-bd2f09901e57",
    "status": "pending",
    "url": "https://verify.didit.me/session/OcTUSqkMkW7Q"
  }
  ```
- `503 Service Unavailable` - KYC not configured (missing DIDIT_API_KEY or DIDIT_WORKFLOW_ID)

**Notes:**
- Only one active session per user is allowed
- If a previous session exists, the endpoint returns 409 with the existing session URL
- User should visit the `url` to complete verification

---

### GET /auth/kyc/status

Get current KYC verification status for the authenticated user.

**Authentication:** Required (JWT)

**Response:**
```json
{
  "status": "verified",
  "session_id": "871e9803-178d-4290-a36b-bd2f09901e57",
  "verified_at": "2025-12-31T03:07:27.273068+05:30",
  "data": {
    "decision": {},
    "data": {
      "id_verification": {
        "full_name": "Jagadeesh",
        "date_of_birth": "2002-06-26",
        "age": 23,
        "address": "1/136, School Street...",
        "document_type": "Identity Card",
        "document_number": "382195807643",
        "status": "Approved"
      },
      "face_match": {
        "score": 26.89,
        "status": "In Review"
      }
    },
    "extracted": {
      "full_name": "Jagadeesh",
      "date_of_birth": "2002-06-26",
      "age": 23,
      "address": "1/136, School Street...",
      "document_type": "Identity Card",
      "document_number": "382195807643",
      "id_verification_status": "Approved",
      "face_match_score": 26.89,
      "face_match_status": "In Review"
    },
    "session_url": "https://verify.didit.me/session/OcTUSqkMkW7Q"
  },
  "extracted": {
    "full_name": "Jagadeesh",
    "date_of_birth": "2002-06-26",
    "age": 23,
    "address": "1/136, School Street...",
    "document_type": "Identity Card",
    "document_number": "382195807643",
    "id_verification_status": "Approved",
    "face_match_score": 26.89,
    "face_match_status": "In Review"
  }
}
```

**Status Values:**
- `null` - No KYC session started
- `"not_started"` - Session created but user hasn't clicked verification link
- `"pending"` - User has started verification (clicked link, submitted documents)
- `"in_review"` - Didit is actively reviewing the verification
- `"verified"` - KYC approved
- `"rejected"` - KYC declined
- `"expired"` - Session expired or deleted

**Rejected Status Response:**
```json
{
  "status": "rejected",
  "session_id": "871e9803-178d-4290-a36b-bd2f09901e57",
  "verified_at": null,
  "rejection_reason": "The facial features of the provided image don't closely match the reference image, suggesting a potential identity mismatch.",
  "extracted": {
    "rejection_reasons": [
      "The facial features of the provided image don't closely match the reference image, suggesting a potential identity mismatch."
    ]
  }
}
```

**Notes:**
- Status is automatically synced with Didit API
- If session is deleted in Didit dashboard, status updates to `"expired"`
- `extracted` field contains structured KYC information for easy display

---

## Projects

### POST /projects

Register a new GitHub repository as a project.

**Authentication:** Required (JWT)

**Request Body:**
```json
{
  "github_full_name": "owner/repo",
  "ecosystem_name": "Starknet",
  "language": "TypeScript",
  "tags": ["good first issue", "help wanted"],
  "category": "Frontend"
}
```

**Field Descriptions:**
- `github_full_name` (required): Repository full name (owner/repo)
- `ecosystem_name` (required): Name of an active ecosystem (must exist in database)
- `language` (optional): Programming language
- `tags` (optional): Array of tag strings
- `category` (optional): Project category

**Response:**
```json
{
  "id": "79caaf9a-f1e6-4da0-be79-52c5bee169e1",
  "github_full_name": "owner/repo",
  "status": "pending_verification",
  "ecosystem_name": "Starknet",
  "language": "TypeScript",
  "tags": ["good first issue", "help wanted"],
  "category": "Frontend",
  "created_at": "2025-12-30T21:25:50.85241+05:30",
  "updated_at": "2025-12-30T21:25:50.85241+05:30"
}
```

**Error Responses:**
- `400 Bad Request` - Invalid request (missing required fields, ecosystem not found)
- `401 Unauthorized` - Invalid or missing JWT token

---

### GET /projects/mine

Get all projects owned by the authenticated user.

**Authentication:** Required (JWT)

**Response:**
```json
[
  {
    "id": "79caaf9a-f1e6-4da0-be79-52c5bee169e1",
    "github_full_name": "owner/repo",
    "github_repo_id": 1038118146,
    "status": "verified",
    "ecosystem_name": "Starknet",
    "language": "TypeScript",
    "tags": ["good first issue", "help wanted"],
    "category": "Frontend",
    "verification_error": null,
    "verified_at": "2025-12-30T22:52:00.3484+05:30",
    "webhook_created_at": "2025-12-30T21:30:18.524427+05:30",
    "webhook_id": 588988804,
    "webhook_url": "https://slfs8kjg75.loclx.io/webhooks/github",
    "created_at": "2025-12-30T21:25:50.85241+05:30",
    "updated_at": "2025-12-30T22:52:00.3484+05:30"
  }
]
```

**Status Values:**
- `"pending_verification"` - Project created but not yet verified
- `"verified"` - Project verified and webhook enabled
- `"rejected"` - Project verification failed

---

### POST /projects/:id/verify

Verify project ownership and enable GitHub webhook.

**Authentication:** Required (JWT)

**URL Parameters:**
- `id` - Project UUID

**Request Body:** None

**Response:**
```json
{
  "id": "79caaf9a-f1e6-4da0-be79-52c5bee169e1",
  "status": "verified",
  "verified_at": "2025-12-30T22:52:00.3484+05:30",
  "webhook_id": 588988804,
  "webhook_url": "https://slfs8kjg75.loclx.io/webhooks/github"
}
```

**Error Responses:**
- `400 Bad Request` - Verification failed (see `verification_error` in response)
- `404 Not Found` - Project not found
- `503 Service Unavailable` - Webhook not configured (missing PUBLIC_BASE_URL or GITHUB_WEBHOOK_SECRET)

**Notes:**
- Requires PUBLIC_BASE_URL and GITHUB_WEBHOOK_SECRET to be configured
- Verifies user has admin access to the repository
- Creates GitHub webhook for the repository

---

### POST /projects/:id/sync

Enqueue a full sync job for a project (syncs issues and PRs from GitHub).

**Authentication:** Required (JWT)

**URL Parameters:**
- `id` - Project UUID

**Request Body:** None

**Response:**
```json
{
  "ok": true,
  "message": "Sync job enqueued"
}
```

**Notes:**
- Sync runs asynchronously
- Use `/projects/:id/sync/jobs` to check sync status

---

### GET /projects/:id/sync/jobs

Get sync jobs for a project.

**Authentication:** Required (JWT)

**URL Parameters:**
- `id` - Project UUID

**Response:**
```json
[
  {
    "id": "job-uuid",
    "job_type": "sync_issues",
    "status": "completed",
    "run_at": "2025-12-30T22:56:03.058032+05:30",
    "attempts": 1,
    "last_error": null,
    "created_at": "2025-12-30T22:56:03.058032+05:30",
    "updated_at": "2025-12-30T22:56:03.058032+05:30"
  }
]
```

**Status Values:**
- `"pending"` - Job queued, not started
- `"running"` - Job in progress
- `"completed"` - Job finished successfully
- `"failed"` - Job failed (check `last_error`)

---

### GET /projects/:id/issues

Get issues for a project.

**Authentication:** Required (JWT)

**URL Parameters:**
- `id` - Project UUID

**Response:**
```json
[
  {
    "id": "issue-uuid",
    "github_issue_id": 3770820248,
    "number": 1,
    "state": "open",
    "title": "Button not responding on click",
    "description": "The submit button does not respond when clicked...",
    "author_login": "1nonlypiece",
    "url": "https://github.com/owner/repo/issues/1",
    "assignees": [
      {
        "login": "1nonlypiece"
      }
    ],
    "labels": [
      {
        "name": "bug",
        "color": "d73a4a"
      },
      {
        "name": "documentation",
        "color": "0075ca"
      }
    ],
    "comments_count": 1,
    "comments": [
      {
        "id": 1234567890,
        "user": {
          "login": "commenter"
        },
        "body": "I can reproduce this issue...",
        "created_at": "2025-12-30T23:02:40.298969+05:30"
      }
    ],
    "created_at_github": "2025-12-30T22:56:03.058032+05:30",
    "updated_at_github": "2025-12-30T22:56:03.058032+05:30",
    "closed_at_github": null
  }
]
```

**Notes:**
- Returns latest 50 issues
- Includes assignees, labels, and comments
- Only includes issues from verified projects

---

### GET /projects/:id/prs

Get pull requests for a project.

**Authentication:** Required (JWT)

**URL Parameters:**
- `id` - Project UUID

**Response:**
```json
[
  {
    "id": "pr-uuid",
    "github_pr_id": 1234567890,
    "number": 42,
    "state": "open",
    "title": "Add new feature",
    "body": "This PR adds...",
    "author_login": "contributor",
    "url": "https://github.com/owner/repo/pull/42",
    "merged": false,
    "created_at_github": "2025-12-30T22:56:03.058032+05:30",
    "updated_at_github": "2025-12-30T22:56:03.058032+05:30",
    "merged_at_github": null,
    "closed_at_github": null
  }
]
```

**Notes:**
- Returns latest 50 PRs
- Only includes PRs from verified projects

---

### GET /projects/:id/events

Get webhook events for a project.

**Authentication:** Required (JWT)

**URL Parameters:**
- `id` - Project UUID

**Response:**
```json
[
  {
    "delivery_id": "abc123",
    "event": "issues",
    "action": "opened",
    "received_at": "2025-12-30T22:56:03.058032+05:30"
  }
]
```

---

## Public Projects

### GET /projects

Get a filtered list of verified projects (public endpoint).

**Authentication:** None required

**Query Parameters:**
- `ecosystem` (optional) - Filter by ecosystem name (case-insensitive)
- `language` (optional) - Filter by programming language
- `category` (optional) - Filter by category
- `tags` (optional) - Comma-separated list of tags (project must have ALL tags)
- `limit` (optional, default: 50, max: 200) - Number of results per page
- `offset` (optional, default: 0) - Pagination offset

**Example Request:**
```
GET /projects?ecosystem=Starknet&language=TypeScript&tags=good first issue,help wanted&limit=50&offset=0
```

**Response:**
```json
{
  "projects": [
    {
      "id": "79caaf9a-f1e6-4da0-be79-52c5bee169e1",
      "github_full_name": "owner/repo",
      "language": "TypeScript",
      "tags": ["good first issue", "help wanted"],
      "category": "Frontend",
      "ecosystem_name": "Starknet",
      "ecosystem_slug": "starknet",
      "created_at": "2025-12-30T21:25:50.85241+05:30",
      "updated_at": "2025-12-30T22:52:00.3484+05:30"
    }
  ],
  "total": 150,
  "limit": 50,
  "offset": 0
}
```

**Notes:**
- Only returns verified projects
- Multiple filters are combined with AND logic
- Tags filter requires project to have ALL specified tags

---

### GET /projects/filters

Get available filter options (languages, categories, tags) from verified projects.

**Authentication:** None required

**Response:**
```json
{
  "languages": ["Go", "TypeScript", "JavaScript", "Python"],
  "categories": ["Frontend", "Backend", "Full Stack"],
  "tags": ["good first issue", "help wanted", "documentation", "bug"]
}
```

**Notes:**
- Returns distinct values from verified projects only
- Useful for populating filter dropdowns

---

## Ecosystems

### GET /ecosystems

Get list of active ecosystems with project and user counts (public endpoint).

**Authentication:** None required

**Response:**
```json
{
  "ecosystems": [
    {
      "id": "ecosystem-uuid",
      "slug": "starknet",
      "name": "Starknet",
      "description": "A permissionless Validity-Rollup Layer 2 network",
      "website_url": "https://www.starknet.io",
      "status": "active",
      "project_count": 45,
      "user_count": 23,
      "created_at": "2025-12-30T21:25:50.85241+05:30",
      "updated_at": "2025-12-30T22:52:00.3484+05:30"
    }
  ]
}
```

**Notes:**
- Only returns active ecosystems
- `project_count` and `user_count` are computed dynamically
- Useful for populating ecosystem dropdowns

---

## Admin

All admin endpoints require:
1. Valid JWT token
2. User role must be `"admin"`

### POST /admin/bootstrap

Bootstrap the first admin user (or promote current user to admin if already admin).

**Authentication:** Required (JWT) + `X-Admin-Bootstrap-Token` header

**Headers:**
- `Authorization: Bearer <jwt_token>`
- `X-Admin-Bootstrap-Token: <ADMIN_BOOTSTRAP_TOKEN>`

**Request Body:** None

**Response:**
```json
{
  "ok": true,
  "token": "new_jwt_token_with_admin_role",
  "role": "admin"
}
```

**Error Responses:**
- `401 Unauthorized` - Invalid bootstrap token
- `403 Forbidden` - Admin already exists and user is not admin
- `503 Service Unavailable` - Bootstrap not configured

**Notes:**
- Only works if there are 0 admins in the database, OR the user is already an admin
- Returns a new JWT token with updated role (use this token for subsequent requests)

---

### GET /admin/users

Get list of users (admin only).

**Authentication:** Required (JWT, admin role)

**Response:**
```json
{
  "users": [
    {
      "id": "8420cb43-eb78-4aa8-b8fb-9d3ab0e2d7c8",
      "role": "contributor",
      "github_user_id": 92681651,
      "created_at": "2025-12-30T21:25:50.85241+05:30",
      "updated_at": "2025-12-30T22:52:00.3484+05:30"
    }
  ]
}
```

**Notes:**
- Returns latest 50 users
- Ordered by creation date (newest first)

---

### PUT /admin/users/:id/role

Update user role (admin only).

**Authentication:** Required (JWT, admin role)

**URL Parameters:**
- `id` - User UUID

**Request Body:**
```json
{
  "role": "maintainer"
}
```

**Valid Roles:**
- `"contributor"` - Default role
- `"maintainer"` - Can manage projects
- `"admin"` - Full access

**Response:**
```json
{
  "ok": true
}
```

**Error Responses:**
- `400 Bad Request` - Invalid role
- `404 Not Found` - User not found

---

### GET /admin/ecosystems

Get all ecosystems (admin only, includes inactive).

**Authentication:** Required (JWT, admin role)

**Response:**
```json
{
  "ecosystems": [
    {
      "id": "ecosystem-uuid",
      "slug": "starknet",
      "name": "Starknet",
      "description": "A permissionless Validity-Rollup Layer 2 network",
      "website_url": "https://www.starknet.io",
      "status": "active",
      "project_count": 45,
      "user_count": 23,
      "created_at": "2025-12-30T21:25:50.85241+05:30",
      "updated_at": "2025-12-30T22:52:00.3484+05:30"
    }
  ]
}
```

**Notes:**
- Includes both active and inactive ecosystems
- `project_count` and `user_count` are computed dynamically

---

### POST /admin/ecosystems

Create a new ecosystem (admin only).

**Authentication:** Required (JWT, admin role)

**Request Body:**
```json
{
  "name": "Ethereum",
  "description": "A decentralized platform for smart contracts",
  "website_url": "https://ethereum.org",
  "status": "active"
}
```

**Field Descriptions:**
- `name` (required) - Ecosystem name (slug is auto-generated)
- `description` (optional) - Ecosystem description
- `website_url` (optional) - Ecosystem website URL
- `status` (required) - Either `"active"` or `"inactive"`

**Response:**
```json
{
  "id": "ecosystem-uuid",
  "slug": "ethereum",
  "name": "Ethereum",
  "description": "A decentralized platform for smart contracts",
  "website_url": "https://ethereum.org",
  "status": "active",
  "project_count": 0,
  "user_count": 0,
  "created_at": "2025-12-30T21:25:50.85241+05:30",
  "updated_at": "2025-12-30T21:25:50.85241+05:30"
}
```

**Error Responses:**
- `400 Bad Request` - Missing required fields or invalid status

---

### PUT /admin/ecosystems/:id

Update an ecosystem (admin only).

**Authentication:** Required (JWT, admin role)

**URL Parameters:**
- `id` - Ecosystem UUID

**Request Body:**
```json
{
  "name": "Ethereum",
  "description": "Updated description",
  "website_url": "https://ethereum.org",
  "status": "active"
}
```

**Response:**
```json
{
  "id": "ecosystem-uuid",
  "slug": "ethereum",
  "name": "Ethereum",
  "description": "Updated description",
  "website_url": "https://ethereum.org",
  "status": "active",
  "project_count": 45,
  "user_count": 23,
  "created_at": "2025-12-30T21:25:50.85241+05:30",
  "updated_at": "2025-12-30T22:52:00.3484+05:30"
}
```

**Error Responses:**
- `400 Bad Request` - Invalid request
- `404 Not Found` - Ecosystem not found

---

## Webhooks

### POST /webhooks/github

GitHub webhook receiver (for GitHub to send events).

**Authentication:** None required (uses webhook secret for verification)

**Note:** This endpoint is called by GitHub, not by the frontend.

---

### GET /webhooks/didit
### POST /webhooks/didit

Didit KYC webhook receiver (for Didit to send status updates).

**Authentication:** None required

**Note:** This endpoint is called by Didit, not by the frontend.

---

## Error Responses

All endpoints may return the following error responses:

### 400 Bad Request
```json
{
  "error": "error_code",
  "message": "Human-readable error message (optional)"
}
```

### 401 Unauthorized
```json
{
  "error": "invalid_user"
}
```
or
```json
{
  "error": "invalid_token"
}
```

### 403 Forbidden
```json
{
  "error": "insufficient_permissions"
}
```

### 404 Not Found
```json
{
  "error": "resource_not_found"
}
```

### 409 Conflict
```json
{
  "error": "resource_exists",
  "message": "Detailed message"
}
```

### 503 Service Unavailable
```json
{
  "error": "service_not_configured",
  "message": "Service configuration missing"
}
```

---

## Common Patterns

### Authentication Flow

1. User clicks "Sign in with GitHub"
2. Frontend redirects to `/auth/github/login/start`
3. User authorizes on GitHub
4. GitHub redirects to `/auth/github/callback`
5. Backend processes OAuth and redirects to frontend with JWT token
6. Frontend stores JWT token
7. Frontend includes token in `Authorization: Bearer <token>` header for all authenticated requests

### Pagination

For paginated endpoints, use `limit` and `offset`:
- First page: `?limit=50&offset=0`
- Second page: `?limit=50&offset=50`
- Third page: `?limit=50&offset=100`

Use the `total` field in the response to calculate total pages.

### Date Formats

All dates are returned in ISO 8601 format:
- Date only: `"2025-12-30"`
- Date and time: `"2025-12-30T22:52:00.3484+05:30"` (with timezone)

### UUIDs

All IDs are UUIDs (version 4) returned as strings:
- Example: `"8420cb43-eb78-4aa8-b8fb-9d3ab0e2d7c8"`

---

## Frontend Integration Checklist

- [ ] Set base URL (e.g., `http://localhost:8080` for development)
- [ ] Implement GitHub OAuth flow
- [ ] Store JWT token securely (localStorage or secure cookie)
- [ ] Include JWT token in `Authorization` header for authenticated requests
- [ ] Handle token expiration (401 responses) and prompt re-login
- [ ] Implement error handling for all endpoints
- [ ] Use pagination for list endpoints
- [ ] Handle empty states (no data)
- [ ] Format dates for display
- [ ] Implement contribution calendar visualization using `level` field (0-4)
- [ ] Group activity feed by `month_year` field

---

## Example Frontend Code Snippets

### Making Authenticated Requests

```typescript
const token = localStorage.getItem('patchwork_jwt');

const response = await fetch('http://localhost:8080/profile', {
  headers: {
    'Authorization': `Bearer ${token}`,
    'Content-Type': 'application/json'
  }
});

const data = await response.json();
```

### Handling Errors

```typescript
if (!response.ok) {
  if (response.status === 401) {
    // Token expired, redirect to login
    localStorage.removeItem('patchwork_jwt');
    window.location.href = '/login';
  } else {
    const error = await response.json();
    console.error('API Error:', error.error, error.message);
  }
}
```

### Pagination Example

```typescript
async function fetchActivities(page: number = 0) {
  const limit = 50;
  const offset = page * limit;
  
  const response = await fetch(
    `http://localhost:8080/profile/activity?limit=${limit}&offset=${offset}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    }
  );
  
  const data = await response.json();
  // data.activities - array of activities
  // data.total - total count
  // data.limit, data.offset - pagination info
}
```

---

## Support

For questions or issues, refer to the backend codebase or contact the backend team.

**Last Updated:** 2025-12-31


















