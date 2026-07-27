/**
 * @file The 6-step contributor onboarding flow.
 *
 * Each step pairs a persistent anchor ({@link TOUR_TARGET}) with the dashboard
 * page it teaches (`data.page`). The provider navigates to `data.page` before
 * the step is shown, so the tour doubles as a guided walk through the product:
 *   1. Find a bounty      → Discover
 *   2. Connect your wallet → Settings (Payout Preferences)
 *   3. Claim an issue      → Browse
 *   4. Track your progress → Public Profile
 *   5. View your rewards   → Leaderboard
 *   6. Complete your profile → account menu
 */

import type { Step } from 'react-joyride';
import { TOUR_TARGET, tourSelector } from './targets';

/** Page keys understood by the dashboard's (`Dashboard.tsx`) `currentPage` switch. */
export type TourPage = 'discover' | 'browse' | 'profile' | 'leaderboard' | 'settings';

/** Extra payload carried on each `Step.data`. */
export interface TourStepData {
  /** Dashboard page to navigate to before the step renders. */
  page: TourPage;
  /** Stable machine id — used for analytics and aria ids. */
  key: string;
}

/** A tour step with strongly-typed `data`. Assignable to Joyride's `Step`. */
export type TourStep = Step & { data: TourStepData };

export const tourSteps: TourStep[] = [
  {
    target: tourSelector(TOUR_TARGET.navDiscover),
    placement: 'right',
    title: 'Find a bounty',
    content:
      'Welcome to Grainlify! Start on Discover to see open-source projects and ' +
      'bounties matched to your skills. Browse the recommendations to spot an ' +
      'issue worth claiming.',
    data: { page: 'discover', key: 'find-bounty' },
  },
  {
    target: tourSelector(TOUR_TARGET.navSettings),
    placement: 'right',
    title: 'Connect your wallet',
    content:
      'Open Settings → Payout Preferences to connect the wallet where your ' +
      'rewards will be paid. Do this once, before you claim your first issue, ' +
      'so payouts have somewhere to land.',
    data: { page: 'settings', key: 'connect-wallet' },
  },
  {
    target: tourSelector(TOUR_TARGET.navBrowse),
    placement: 'right',
    title: 'Claim an issue',
    content:
      'Use Browse to filter every open issue. Open one that fits, then claim ' +
      'it to tell maintainers you are on it. Claiming reserves the bounty while ' +
      'you work.',
    data: { page: 'browse', key: 'claim-issue' },
  },
  {
    target: tourSelector(TOUR_TARGET.topbarProfile),
    placement: 'bottom-end',
    title: 'Track your progress',
    content:
      'Open your profile from the account menu — your contributor home base. ' +
      'Claimed issues, open pull requests, and contribution history all update ' +
      'here as you work.',
    data: { page: 'profile', key: 'track-progress' },
  },
  {
    target: tourSelector(TOUR_TARGET.navLeaderboard),
    placement: 'right',
    title: 'View your rewards',
    content:
      'The Leaderboard shows the rewards you have earned and how you rank among ' +
      'contributors. Watch your standing climb as merged contributions pay out.',
    data: { page: 'leaderboard', key: 'view-rewards' },
  },
  {
    target: tourSelector(TOUR_TARGET.topbarProfile),
    placement: 'bottom-end',
    title: 'Complete your profile',
    content:
      'Finally, finish your profile from the account menu. A complete profile ' +
      'sharpens your bounty matches and helps maintainers trust your claims. ' +
      'You are all set — happy contributing!',
    data: { page: 'profile', key: 'complete-profile' },
  },
];
