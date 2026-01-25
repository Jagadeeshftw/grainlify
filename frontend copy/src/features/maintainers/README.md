# Maintainers Feature - Modular Structure

## Overview
The Maintainers dashboard has been refactored into a clean, modular, feature-based architecture with all tabs now broken down into reusable components.

## Folder Structure

```
/src/features/maintainers/
├── components/
│   ├── dashboard/           # Tab 1: Dashboard components
│   │   ├── ActivityItem.tsx
│   │   ├── ApplicationsChart.tsx
│   │   ├── DashboardTab.tsx
│   │   └── StatsCard.tsx
│   ├── issues/             # Tab 2: Issues components
│   │   ├── ApplicationCard.tsx
│   │   ├── EmptyIssueState.tsx
│   │   ├── IssueCard.tsx
│   │   ├── IssueFilterDropdown.tsx
│   │   ├── IssueListSidebar.tsx
│   │   └── IssuesTab.tsx
│   └── pull-requests/      # Tab 3: Pull Requests components
│       ├── PRFilterDropdown.tsx
│       ├── PRRow.tsx
│       └── PullRequestsTab.tsx
├── data/
│   ├── issuesData.ts       # Issues mock data
│   └── pullRequestsData.ts # Pull requests mock data
├── pages/
│   └── MaintainersPage.tsx # Main page with tab navigation
└── types/
    └── index.ts            # All TypeScript interfaces
```

## Features Completed

### ✅ Tab 1: Dashboard
- **StatsCard** - Displays key metrics (total applications, pending, merged, rejected)
- **ActivityItem** - Individual activity entries for PRs and issues
- **ApplicationsChart** - Bar chart showing applications vs merged PRs by month
- **DashboardTab** - Main dashboard container with all stats and activity

### ✅ Tab 2: Issues
- **IssueCard** - Individual issue cards with tags, stats, and metadata
- **IssueFilterDropdown** - Filter dropdown (All, Waiting for review, In progress, Stale)
- **IssueListSidebar** - Left sidebar with filters, search, and scrollable issue list
- **EmptyIssueState** - Beautiful empty state when no issue is selected
- **ApplicationCard** - Shows applicant details with profile stats, badges, and actions
- **IssuesTab** - Main Issues tab with detail view and Applications/Discussions tabs
- Three application states: none, assigned, pending
- Discussion threads with author badges
- Clickable profile links

### ✅ Tab 3: Pull Requests
- **PRRow** - Individual PR row in the table view
- **PRFilterDropdown** - Filter by status (All states, Open, Merged, Closed, Draft)
- **PullRequestsTab** - Main PR tab with search, filters, and table view
- Status-based coloring (merged, draft, open, closed)
- Author badges with quality indicators
- Repository info and indicator icons

### ❌ Tab 4: Waves - REMOVED
The Waves tab has been completely removed from the codebase as requested.

## Key Design Patterns

1. **Feature-based organization** - Each feature owns its components, data, and types
2. **Component composition** - Large components broken into smaller, reusable pieces
3. **Type safety** - Comprehensive TypeScript interfaces for all data structures
4. **Clean separation** - Clear separation between presentation and data
5. **Glassmorphism design** - Consistent warm neutral tones with frosted glass effects

## Navigation

The MaintainersPage component provides:
- Repository selector dropdown with expandable organization list
- Tab navigation between Dashboard, Issues, and Pull Requests
- State management for active tab and selected repositories
- Routing through `onNavigate` prop for profile links

## Data

Mock data is provided for:
- Issues with applicants, discussions, tags, and metadata
- Pull requests with author badges, status, and indicators
- Dashboard stats and recent activity

All data files are located in `/src/features/maintainers/data/` for easy maintenance and updates.
