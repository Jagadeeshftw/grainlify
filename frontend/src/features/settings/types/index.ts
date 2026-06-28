// Tab types
export type SettingsTabType = 'profile' | 'notifications' | 'payout' | 'billing' | 'terms' | 'referrals';

// Billing Profile types
export type BillingProfileStatus = 'verified' | 'missing-verification' | 'limit-reached';
export type BillingProfileType = 'individual' | 'self-employed' | 'organization';
export type ProfileDetailTabType = 'general' | 'payment' | 'invoices';

export interface BillingProfile {
  id: number;
  name: string;
  type: BillingProfileType;
  status: BillingProfileStatus;
  firstName?: string;
  lastName?: string;
  address?: string;
  city?: string;
  postalCode?: string;
  country?: string;
  taxId?: string;
  paymentMethods?: PaymentMethod[];
  invoices?: Invoice[];
}

// Payment Method types
export type CryptoType = 'usdc' | 'usdt' | 'xlm';
export type EcosystemType = 'stellar';

export interface PaymentMethod {
  id: number;
  ecosystem: EcosystemType;
  cryptoType: CryptoType;
  walletAddress: string;
  isDefault: boolean;
  createdAt: string;
}

// Invoice types
export type InvoiceStatus = 'paid' | 'pending' | 'overdue';

export interface Invoice {
  id: string;
  invoiceNumber: string;
  date: string;
  amount: number;
  currency: string;
  status: InvoiceStatus;
  description: string;
  billingPeriod: string;
}

// Payout types
export interface PayoutProject {
  id: number;
  initial: string;
  name: string;
  billingProfile: string | null;
}

// Notification types
export type NotificationChannel = 'inApp' | 'email' | 'push';
export type NotificationEvent = 'payoutReceived' | 'programPublished' | 'bountyClaimed' | 'disputeOpened' | 'systemMaintenance';
export type NotificationPreferenceState = 'enabled' | 'disabled' | 'notAvailable';

export interface NotificationPreference {
  event: NotificationEvent;
  label: string;
  description: string;
  channels: {
    [key in NotificationChannel]: NotificationPreferenceState;
  };
}

export interface NotificationSettings {
  preferences: NotificationPreference[];
}

// Wallet types
export interface TokenWallets {
  usdc: string;
  usdt: string;
  xlm?: string;
}

export interface WalletAddresses {
  stellar: {
    usdc: string;
    usdt: string;
    xlm: string;
  };
}