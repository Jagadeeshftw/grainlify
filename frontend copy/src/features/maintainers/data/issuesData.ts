import { Issue } from '../types';

export const issuesData: Issue[] = [
  {
    id: 133,
    title: 'Add Comprehensive Test Coverage for Scheduling Module',
    repo: 'stellopay-core',
    comments: 5,
    applicants: 1,
    tags: ['good first issue', 'onlydust-wave', 'smart-contract', 'soroban', 'test'],
    user: 'JagadeeshItw',
    timeAgo: '3 months ago',
    icon: 'rocket',
    applicationStatus: 'assigned',
    applicant: {
      name: 'Benjitaikchow',
      appliedDate: 'Sep 23, 2025',
      badge: 'Gold Contributor',
      stats: [
        { label: 'High OSS XP', value: 'Top 20%', color: 'golden' },
        { label: 'High PR Merge Rate', value: '80%', color: 'golden' },
        { label: 'Highly Reliable Contributor', value: '97%', color: 'golden' }
      ],
      profileStats: {
        contributions: 165,
        rewards: 4,
        contributorProjects: 48,
        leadProjects: 3
      },
      message: 'Hi @JagadeeshItw I can handle this task. Kindly assign. Highly recommended by {Onlydust}(https://www.onlydust.com/) for strong commitment and contributions!'
    },
    discussions: [
      {
        id: 1,
        user: 'JagadeeshItw',
        timeAgo: '3 months ago',
        isAuthor: true,
        content: `The scheduling module needs comprehensive test coverage to ensure reliability and prevent regressions.

**Requirements:**
• Unit tests for all scheduling functions
• Integration tests for schedule execution
• Edge case testing for timezone handling
• Performance tests for large schedules
• Mock external dependencies properly

**Acceptance Criteria:**
☑ 90%+ code coverage for scheduling module
☑ All edge cases covered
☑ Integration tests passing
☑ Performance benchmarks met
☑ Documentation updated`
      },
      {
        id: 2,
        user: 'Benjitaikchow',
        timeAgo: '3 months ago',
        appliedForContribution: true,
        content: 'Hi @JagadeeshItw I can handle this task. Kindly assign. Highly recommended by {Onlydust}(https://www.onlydust.com/) for strong commitment and contributions!'
      },
      {
        id: 3,
        user: 'JagadeeshItw',
        timeAgo: '3 months ago',
        isAuthor: true,
        content: 'Assigned! Looking forward to your contribution.'
      }
    ]
  },
  {
    id: 77,
    title: 'Add Invoice Expiration and Auto-Processing',
    repo: 'stellopay-core',
    comments: 0,
    applicants: 1,
    tags: ['onlydust-wave'],
    user: 'Baskarayelu',
    timeAgo: '5 months ago',
    icon: 'rocket',
    applicationStatus: 'pending',
    applicant: {
      name: 'FrankiePower',
      appliedDate: 'Oct 8, 2025',
      badge: 'Silver Contributor',
      stats: [
        { label: 'Moderate OSS XP', value: 'Top 28%', color: 'orange' },
        { label: 'Excellent PR Merge Rate', value: '95%', color: 'green' },
        { label: 'Moderate Completion Rate', value: '56%', color: 'orange' }
      ],
      profileStats: {
        contributions: 89,
        rewards: 2,
        contributorProjects: 18,
        leadProjects: 1
      },
      message: 'can i handle this?'
    },
    discussions: [
      {
        id: 1,
        user: 'Baskarayelu',
        timeAgo: '5 months ago',
        isAuthor: true,
        content: `Currently, invoice expiration is basic. We need sophisticated expiration and auto-processing logic.

**Requirements:**
• Implement configurable expiration rules
• Add auto-processing triggers
• Create expiration notification system
• Implement grace period functionality
• Add expiration fee structures
• Create expiration history tracking

**Acceptance Criteria:**
☑ Configurable expiration rules
☑ Auto-processing triggers
☑ Expiration notifications
☑ Grace period management
☑ Expiration fee calculation
☑ Expiration history tracking
☑ Expiration override mechanisms

**Technical Notes:**
• Consider using time-based triggers
• Implement proper state transitions
• Add maximum grace period limits`
      },
      {
        id: 2,
        user: 'FrankiePower',
        timeAgo: '3 months ago',
        appliedForContribution: true,
        content: 'can i handle this?'
      }
    ]
  },
  {
    id: 76,
    title: 'Add Invoice Categories and Tagging System',
    repo: 'stellopay-core',
    comments: 0,
    applicants: 0,
    tags: ['onlydust-wave'],
    user: 'Baskarayelu',
    timeAgo: '5 months ago',
    icon: 'rocket',
    applicationStatus: 'none'
  },
  {
    id: 130,
    title: 'Add Comprehensive Test Coverage for Enterprise Module',
    repo: 'stellopay-core',
    comments: 5,
    applicants: 0,
    tags: ['good first issue', 'onlydust-wave', 'smart-contract', 'soroban', 'test'],
    user: 'JagadeeshHtv',
    timeAgo: '3 months ago',
    icon: 'rocket',
    applicationStatus: 'none'
  },
  {
    id: 127,
    title: 'Add Advanced Payroll Security and Fraud Prevention',
    repo: 'stellopay-core',
    comments: 2,
    applicants: 0,
    tags: ['enhancement', 'good first issue', 'help wanted', 'onlydust-wave', 'smart-contract', 'soroban'],
    user: 'JagadeeshHtv',
    timeAgo: '3 months ago',
    icon: 'users',
    applicationStatus: 'none'
  },
  {
    id: 126,
    title: 'Add Advanced Multi-Currency and International Support',
    repo: 'stellopay-core',
    comments: 1,
    applicants: 0,
    tags: [],
    user: 'JagadeeshHtv',
    timeAgo: '3 months ago',
    icon: 'user',
    applicationStatus: 'none'
  },
];
