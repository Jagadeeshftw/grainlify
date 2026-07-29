import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ContributionDiffViewer, type ContributionDiff } from '../ContributionDiffViewer';

vi.mock('../../../contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark' }),
}));

const diff: ContributionDiff = {
  title: 'Improve wallet flow',
  number: 42,
  url: 'https://github.com/org/repo/pull/42',
  additions: 2,
  deletions: 1,
  changedFiles: 1,
  files: [
    {
      path: 'src/wallet.ts',
      additions: 2,
      deletions: 1,
      isPartial: true,
      hunks: [
        {
          id: 'wallet-hunk',
          header: '@@ -10,4 +10,5 @@',
          rows: [
            {
              id: 'removed',
              left: { kind: 'removed', lineNumber: 10, content: 'return oldWallet;' },
              right: { kind: 'empty', content: '' },
            },
            {
              id: 'added',
              left: { kind: 'empty', content: '' },
              right: { kind: 'added', lineNumber: 10, content: 'return connectedWallet;' },
            },
            {
              kind: 'collapsed-hunk',
              id: 'wallet-collapsed',
              unchangedLines: 4,
              rows: [
                {
                  id: 'expanded-context',
                  left: { kind: 'context', lineNumber: 12, content: 'export default wallet;' },
                  right: { kind: 'context', lineNumber: 12, content: 'export default wallet;' },
                },
              ],
            },
            {
              id: 'context',
              left: { kind: 'context', lineNumber: 11, content: '}' },
              right: { kind: 'context', lineNumber: 11, content: '}' },
            },
          ],
        },
      ],
    },
  ],
};

function setMatchMedia(matches: boolean) {
  vi.stubGlobal('matchMedia', vi.fn().mockImplementation(() => ({
    matches,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  })));
}

describe('ContributionDiffViewer', () => {
  beforeEach(() => setMatchMedia(false));
  afterEach(() => vi.unstubAllGlobals());

  it('defaults to side-by-side on desktop and switches to inline with the toggle', async () => {
    const user = userEvent.setup();
    render(<ContributionDiffViewer diff={diff} />);

    expect(screen.getByRole('table', { name: 'Side-by-side diff for src/wallet.ts' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Side-by-side view' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText('Added line')).toBeInTheDocument();
    expect(screen.getByText('Removed line')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Inline view' }));
    expect(screen.getByRole('table', { name: 'Inline diff for src/wallet.ts' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Inline view' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('defaults to inline below the 768px breakpoint', () => {
    setMatchMedia(true);
    render(<ContributionDiffViewer diff={diff} />);

    expect(screen.getByRole('table', { name: 'Inline diff for src/wallet.ts' })).toBeInTheDocument();
  });

  it('expands collapsed unchanged hunks with a keyboard-operable button', async () => {
    const user = userEvent.setup();
    render(<ContributionDiffViewer diff={diff} defaultViewMode="inline" />);

    const expandButton = screen.getByRole('button', { name: '+4 lines unchanged' });
    expect(expandButton).toHaveAttribute('aria-expanded', 'false');
    expandButton.focus();
    await user.keyboard('{Enter}');

    expect(screen.getByRole('button', { name: 'Collapse unchanged lines' })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getAllByText('export default wallet;')).toHaveLength(1);
  });

  it('keeps the view toggle in the keyboard order', async () => {
    const user = userEvent.setup();
    render(<ContributionDiffViewer diff={diff} />);

    await user.tab();
    expect(document.activeElement).toHaveAccessibleName('Side-by-side view');
    await user.tab();
    expect(document.activeElement).toHaveAccessibleName('Inline view');
  });

  it('renders loading and binary-file states with explicit status text', () => {
    const { rerender } = render(<ContributionDiffViewer status="loading-diff" />);
    expect(screen.getByText('Loading diff')).toBeInTheDocument();
    expect(screen.getByRole('region')).toHaveAttribute('aria-busy', 'true');

    rerender(
      <ContributionDiffViewer
        diff={{ ...diff, files: [{ ...diff.files[0], isBinary: true, isPartial: false }] }}
      />,
    );
    expect(screen.getByText('Binary file preview is not supported.')).toBeInTheDocument();
  });

  it('renders an honest unsupported state when no patch is available', () => {
    render(<ContributionDiffViewer status="unsupported-preview" />);

    expect(screen.getByText('Diff preview unavailable')).toBeInTheDocument();
    expect(screen.getByText(/patch is not available/)).toBeInTheDocument();
  });

  it('calls the full-file callback from the file footer', async () => {
    const user = userEvent.setup();
    const onLoadFullFile = vi.fn().mockResolvedValue(undefined);
    render(<ContributionDiffViewer diff={diff} onLoadFullFile={onLoadFullFile} />);

    await user.click(screen.getByRole('button', { name: 'Load full file' }));
    await waitFor(() => expect(onLoadFullFile).toHaveBeenCalledWith('src/wallet.ts'));
  });
});
