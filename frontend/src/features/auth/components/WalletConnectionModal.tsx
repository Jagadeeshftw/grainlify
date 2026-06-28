import { useState, useEffect, useRef } from 'react';
import { X, ExternalLink, AlertTriangle, WifiOff, CheckCircle2 } from 'lucide-react';
import {
  WalletProvider,
  WalletProviderWithStatus,
  WalletProviderId,
} from '../types';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { WalletQRPane } from './WalletQRPane';

// ---------------------------------------------------------------------------
// Provider registry
// ---------------------------------------------------------------------------
const PROVIDERS: WalletProvider[] = [
  {
    id: 'freighter',
    name: 'Freighter',
    description: 'Official Stellar browser wallet',
    installUrl: 'https://www.freighter.app',
    supportsQR: false,
  },
  {
    id: 'albedo',
    name: 'Albedo',
    description: 'Web-based Stellar signer',
    installUrl: 'https://albedo.link',
    supportsQR: false,
  },
  {
    id: 'walletconnect',
    name: 'WalletConnect',
    description: 'Scan QR to connect a mobile wallet',
    installUrl: 'https://walletconnect.com/explorer',
    supportsQR: true,
  },
  {
    id: 'hana',
    name: 'Hana',
    description: 'Multi-chain browser extension',
    installUrl: 'https://hanawallet.io',
    supportsQR: false,
  },
];

// ---------------------------------------------------------------------------
// Provider availability detection
// ---------------------------------------------------------------------------
function detectStatus(id: WalletProviderId): 'installed' | 'degraded' | 'not-installed' {
  const win = window as Record<string, unknown>;
  try {
    switch (id) {
      case 'freighter':
        if (win.freighter) return 'installed';
        break;
      case 'albedo':
        if (win.albedo) return 'installed';
        break;
      case 'walletconnect':
        // WalletConnect is always "available" — connection happens via QR
        return 'installed';
      case 'hana':
        if (win.hana) return 'installed';
        break;
    }
  } catch {
    return 'degraded';
  }
  return 'not-installed';
}

// ---------------------------------------------------------------------------
// Status indicator dot + label
// ---------------------------------------------------------------------------
const STATUS_CFG = {
  installed: {
    dot: 'bg-green-400',
    label: 'Installed',
    icon: <CheckCircle2 className="w-3 h-3" />,
    textClass: 'text-green-400',
  },
  degraded: {
    dot: 'bg-orange-400',
    label: 'Degraded',
    icon: <AlertTriangle className="w-3 h-3" />,
    textClass: 'text-orange-400',
  },
  'not-installed': {
    dot: 'bg-red-400',
    label: 'Not installed',
    icon: <WifiOff className="w-3 h-3" />,
    textClass: 'text-red-400',
  },
} as const;

function StatusBadge({ status }: { status: keyof typeof STATUS_CFG }) {
  const cfg = STATUS_CFG[status];
  return (
    <span className={`inline-flex items-center gap-1 text-[11px] font-medium ${cfg.textClass}`}>
      <span className={`w-1.5 h-1.5 rounded-full ${cfg.dot}`} aria-hidden="true" />
      {cfg.label}
    </span>
  );
}

// ---------------------------------------------------------------------------
// WalletConnectionModal
// ---------------------------------------------------------------------------
interface WalletConnectionModalProps {
  onClose: () => void;
  /** Called with provider id when user selects an installed provider */
  onConnect: (providerId: WalletProviderId) => void;
}

/**
 * Multi-provider wallet selection modal.
 *
 * Features:
 * - Provider grid (Freighter, Albedo, WalletConnect, Hana)
 * - Real-time availability indicators (installed / degraded / not-installed)
 * - Install prompt deep links for not-installed providers
 * - QR-code pane for WalletConnect mobile pairing
 *
 * Accessibility:
 * - role="dialog", aria-modal="true", aria-labelledby
 * - Focus trap (Tab / Shift+Tab), Escape closes
 * - Backdrop click closes
 * - Body scroll locked while open
 */
