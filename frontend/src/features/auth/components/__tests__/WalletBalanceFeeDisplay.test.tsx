import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeProvider } from '../../../shared/contexts/ThemeContext';
import { WalletBalanceFeeDisplay } from '../WalletBalanceFeeDisplay';
import { WALLET_FEE_DISCLOSURE_COPY } from '../walletFeeDisclosureCopy';

const baseProps = {
  balance: '1,245.80',
  usdEquivalent: 312.45,
  estimatedFee: '0.00001',
  feeUsdEquivalent: 0.0000025,
  isLoading: false,
  isStale: false,
  lastUpdated: null as Date | null,
};

function renderDisplay(overrides: Partial<typeof baseProps & { ticker?: string }> = {}) {
  localStorage.setItem('theme', 'light');
  return render(
    <ThemeProvider>
      <WalletBalanceFeeDisplay {...baseProps} {...overrides} />
    </ThemeProvider>
  );
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.className = '';
});

afterEach(() => {
  localStorage.clear();
  vi.useRealTimers();
});

describe('WalletBalanceFeeDisplay — visibility', () => {
  it('renders nothing when balance is null (wallet not connected)', () => {
    const { container } = renderDisplay({ balance: null });
    expect(container.firstChild).toBeNull();
  });

  it('shows loading skeleton with busy state', () => {
    renderDisplay({ isLoading: true });
    expect(screen.getByLabelText(WALLET_FEE_DISCLOSURE_COPY.loadingAriaLabel)).toHaveAttribute(
      'aria-busy',
      'true'
    );
  });
});

describe('WalletBalanceFeeDisplay — fee disclosure copy', () => {
  it('shows network fee label and XLM amount with sub-cent USD', () => {
    renderDisplay();
    expect(screen.getByText(WALLET_FEE_DISCLOSURE_COPY.feeLabel)).toBeInTheDocument();
    expect(screen.getByText(/0\.00001/)).toBeInTheDocument();
    expect(screen.getByText(/< \$0\.01/)).toBeInTheDocument();
  });

  it('reveals expanded fee tooltip on info button click', async () => {
    const user = userEvent.setup();
    renderDisplay();
    await user.click(screen.getByRole('button', { name: WALLET_FEE_DISCLOSURE_COPY.infoButtonAriaLabel }));
    expect(screen.getByRole('tooltip')).toHaveTextContent(WALLET_FEE_DISCLOSURE_COPY.feeTooltip);
  });

  it('shows fee-unavailable copy and accessible alert icon', () => {
    renderDisplay({ estimatedFee: null, feeUsdEquivalent: null });
    expect(screen.getByText(WALLET_FEE_DISCLOSURE_COPY.feeUnavailable)).toBeInTheDocument();
    expect(
      screen.getByLabelText(WALLET_FEE_DISCLOSURE_COPY.feeUnavailableAriaLabel)
    ).toBeInTheDocument();
  });

  it('omits fee USD line when feeUsdEquivalent is null but native fee exists', () => {
    renderDisplay({ feeUsdEquivalent: null });
    expect(screen.getByText(/0\.00001/)).toBeInTheDocument();
    expect(screen.queryByText(/≈ \$/)).not.toBeInTheDocument();
  });

  it('shows USD unavailable copy when balance USD is missing', () => {
    renderDisplay({ usdEquivalent: null });
    expect(screen.getByText(WALLET_FEE_DISCLOSURE_COPY.usdUnavailable)).toBeInTheDocument();
  });
});

