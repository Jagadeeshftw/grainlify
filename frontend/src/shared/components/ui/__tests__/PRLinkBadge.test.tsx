/**
 * PRLinkBadge — unit & interaction tests
 *
 * Coverage targets:
 *  - Badge renders correct state per PR status (open, merged, closed, draft, multi)
 *  - Badge is hidden in unlinked state
 *  - Loading skeleton renders correctly
 *  - ARIA attributes are correct (aria-label, aria-expanded, aria-controls, role)
 *  - Keyboard: Enter/Space toggles preview, Escape closes, focus returns to badge
 *  - Mouse: hover opens preview after delay, mouse-leave closes it
 *  - Preview card content: title, author, status pill, statusDetail, GitHub link
 *  - Multi-PR preview shows list and "+ N more" overflow
 *  - Preview visibility via CSS (not display:none) so aria-describedby works
 *  - Click outside closes preview
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  render,
  screen,
  fireEvent,
  waitFor,
  act,
  within,
} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PRLinkBadge } from '../PRLinkBadge';
import { LinkedPR } from '../../../../features/maintainers/types';

/* ── mock ThemeContext ───────────────────────────────────────────────────── */

vi.mock('../../../contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark' }),
}));

/* ── fixtures ────────────────────────────────────────────────────────────── */

const openPR: LinkedPR = {
  id: 1,
  number: 42,
  title: 'Add KYC verification flow',
  status: 'open',
  statusDetail: 'opened 3 days ago',
  author: { name: 'alice', avatar: 'https://github.com/alice.png' },
  url: 'https://github.com/org/repo/pull/42',
};

const mergedPR: LinkedPR = {
  id: 2,
  number: 99,
  title: 'Fix RSC vulnerability',
  status: 'merged',
  statusDetail: 'merged 2 days ago by JagadeeshFtw',
  author: { name: 'vercel[bot]' },
  url: 'https://github.com/org/repo/pull/99',
};

const closedPR: LinkedPR = {
  id: 3,
  number: 10,
  title: 'WIP: Refactor auth',
  status: 'closed',
  statusDetail: 'closed 1 week ago',
  author: { name: 'bob' },
};

const draftPR: LinkedPR = {
  id: 4,
  number: 55,
  title: 'Draft: DAO governance',
  status: 'draft',
  statusDetail: 'opened 1 month ago • Draft',
  author: { name: 'carol' },
};

const multiPRs: LinkedPR[] = [openPR, mergedPR, closedPR];

/* ── helpers ─────────────────────────────────────────────────────────────── */

function renderBadge(overrides: Partial<Parameters<typeof PRLinkBadge>[0]> = {}) {
  return render(<PRLinkBadge issueId="issue-1" {...overrides} />);
}

/* ── tests ───────────────────────────────────────────────────────────────── */

describe('PRLinkBadge — badge states', () => {
  it('renders nothing when no PRs are linked', () => {
    const { container } = renderBadge({ linkedPRs: [] });
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when linkedPRs prop is absent', () => {
    const { container } = renderBadge();
    expect(container.firstChild).toBeNull();
  });

  it('renders a skeleton when linkedPRsLoading is true', () => {
    renderBadge({ linkedPRsLoading: true });
    expect(screen.getByLabelText('Loading pull request data')).toBeInTheDocument();
  });

  it('renders PR Open badge for an open PR', () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    expect(btn).toHaveAttribute('aria-label', '1 linked pull request — open');
    expect(btn).toHaveTextContent('PR Open');
  });

  it('renders Merged badge for a merged PR', () => {
    renderBadge({ linkedPRs: [mergedPR] });
    const btn = screen.getByRole('button');
    expect(btn).toHaveAttribute('aria-label', '1 linked pull request — merged');
    expect(btn).toHaveTextContent('Merged');
  });

  it('renders Closed badge for a closed PR', () => {
    renderBadge({ linkedPRs: [closedPR] });
    const btn = screen.getByRole('button');
    expect(btn).toHaveAttribute('aria-label', '1 linked pull request — closed');
    expect(btn).toHaveTextContent('Closed');
  });

  it('renders Draft badge for a draft PR', () => {
    renderBadge({ linkedPRs: [draftPR] });
    const btn = screen.getByRole('button');
    expect(btn).toHaveAttribute('aria-label', '1 linked pull request — draft');
    expect(btn).toHaveTextContent('Draft');
  });

  it('renders multi-PR count badge when 2+ PRs are linked', () => {
    renderBadge({ linkedPRs: multiPRs });
    const btn = screen.getByRole('button');
    expect(btn).toHaveAttribute('aria-label', '3 linked pull requests');
    expect(btn).toHaveTextContent('3 PRs');
  });
});