export function WalletConnectionModal({ onClose, onConnect }: WalletConnectionModalProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const [providers, setProviders] = useState<WalletProviderWithStatus[]>([]);
  const [activeQR, setActiveQR] = useState<WalletProviderId | null>(null);
  // Simulated WalletConnect URI — in production this comes from the WC SDK
  const [wcUri, setWcUri] = useState('wc:00e46b69-d0cc-4b3e-b6a2-cee442f97188@1?bridge=https%3A%2F%2Fbridge.walletconnect.org&key=91303dedf64285cfbacea');

  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  // Detect provider statuses on mount
  useEffect(() => {
    const detected = PROVIDERS.map((p) => ({ ...p, status: detectStatus(p.id) }));
    setProviders(detected);
  }, []);

  // Focus management + body scroll lock
  useEffect(() => {
    previousFocusRef.current = document.activeElement as HTMLElement;
    closeButtonRef.current?.focus();
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = '';
      previousFocusRef.current?.focus();
    };
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') { onClose(); return; }
    if (e.key !== 'Tab') return;
    const focusable = e.currentTarget.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey ? document.activeElement === first : document.activeElement === last) {
      e.preventDefault();
      (e.shiftKey ? last : first).focus();
    }
  };

  const handleProviderClick = (p: WalletProviderWithStatus) => {
    if (p.status === 'not-installed') return; // show install prompt instead
    if (p.supportsQR) {
      setActiveQR(activeQR === p.id ? null : p.id);
    } else {
      onConnect(p.id);
    }
  };

  const cardBase = `backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.24)] ${
    isDark ? 'bg-[#2d2820]/[0.95] border-white/10 text-[#f5efe5]' : 'bg-white/[0.98] border-white/20 text-[#2d2820]'
  }`;

  return (
    <div
      className="fixed inset-0 z-[10000] flex items-end sm:items-center justify-center sm:p-4"
      style={{ backgroundColor: 'rgba(0,0,0,0.50)' }}
      onClick={onClose}
      aria-hidden="true"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="wallet-modal-title"
        className={`relative w-full sm:max-w-[480px] max-h-[90vh] overflow-y-auto rounded-t-[28px] sm:rounded-[28px] ${cardBase}`}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 pt-6 pb-4">
          <h2 id="wallet-modal-title" className="text-[18px] font-semibold">
            Connect Wallet
          </h2>
          <button
            ref={closeButtonRef}
            onClick={onClose}
            aria-label="Close wallet selection"
            className={`p-2 rounded-[10px] transition-colors focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50 ${
              isDark ? 'hover:bg-white/10' : 'hover:bg-black/[0.06]'
            }`}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <p className={`px-6 pb-4 text-[13px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
          Select a wallet to connect to Grainlify on the Stellar network.
        </p>

        {/* Provider grid */}
        <div className="px-6 pb-4 grid grid-cols-2 gap-3">
          {providers.map((p) => {
            const notInstalled = p.status === 'not-installed';
            const isQROpen = activeQR === p.id;

            return (
              <div key={p.id} className="flex flex-col gap-2">
                <button
                  onClick={() => handleProviderClick(p)}
                  aria-pressed={isQROpen}
                  aria-label={`${p.name} — ${STATUS_CFG[p.status].label}`}
                  className={`flex flex-col gap-2 p-4 rounded-[16px] border text-left transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
                    isQROpen
                      ? isDark
                        ? 'bg-[#c9983a]/[0.15] border-[#c9983a]/40'
                        : 'bg-[#c9983a]/[0.10] border-[#c9983a]/40'
                      : notInstalled
                        ? isDark
                          ? 'bg-white/[0.04] border-white/[0.07] opacity-70'
                          : 'bg-neutral-50 border-neutral-200 opacity-70'
                        : isDark
                          ? 'bg-white/[0.07] border-white/[0.12] hover:bg-white/[0.12] hover:border-[#c9983a]/30'
                          : 'bg-white/[0.25] border-white/30 hover:bg-white/[0.45] hover:border-[#c9983a]/30'
                  }`}
                >
                  {/* Provider name + status */}
                  <div>
                    <p className="text-[14px] font-semibold">{p.name}</p>
                    <StatusBadge status={p.status} />
                  </div>
                  <p className={`text-[12px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
                    {p.description}
                  </p>
                </button>

                {/* Install prompt (not-installed) */}
                {notInstalled && (
                  <a
                    href={p.installUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-[8px] text-[12px] font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
                      isDark
                        ? 'text-[#c9983a] hover:text-[#d4af37] bg-[#c9983a]/[0.08]'
                        : 'text-[#a2792c] hover:text-[#c9983a] bg-[#c9983a]/[0.08]'
                    }`}
                  >
                    <ExternalLink className="w-3 h-3" />
                    Install {p.name}
                  </a>
                )}
              </div>
            );
          })}
        </div>

        {/* QR pane (WalletConnect) */}
        {activeQR && (
          <div className={`mx-6 mb-6 p-5 rounded-[20px] border ${
            isDark ? 'bg-white/[0.05] border-white/10' : 'bg-neutral-50 border-neutral-200'
          }`}>
            <WalletQRPane
              uri={wcUri}
              expiresIn={120}
              onRefresh={() => {
                // In production: call WalletConnect SDK to get new URI
                setWcUri(`wc:${crypto.randomUUID()}@1?bridge=https%3A%2F%2Fbridge.walletconnect.org&key=${Math.random().toString(36).slice(2)}`);
              }}
            />
          </div>
        )}

        {/* Footer note */}
        <div className={`mx-6 mb-6 rounded-[12px] border p-3 text-[12px] ${
          isDark ? 'bg-white/[0.04] border-white/[0.08] text-[#b8a898]' : 'bg-[#fef7e6]/60 border-[#c9983a]/20 text-[#7a6b5a]'
        }`}>
          Only connect wallets you own. Grainlify never requests your private key or seed phrase.
        </div>
      </div>
    </div>
  );
}
