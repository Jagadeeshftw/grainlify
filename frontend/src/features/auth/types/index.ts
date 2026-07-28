// Wallet provider identifiers
export type WalletProviderId = 'freighter' | 'albedo' | 'walletconnect' | 'hana';

/**
 * Real-time availability status of a wallet provider.
 * - installed   : extension/app detected and ready (green)
 * - degraded    : detected but reporting an error or outdated (orange)
 * - not-installed: not found in the browser environment (red)
 */
export type WalletProviderStatus = 'installed' | 'degraded' | 'not-installed';

export interface WalletProvider {
  id: WalletProviderId;
  name: string;
  /** Short one-line description shown below the provider name */
  description: string;
  /** Deep-link URL to install the extension/app */
  installUrl: string;
  /** Whether this provider supports a QR-code connection flow (mobile pairing) */
  supportsQR: boolean;
}

export interface WalletProviderWithStatus extends WalletProvider {
  status: WalletProviderStatus;
}