describe('PRLinkBadge — ARIA', () => {
  it('badge starts with aria-expanded="false"', () => {
    renderBadge({ linkedPRs: [openPR] });
    expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'false');
  });

  it('badge has aria-controls pointing to an element in the DOM', () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    const controlsId = btn.getAttribute('aria-controls');
    expect(controlsId).toBeTruthy();
    expect(document.getElementById(controlsId!)).toBeInTheDocument();
  });

  it('preview element has role="tooltip"', () => {
    renderBadge({ linkedPRs: [openPR] });
    expect(screen.getByRole('tooltip')).toBeInTheDocument();
  });

  it('preview is invisible by default but present in the DOM', () => {
    renderBadge({ linkedPRs: [openPR] });
    const tooltip = screen.getByRole('tooltip');
    expect(tooltip).toHaveStyle('visibility: hidden');
    expect(tooltip).toBeInTheDocument();
  });

  it('preview has aria-live="polite"', () => {
    renderBadge({ linkedPRs: [openPR] });
    expect(screen.getByRole('tooltip')).toHaveAttribute('aria-live', 'polite');
  });
});

describe('PRLinkBadge — keyboard interactions', () => {
  it('Enter opens the preview and sets aria-expanded="true"', async () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    btn.focus();
    await userEvent.keyboard('{Enter}');
    expect(btn).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('tooltip')).toHaveStyle('visibility: visible');
  });

  it('Space opens the preview', async () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    btn.focus();
    await userEvent.keyboard(' ');
    expect(btn).toHaveAttribute('aria-expanded', 'true');
  });

  it('Enter a second time closes the preview', async () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    btn.focus();
    await userEvent.keyboard('{Enter}');
    await userEvent.keyboard('{Enter}');
    expect(btn).toHaveAttribute('aria-expanded', 'false');
  });

  it('Escape closes the preview and returns focus to the badge', async () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    btn.focus();
    await userEvent.keyboard('{Enter}');
    expect(btn).toHaveAttribute('aria-expanded', 'true');
    await userEvent.keyboard('{Escape}');
    expect(btn).toHaveAttribute('aria-expanded', 'false');
    expect(document.activeElement).toBe(btn);
  });
});

describe('PRLinkBadge — mouse interactions', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('opens preview after a 150ms hover delay', () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    fireEvent.mouseEnter(btn);
    expect(screen.getByRole('tooltip')).toHaveStyle('visibility: hidden');
    act(() => { vi.advanceTimersByTime(150); });
    expect(screen.getByRole('tooltip')).toHaveStyle('visibility: visible');
  });

  it('closes preview after a 100ms mouse-leave delay', () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    fireEvent.mouseEnter(btn);
    act(() => { vi.advanceTimersByTime(150); });
    expect(screen.getByRole('tooltip')).toHaveStyle('visibility: visible');
    fireEvent.mouseLeave(btn);
    act(() => { vi.advanceTimersByTime(100); });
    expect(screen.getByRole('tooltip')).toHaveStyle('visibility: hidden');
  });

  it('hovering into the preview cancels the close timer', () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    const tooltipWrapper = screen.getByRole('tooltip').parentElement!;
    fireEvent.mouseEnter(btn);
    act(() => { vi.advanceTimersByTime(150); });
    // leave badge, immediately enter preview wrapper
    fireEvent.mouseLeave(btn);
    fireEvent.mouseEnter(tooltipWrapper);
    act(() => { vi.advanceTimersByTime(200); }); // close timer should have been cancelled
    expect(screen.getByRole('tooltip')).toHaveStyle('visibility: visible');
  });

  it('leaving preview closes it after 100ms', () => {
    renderBadge({ linkedPRs: [openPR] });
    const btn = screen.getByRole('button');
    const tooltipWrapper = screen.getByRole('tooltip').parentElement!;
    fireEvent.mouseEnter(btn);
    act(() => { vi.advanceTimersByTime(150); });
    fireEvent.mouseLeave(tooltipWrapper);
    act(() => { vi.advanceTimersByTime(100); });
    expect(screen.getByRole('tooltip')).toHaveStyle('visibility: hidden');
  });
});

