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
