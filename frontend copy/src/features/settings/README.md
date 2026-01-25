# Settings Feature - Modular Structure

## Overview
The Settings page has been refactored into a clean, modular, feature-based architecture with all tabs broken down into reusable components following glassmorphism design principles.

## Folder Structure

```
/src/features/settings/
├── components/
│   ├── shared/                 # Shared components
│   │   ├── SkeletonLoader.tsx
│   │   └── ToggleSwitch.tsx
│   ├── profile/               # Tab 1: Profile components
│   │   └── ProfileTab.tsx
│   ├── notifications/         # Tab 2: Notifications components
│   │   ├── NotificationRow.tsx
│   │   ├── NotificationSection.tsx
│   │   └── NotificationsTab.tsx
│   ├── payout/               # Tab 3: Payout components
│   │   └── PayoutTab.tsx
│   ├── billing/              # Tab 4: Billing components
│   │   ├── BillingProfileCard.tsx
│   │   └── BillingTab.tsx
│   └── terms/                # Tab 5: Terms components
│       └── TermsTab.tsx
├── data/
│   ├── billingProfilesData.ts
│   └── payoutProjectsData.ts
├── pages/
│   └── SettingsPage.tsx       # Main page with tab navigation
└── types/
    └── index.ts               # All TypeScript interfaces
```

## Features Completed

### ✅ Tab 1: Profile
- **GitHub Account** - Display GitHub username/email with resync and edit options
- **Profile Picture** - Upload and update profile picture
- **Personal Information** - Form fields for first name, last name, location, website, and bio
- **Contact Information** - Social media handles (Telegram, LinkedIn, WhatsApp, Twitter, Discord)
- Save functionality with glassmorphism button styles

### ✅ Tab 2: Notifications
- **NotificationRow** - Reusable row component with email and weekly toggle switches
- **NotificationSection** - Grouped notification categories
- **Global Notifications** - Billing profile and marketing notifications
- **Contributor Notifications** - Project, reward, and reward accepted notifications
- **Maintainer Notifications** - Project + contributor data and project + program notifications
- **Programs Notifications** - Transaction notifications
- **Sponsors Notifications** - Transaction notifications
- Enable/Disable all buttons
- Individual toggle switches with smooth animations

### ✅ Tab 3: Payout Preferences
- **Skeleton Loading** - 1-second loading animation on tab switch
- **Project List** - Table view with project avatars and names
- **Billing Profile Selector** - Dropdown to assign billing profiles to projects
- Info message about reward eligibility
- Clean grid layout with glassmorphism cards

### ✅ Tab 4: Billing Profiles
- **Profile List View** - Grid of billing profile cards
- **BillingProfileCard** - Individual profile card with status badges
- **Profile Detail View** - Three sub-tabs (General, Payment, Invoices)
- **General Information** - KYC verification workflow with 2-second simulation
- **Status Badges** - Verified, Missing Verification, Limit Reached states
- **Profile Types** - Individual, Self-Employed, Organization
- **Create Profile Modal** - Modal dialog with profile name and type selection
- Back navigation from detail to list view
- Reward limit display (5000 USD remaining)

### ✅ Tab 5: Terms and Conditions
- **Terms Content** - Formatted text with sections for Terms of Service, Privacy Policy, Data Collection, and User Responsibilities
- **Accept Button** - Call-to-action to accept terms
- Clean prose-style layout

## Shared Components

### ToggleSwitch
- Smooth animated toggle with gradient active state
- Clean white/transparent inactive state
- Duration-300 transitions

### SkeletonLoader
- Shimmer animation effect
- Customizable className for flexible sizing
- Used for loading states

## Key Design Patterns

1. **Feature-based organization** - Each tab owns its components
2. **Component composition** - Large components broken into smaller, reusable pieces
3. **Type safety** - Comprehensive TypeScript interfaces for all data structures
4. **Clean separation** - Clear separation between presentation and data logic
5. **Glassmorphism design** - Consistent warm neutral tones (beige/taupe) with heavy backdrop blur (25-40px)
6. **Smooth animations** - Transitions on hover states, toggle switches, and loading states

## Tab Navigation

The SettingsPage component provides:
- Horizontal tab navigation with 5 tabs
- Active tab highlighting with golden gradient background
- Hover states for inactive tabs
- Smooth transitions between tabs
- State management for active tab

## State Management

- **Profile Tab** - Form state for personal info and social media handles
- **Notifications Tab** - Complex notification settings object with 18 boolean flags
- **Payout Tab** - Loading state with 1-second skeleton animation
- **Billing Tab** - Profile list, selected profile, modal state, KYC verification state, and detail tab navigation
- **Terms Tab** - Static content with acceptance action

## Data Files

Mock data is provided for:
- 5 billing profiles (3 verified, 1 missing verification, 1 limit reached)
- 4 payout projects (React, Next.js, Vue.js, Angular)

All data files are located in `/src/features/settings/data/` for easy maintenance and updates.

## Key Interactions

1. **Tab Switching** - Instant tab content switching with no page reload
2. **Toggle Switches** - Click to toggle email/weekly notifications
3. **Profile Creation** - Modal workflow for creating new billing profiles
4. **KYC Verification** - Simulated 2-second verification process
5. **Profile Detail Navigation** - Click profile card to view details, back button to return to list

## Responsive Design

- All tabs are responsive with md: breakpoints
- Grid layouts adjust for mobile, tablet, and desktop
- Forms stack vertically on mobile
- Profile cards use 1/2/3 column grids based on screen size