describe('PRLinkBadge — single PR preview content', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  function openPreview() {
    const btn = screen.getByRole('button');
    fireEvent.mouseEnter(btn);
    act(() => { vi.advanceTimersByTime(150); });
    return screen.getByRole('tooltip');
  }

  it('shows PR number and title', () => {
    renderBadge({ linkedPRs: [openPR] });
    const tooltip = openPreview();
    expect(within(tooltip).getByText(/#42/)).toBeInTheDocument();
    expect(within(tooltip).getByText(/Add KYC verification flow/)).toBeInTheDocument();
  });

  it('shows the author name', () => {
    renderBadge({ linkedPRs: [openPR] });
    const tooltip = openPreview();
    expect(within(tooltip).getByText('alice')).toBeInTheDocument();
  });

  it('shows statusDetail text', () => {
    renderBadge({ linkedPRs: [openPR] });
    const tooltip = openPreview();
    expect(within(tooltip).getByText('opened 3 days ago')).toBeInTheDocument();
  });

  it('renders an author avatar img', () => {
    renderBadge({ linkedPRs: [openPR] });
    openPreview();
    const img = screen.getAllByRole('img').find(
      (el) => el.getAttribute('alt') === 'alice',
    );
    expect(img).toBeInTheDocument();
  });

  it('renders "Open on GitHub" link with correct href and rel', () => {
    renderBadge({ linkedPRs: [openPR] });
    openPreview();
    const link = screen.getByRole('link', { name: /open on github/i });
    expect(link).toHaveAttribute('href', openPR.url);
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('does not render a GitHub link when url is absent', () => {
    renderBadge({ linkedPRs: [closedPR] });
    openPreview();
    expect(screen.queryByRole('link', { name: /open on github/i })).toBeNull();
  });

  it('renders initials fallback when avatar is absent', () => {
    renderBadge({ linkedPRs: [closedPR] }); // closedPR has no avatar
    openPreview();
    // No img with alt=bob; instead an initials span with text "BO"
    expect(screen.queryByRole('img', { name: 'bob' })).toBeNull();
    expect(screen.getByText('BO')).toBeInTheDocument();
  });
});

describe('PRLinkBadge — multi-PR preview content', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  function openPreview() {
    const btn = screen.getByRole('button');
    fireEvent.mouseEnter(btn);
    act(() => { vi.advanceTimersByTime(150); });
    return screen.getByRole('tooltip');
  }

  it('shows heading with PR count', () => {
    renderBadge({ linkedPRs: multiPRs });
    const tooltip = openPreview();
    expect(within(tooltip).getByText('3 Pull Requests linked')).toBeInTheDocument();
  });

  it('lists all PR titles', () => {
    renderBadge({ linkedPRs: multiPRs });
    const tooltip = openPreview();
    expect(within(tooltip).getByText(/Add KYC verification flow/)).toBeInTheDocument();
    expect(within(tooltip).getByText(/Fix RSC vulnerability/)).toBeInTheDocument();
    expect(within(tooltip).getByText(/WIP: Refactor auth/)).toBeInTheDocument();
  });

  it('renders status pills for each PR', () => {
    renderBadge({ linkedPRs: multiPRs });
    const tooltip = openPreview();
    expect(within(tooltip).getByText('Open')).toBeInTheDocument();
    expect(within(tooltip).getByText('Merged')).toBeInTheDocument();
    expect(within(tooltip).getByText('Closed')).toBeInTheDocument();
  });

  it('shows "+ N more" when more than 5 PRs are linked', () => {
    const many: LinkedPR[] = Array.from({ length: 7 }, (_, i) => ({
      ...openPR,
      id: i,
      number: 100 + i,
      title: `PR number ${i}`,
    }));
    renderBadge({ linkedPRs: many });
    const tooltip = openPreview();
    expect(within(tooltip).getByText('+ 2 more')).toBeInTheDocument();
  });

  it('does not show "+ N more" when exactly 5 PRs are linked', () => {
    const five: LinkedPR[] = Array.from({ length: 5 }, (_, i) => ({
      ...openPR,
      id: i,
      number: 200 + i,
      title: `PR ${i}`,
    }));
    renderBadge({ linkedPRs: five });
    const tooltip = openPreview();
    expect(within(tooltip).queryByText(/more/)).toBeNull();
  });
});

describe('PRLinkBadge — outside click', () => {
  it('closes preview when clicking outside both badge and preview', async () => {
    render(
      <div>
        <PRLinkBadge issueId="i1" linkedPRs={[openPR]} />
        <button data-testid="outside">outside</button>
      </div>,
    );
    const btn = screen.getByRole('button', { name: /1 linked pull request/i });
    fireEvent.click(btn);
    expect(btn).toHaveAttribute('aria-expanded', 'true');
    fireEvent.mouseDown(screen.getByTestId('outside'));
    await waitFor(() => {
      expect(btn).toHaveAttribute('aria-expanded', 'false');
    });
  });
});