describe('WalletBalanceFeeDisplay — edge states', () => {
  it('flags insufficient balance with alert and aria-invalid', () => {
    renderDisplay({ balance: '0.00' });
    expect(screen.getByRole('alert')).toHaveTextContent(
      WALLET_FEE_DISCLOSURE_COPY.insufficientBalanceAlert
    );
    expect(screen.getByText('0.00').closest('[aria-invalid="true"]')).toBeTruthy();
  });

  it('parses comma-separated balances for insufficient check', () => {
    renderDisplay({ balance: '0,00' });
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('shows stale banner and last-updated label on clock when lastUpdated is set', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-07-27T12:00:00.000Z'));
    const lastUpdated = new Date('2026-07-27T11:55:00.000Z');
    renderDisplay({ isStale: true, lastUpdated });
    expect(screen.getByText(WALLET_FEE_DISCLOSURE_COPY.staleBanner)).toBeInTheDocument();
    expect(screen.getByLabelText('Balance last updated 5m ago')).toBeInTheDocument();
  });

  it('uses custom ticker in fee row', () => {
    renderDisplay({ ticker: 'TEST' });
    expect(screen.getAllByText('TEST').length).toBeGreaterThanOrEqual(1);
  });
});

describe('WalletBalanceFeeDisplay — live region', () => {
  it('announces balance updates politely', () => {
    renderDisplay();
    expect(screen.getByText('1,245.80').closest('[aria-live="polite"]')).toBeTruthy();
  });
});

describe('WalletBalanceFeeDisplay — deterministic behavior', () => {
  it('generates deterministic tooltip IDs across renders', () => {
    const { rerender } = renderDisplay();
    const tooltipButton = screen.getByRole('button', {
      name: WALLET_FEE_DISCLOSURE_COPY.infoButtonAriaLabel,
    });
    const firstAriaDescribedby = tooltipButton.getAttribute('aria-describedby');

    // Rerender with same props should not change the tooltip ID
    rerender(
      <ThemeProvider>
        <WalletBalanceFeeDisplay {...baseProps} />
      </ThemeProvider>
    );
    const secondAriaDescribedby = tooltipButton.getAttribute('aria-describedby');

    expect(firstAriaDescribedby).toBe(secondAriaDescribedby);
    expect(firstAriaDescribedby).toMatch(/^tooltip-\d+$/);
  });

  it('generates unique tooltip IDs for multiple instances', () => {
    render(
      <ThemeProvider>
        <div>
          <WalletBalanceFeeDisplay {...baseProps} />
          <WalletBalanceFeeDisplay {...baseProps} balance="2,000.00" />
        </div>
      </ThemeProvider>
    );
    const buttons = screen.getAllByRole('button', {
      name: WALLET_FEE_DISCLOSURE_COPY.infoButtonAriaLabel,
    });
    const ids = buttons.map((btn) => btn.getAttribute('aria-describedby'));
    expect(new Set(ids).size).toBe(2);
  });
});

describe('WalletBalanceFeeDisplay — edge case: partial data', () => {
  it('handles null balance with null fee (not connected)', () => {
    const { container } = renderDisplay({ balance: null, estimatedFee: null });
    expect(container.firstChild).toBeNull();
  });

  it('handles null balance with available fee (edge case)', () => {
    const { container } = renderDisplay({ balance: null, estimatedFee: '0.00001' });
    expect(container.firstChild).toBeNull();
  });

  it('handles zero balance with available fee', () => {
    renderDisplay({ balance: '0.00', estimatedFee: '0.00001' });
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/0\.00001/)).toBeInTheDocument();
  });

  it('handles negative balance (edge case parsing)', () => {
    renderDisplay({ balance: '-1.00' });
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('handles very small positive balance (below fee threshold)', () => {
    renderDisplay({ balance: '0.00001' });
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});

describe('WalletBalanceFeeDisplay — edge case: fee disclosure clarity', () => {
  it('shows fee tooltip when fee is available', () => {
    renderDisplay();
    expect(
      screen.getByRole('button', { name: WALLET_FEE_DISCLOSURE_COPY.infoButtonAriaLabel })
    ).toBeInTheDocument();
  });

  it('hides fee tooltip when fee is unavailable', () => {
    renderDisplay({ estimatedFee: null, feeUsdEquivalent: null });
    expect(
      screen.queryByRole('button', { name: WALLET_FEE_DISCLOSURE_COPY.infoButtonAriaLabel })
    ).not.toBeInTheDocument();
  });

  it('shows alert icon with aria-label when fee unavailable', () => {
    renderDisplay({ estimatedFee: null, feeUsdEquivalent: null });
    expect(
      screen.getByLabelText(WALLET_FEE_DISCLOSURE_COPY.feeUnavailableAriaLabel)
    ).toBeInTheDocument();
  });

  it('reduces opacity of fee row when balance insufficient', () => {
    const { container } = renderDisplay({ balance: '0.00' });
    const feeRow = container.querySelector('[class*="border-t"]');
    expect(feeRow).toHaveClass('opacity-60');
  });
});

describe('WalletBalanceFeeDisplay — edge case: stale data', () => {
  it('shows stale banner when isStale is true', () => {
    renderDisplay({ isStale: true, lastUpdated: null });
    expect(screen.getByText(WALLET_FEE_DISCLOSURE_COPY.staleBanner)).toBeInTheDocument();
  });

  it('shows clock icon with aria-label when stale with lastUpdated', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-07-27T12:00:00.000Z'));
    const lastUpdated = new Date('2026-07-27T11:55:00.000Z');
    renderDisplay({ isStale: true, lastUpdated });
    expect(screen.getByLabelText('Balance last updated 5m ago')).toBeInTheDocument();
    vi.useRealTimers();
  });

  it('shows generic aria-label when stale without lastUpdated', () => {
    renderDisplay({ isStale: true, lastUpdated: null });
    expect(screen.getByLabelText('Balance may be outdated')).toBeInTheDocument();
  });
});

describe('WalletBalanceFeeDisplay — edge case: USD formatting', () => {
  it('shows < $0.01 for sub-cent fee USD', () => {
    renderDisplay({ feeUsdEquivalent: 0.0000025 });
    expect(screen.getByText(/< \$0\.01/)).toBeInTheDocument();
  });

  it('shows formatted USD for standard fee amounts', () => {
    renderDisplay({ feeUsdEquivalent: 0.05 });
    expect(screen.getByText(/\$0\.05/)).toBeInTheDocument();
  });

  it('handles null balance USD with available fee USD', () => {
    renderDisplay({ usdEquivalent: null, feeUsdEquivalent: 0.05 });
    expect(screen.getByText(WALLET_FEE_DISCLOSURE_COPY.usdUnavailable)).toBeInTheDocument();
    expect(screen.getByText(/\$0\.05/)).toBeInTheDocument();
  });
});
