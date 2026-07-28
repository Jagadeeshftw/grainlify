# Currency Selector UX Design Specification

## Overview
This document outlines the UI/UX design and interactions for the multi-currency payout display and currency selector in `PayoutTab.tsx` and related views.

## 1. Currency Selector Control
**Anatomy & Location:**
- **Control Type:** Dropdown selector displaying the currency code and symbol (e.g., "USD ($)").
- **Searchability:** If the list of available currencies exceeds 8 items, a search input is introduced at the top of the dropdown.
- **Exposure:** Located within payout history views and invoice previews, ideally positioned at the top right of the data table or list header.

## 2. Dual-Display Convention
**Visual Layout:**
- **Primary:** The native token amount (e.g., `500 XLM`) is always shown as the primary value.
- **Secondary (Equivalent):** The selected display-currency equivalent is shown alongside or directly below the native amount.
- **Styling:** The equivalent amount must be presented in a de-emphasized style (e.g., smaller font size, muted text color from `/design-tokens.json`) accompanied by an "approximate" indicator (e.g., `~ $45.00 USD`).
- **Contrast Requirement:** Both the native amount and the de-emphasized equivalent amount must meet the WCAG 2.1 AA 4.5:1 contrast ratio against the background.

## 3. Settings & Persistence
**Location:** 
- The default display currency setting is located within `frontend/src/features/settings/pages/SettingsPage.tsx` under a "Preferences" or "Display Options" section.
**Persistence:**
- The selected currency is persisted to the user's account preferences in the backend and cached in local storage for immediate application on load.

## 4. States
- **Default-Currency-Selected:** The dropdown displays the currently active currency (e.g., "USD"). The UI reflects equivalent amounts based on this selection.
- **Selector-Open:** The dropdown expands, revealing the searchable list of supported currencies. Focus is managed correctly for keyboard navigation.
- **Rate-Unavailable:** If the exchange rate cannot be fetched, the UI falls back to a native-only display, hiding the approximate equivalent to avoid confusion.
- **Rate-Stale:** If the cached rate is older than the acceptable threshold, a subtle refresh indicator (e.g., a small warning icon with a tooltip) is displayed next to the equivalent amount.

## 5. Accessibility (a11y)
- **Screen Reader Announcements:** Dual amounts are associated using `aria-describedby`. The native amount element references the ID of the equivalent amount element so that screen readers announce both values consecutively (e.g., "500 XLM, approximately 45.00 US Dollars").
- **Keyboard-only Walkthrough:**
  1. User Tabs to the Currency Selector button and presses `Enter` or `Space` to open.
  2. Uses `Arrow Up/Down` to navigate the list (or types to search).
  3. Presses `Enter` to select a currency.
  4. Focus returns to the selector trigger, and an `aria-live` region announces the update to the displayed equivalents.

## 6. Responsive Design
- At narrow viewports (e.g., 375px), the dual-amount rows are ensured not to wrap awkwardly. If horizontal space is insufficient, the equivalent amount should stack neatly below the native amount while maintaining alignment with the rest of the row data.

## 7. Design Tokens Validation
- Colors for text (primary and muted), borders, and backgrounds must reference values strictly from `/design-tokens.json` to ensure consistency.
