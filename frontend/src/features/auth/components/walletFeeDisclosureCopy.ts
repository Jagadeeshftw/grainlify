/**
 * User-facing copy for wallet balance and network fee disclosure in connect flows.
 * Kept in one module so tests and design docs can pin exact strings.
 */
export const WALLET_FEE_DISCLOSURE_COPY = {
  feeLabel: 'Est. network fee',
  feeTooltip:
    'Estimated Stellar network fee for signing transactions, including payouts. Based on the current base fee; your wallet shows the exact amount before you confirm.',
  feeUnavailable: 'Fee unavailable',
  feeUnavailableAriaLabel:
    'Network fee estimate unavailable. Your wallet will show the exact fee before you confirm a transaction.',
  usdUnavailable: 'USD equivalent unavailable',
  staleBanner: 'Balance may be outdated. Pull to refresh.',
  insufficientBalanceAlert:
    'Insufficient balance to cover typical network fees. Add XLM before signing transactions or receiving payouts.',
  loadingAriaLabel: 'Loading wallet balance',
  infoButtonAriaLabel: 'More information about the estimated network fee',
} as const;

export function staleBalanceAriaLabel(lastUpdated: Date): string {
  return `Balance last updated ${formatTimeAgo(lastUpdated)}`;
}

/** Relative time for stale indicators and tooltips. */
export function formatTimeAgo(date: Date, nowMs: number = Date.now()): string {
  const seconds = Math.floor((nowMs - date.getTime()) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}

/** USD formatting for balance and fee equivalents shown in the wallet panel. */
export function formatWalletUsd(value: number): string {
  if (value < 0.01) return '< $0.01';
  return `$${value.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}
