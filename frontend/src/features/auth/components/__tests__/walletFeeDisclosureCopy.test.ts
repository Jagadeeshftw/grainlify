import { describe, it, expect, vi } from 'vitest';
import {
  WALLET_FEE_DISCLOSURE_COPY,
  formatTimeAgo,
  formatWalletUsd,
  staleBalanceAriaLabel,
} from '../walletFeeDisclosureCopy';

describe('walletFeeDisclosureCopy — strings', () => {
  it('documents fee tooltip mentions Stellar network and payouts', () => {
    expect(WALLET_FEE_DISCLOSURE_COPY.feeTooltip).toMatch(/Stellar network fee/i);
    expect(WALLET_FEE_DISCLOSURE_COPY.feeTooltip).toMatch(/payouts/i);
  });

  it('documents fee-unavailable screen reader hint', () => {
    expect(WALLET_FEE_DISCLOSURE_COPY.feeUnavailableAriaLabel.length).toBeGreaterThan(20);
  });
});

describe('formatWalletUsd', () => {
  it('shows sub-cent values as bounded text', () => {
    expect(formatWalletUsd(0.0000025)).toBe('< $0.01');
  });

  it('formats standard USD amounts', () => {
    expect(formatWalletUsd(312.45)).toBe('$312.45');
  });
});

describe('formatTimeAgo', () => {
  const now = new Date('2026-07-27T12:00:00.000Z').getTime();

  it('reports seconds under one minute', () => {
    const date = new Date(now - 45 * 1000);
    expect(formatTimeAgo(date, now)).toBe('45s ago');
  });

  it('reports minutes under one hour', () => {
    const date = new Date(now - 12 * 60 * 1000);
    expect(formatTimeAgo(date, now)).toBe('12m ago');
  });

  it('reports hours beyond one hour', () => {
    const date = new Date(now - 3 * 60 * 60 * 1000);
    expect(formatTimeAgo(date, now)).toBe('3h ago');
  });
});

describe('staleBalanceAriaLabel', () => {
  it('includes relative last-updated text', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-07-27T12:00:00.000Z'));
    const lastUpdated = new Date('2026-07-27T11:50:00.000Z');
    expect(staleBalanceAriaLabel(lastUpdated)).toBe('Balance last updated 10m ago');
    vi.useRealTimers();
  });
});
