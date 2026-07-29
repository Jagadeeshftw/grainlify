/**
 * SearchModal — keyboard navigation & ARIA tests
 *
 * Coverage targets (≥95%):
 *  - All shortcut keys: j/k, ArrowDown/Up, /, ?, Enter, Esc, ⌘K, Ctrl+K
 *  - ARIA live region announcements
 *  - Focus management (open → trap → close → restore)
 *  - Cheat-sheet overlay toggle and dismiss
 *  - Edge cases: empty results, boundary navigation, cheat-sheet Esc priority
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SearchModal } from '../components/SearchModal';

/* ── mock ThemeContext ───────────────────────────────────────────────────── */

vi.mock('../contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark' }),
}));

/* ── helpers ─────────────────────────────────────────────────────────────── */

const onClose = vi.fn();

function renderModal(open = true) {
  return render(<SearchModal isOpen={open} onClose={onClose} />);
}

/* ── setup / teardown ────────────────────────────────────────────────────── */

beforeEach(() => {
  onClose.mockReset();
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  document.body.style.overflow = '';
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — rendering', () => {
  it('renders nothing when isOpen=false', () => {
    renderModal(false);
    expect(screen.queryByTestId('search-modal')).toBeNull();
  });

  it('renders the dialog when isOpen=true', () => {
    renderModal();
    expect(screen.getByRole('dialog', { name: /global search/i })).toBeInTheDocument();
  });

  it('has aria-modal="true"', () => {
    renderModal();
    expect(screen.getByTestId('search-modal')).toHaveAttribute('aria-modal', 'true');
  });

  it('locks body scroll on open', () => {
    renderModal();
    expect(document.body.style.overflow).toBe('hidden');
  });

  it('restores body scroll when isOpen flips to false', () => {
    const { rerender } = renderModal();
    expect(document.body.style.overflow).toBe('hidden');
    rerender(<SearchModal isOpen={false} onClose={onClose} />);
    expect(document.body.style.overflow).toBe('');
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — focus management', () => {
  it('focuses the search input on open', async () => {
    renderModal();
    act(() => vi.runAllTimers());
    expect(screen.getByTestId('search-input')).toHaveFocus();
  });

  it('returns focus to the previously focused element on close', async () => {
    const button = document.createElement('button');
    button.textContent = 'Trigger';
    document.body.appendChild(button);
    button.focus();

    const { rerender } = renderModal(true);
    act(() => vi.runAllTimers());

    rerender(<SearchModal isOpen={false} onClose={onClose} />);
    expect(button).toHaveFocus();
    document.body.removeChild(button);
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — Esc key', () => {
  it('calls onClose when Esc is pressed', () => {
    renderModal();
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('closes cheat-sheet first (not modal) when cheat-sheet is open', () => {
    renderModal();
    // open cheat-sheet
    fireEvent.click(screen.getByTestId('cheat-sheet-button'));
    expect(screen.getByTestId('cheat-sheet')).toBeInTheDocument();

    // Esc should close the cheat-sheet, not the modal
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'Escape' });
    expect(screen.queryByTestId('cheat-sheet')).toBeNull();
    expect(onClose).not.toHaveBeenCalled();

    // second Esc should close modal
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — / shortcut', () => {
  it('focuses the input when / is pressed from within the modal', () => {
    renderModal();
    // move focus away from input
    screen.getByTestId('close-button').focus();
    expect(screen.getByTestId('search-input')).not.toHaveFocus();

    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: '/' });
    expect(screen.getByTestId('search-input')).toHaveFocus();
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — j/k and ArrowDown/Up navigation', () => {
  it('moves selection down with j', () => {
    renderModal();
    // move focus off input so j is captured
    screen.getByTestId('close-button').focus();

    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'j' });
    const first = screen.getByTestId('result-item-0');
    expect(first).toHaveAttribute('aria-selected', 'true');
  });

  it('moves selection down with ArrowDown from input', () => {
    renderModal();
    act(() => vi.runAllTimers());
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowDown' });
    expect(screen.getByTestId('result-item-0')).toHaveAttribute('aria-selected', 'true');
  });

  it('moves selection up with k', () => {
    renderModal();
    screen.getByTestId('close-button').focus();

    // go down twice then up
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'j' });
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'j' });
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'k' });
    expect(screen.getByTestId('result-item-0')).toHaveAttribute('aria-selected', 'true');
  });

  it('moves selection up with ArrowUp', () => {
    renderModal();
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowDown' });
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowDown' });
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowUp' });
    expect(screen.getByTestId('result-item-0')).toHaveAttribute('aria-selected', 'true');
  });

  it('does not go below the last result (boundary)', () => {
    renderModal();
    // press j many times
    for (let i = 0; i < 20; i++) {
      fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowDown' });
    }
    const results = screen.getAllByRole('option');
    const lastIdx = results.length - 1;
    expect(screen.getByTestId(`result-item-${lastIdx}`)).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });

  it('does not go above -1 (boundary)', () => {
    renderModal();
    // pressing k without any down-move should not crash
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowUp' });
    // no item should be selected
    screen.getAllByRole('option').forEach((opt) => {
      expect(opt).toHaveAttribute('aria-selected', 'false');
    });
  });

  it('updates aria-activedescendant on the input', () => {
    renderModal();
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowDown' });
    const input = screen.getByTestId('search-input');
    expect(input).toHaveAttribute('aria-activedescendant', 's0');
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — Enter key', () => {
  it('navigates to the selected result href', () => {
    const originalLocation = window.location;
    // @ts-expect-error override for test
    delete window.location;
    window.location = { ...originalLocation, href: '' } as Location;

    renderModal();
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowDown' });
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'Enter' });

    expect(window.location.href).toBe('/search?q=terminal+markdown+editors');
    expect(onClose).toHaveBeenCalled();

    // restore
    window.location = originalLocation;
  });

  it('does nothing when no result is selected', () => {
    renderModal();
    // no ArrowDown — activeIndex is -1
    expect(() =>
      fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'Enter' })
    ).not.toThrow();
    expect(onClose).not.toHaveBeenCalled();
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — ? cheat-sheet overlay', () => {
  it('opens on button click', () => {
    renderModal();
    fireEvent.click(screen.getByTestId('cheat-sheet-button'));
    expect(screen.getByTestId('cheat-sheet')).toBeInTheDocument();
  });

  it('closes via close button inside overlay', () => {
    renderModal();
    fireEvent.click(screen.getByTestId('cheat-sheet-button'));
    fireEvent.click(screen.getByTestId('close-cheat-sheet'));
    expect(screen.queryByTestId('cheat-sheet')).toBeNull();
  });

  it('shows all 7 shortcuts in the overlay', () => {
    renderModal();
    fireEvent.click(screen.getByTestId('cheat-sheet-button'));
    const rows = screen.getAllByRole('row');
    // 7 shortcut rows (header is sr-only, table has 7 data rows)
    expect(rows.length).toBeGreaterThanOrEqual(7);
  });

  it('has aria-expanded on the cheat-sheet button', () => {
    renderModal();
    const btn = screen.getByTestId('cheat-sheet-button');
    expect(btn).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(btn);
    expect(btn).toHaveAttribute('aria-expanded', 'true');
  });

  it('does not open via ? key when input is focused', () => {
    renderModal();
    act(() => vi.runAllTimers());
    // input is focused — typing ? should not open overlay
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: '?' });
    expect(screen.queryByTestId('cheat-sheet')).toBeNull();
  });

  it('opens via ? key when a non-input element is focused', () => {
    renderModal();
    screen.getByTestId('close-button').focus();
    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: '?' });
    expect(screen.getByTestId('cheat-sheet')).toBeInTheDocument();
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — ARIA live region', () => {
  it('announces result count when a query is typed', () => {
    renderModal();
    act(() => vi.runAllTimers());

    fireEvent.change(screen.getByTestId('search-input'), {
      target: { value: 'Stellar' },
    });
    const live = screen.getByTestId('search-live-region');
    expect(live.textContent).toMatch(/results for 'Stellar'/i);
  });

  it('announces matching result count', () => {
    renderModal();
    act(() => vi.runAllTimers());

    fireEvent.change(screen.getByTestId('search-input'), {
      target: { value: 'markdown' },
    });
    const live = screen.getByTestId('search-live-region');
    expect(live.textContent).toMatch(/1 result for 'markdown'/i);
  });

  it('is empty when query is blank', () => {
    renderModal();
    const live = screen.getByTestId('search-live-region');
    expect(live.textContent).toBe('');
  });

  it('has aria-live="polite" and aria-atomic="true"', () => {
    renderModal();
    const live = screen.getByTestId('search-live-region');
    expect(live).toHaveAttribute('aria-live', 'polite');
    expect(live).toHaveAttribute('aria-atomic', 'true');
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — global ⌘K / Ctrl+K', () => {
  it('calls onClose when ⌘K is pressed while modal is open', () => {
    renderModal();
    fireEvent.keyDown(window, { key: 'k', metaKey: true });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when Ctrl+K is pressed while modal is open', () => {
    renderModal();
    fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('dispatches grainlify:open-search event when / is pressed outside the modal', () => {
    renderModal(false);
    const handler = vi.fn();
    window.addEventListener('grainlify:open-search', handler);

    // simulate user pressing / with no active text field
    Object.defineProperty(document, 'activeElement', {
      value: document.body,
      writable: true,
      configurable: true,
    });
    fireEvent.keyDown(window, { key: '/' });

    expect(handler).toHaveBeenCalledTimes(1);
    window.removeEventListener('grainlify:open-search', handler);
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — search filtering', () => {
  it('resets active selection when query changes', () => {
    renderModal();
    act(() => vi.runAllTimers());

    fireEvent.keyDown(screen.getByTestId('search-modal'), { key: 'ArrowDown' });
    expect(screen.getByTestId('result-item-0')).toHaveAttribute('aria-selected', 'true');

    fireEvent.change(screen.getByTestId('search-input'), {
      target: { value: 'a' },
    });
    // after typing, selection should reset
    const opts = screen.queryAllByRole('option');
    opts.forEach((o) => expect(o).toHaveAttribute('aria-selected', 'false'));
  });

  it('shows "No results found" when query matches nothing', () => {
    renderModal();
    act(() => vi.runAllTimers());

    fireEvent.change(screen.getByTestId('search-input'), {
      target: { value: 'xyzzy_no_match' },
    });
    expect(screen.getByText(/no results found/i)).toBeInTheDocument();
  });
});

/* ══════════════════════════════════════════════════════════════════════════ */

describe('SearchModal — mouse interaction', () => {
  it('sets activeIndex on mouse enter', () => {
    renderModal();
    fireEvent.mouseEnter(screen.getByTestId('result-item-2'));
    expect(screen.getByTestId('result-item-2')).toHaveAttribute('aria-selected', 'true');
  });

  it('closes modal when backdrop is clicked', () => {
    renderModal();
    // backdrop is the sibling div with aria-hidden; find by clicking outside dialog
    const backdrop = document
      .querySelector('[aria-hidden="true"]') as HTMLElement;
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalled();
  });

  it('closes modal when close button is clicked', () => {
    renderModal();
    fireEvent.click(screen.getByTestId('close-button'));
    expect(onClose).toHaveBeenCalled();
  });
});
