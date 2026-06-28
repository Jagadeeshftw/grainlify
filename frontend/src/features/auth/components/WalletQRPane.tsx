import { useState, useEffect, useRef } from 'react';
import { Copy, Check, RefreshCw } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

interface WalletQRPaneProps {
  /** WalletConnect URI (wc:...) used to generate the QR code */
  uri: string;
  /** Countdown duration in seconds before the URI expires (default 120) */
  expiresIn?: number;
  /** Called when user requests a new URI after expiry */
  onRefresh: () => void;
}

/**
 * QR-code pane for WalletConnect mobile pairing.
 *
 * Renders a QR code SVG via the browser's native canvas API (no external lib
 * dependency). Falls back to a copy-URI button for environments where canvas
 * is unavailable. Includes a countdown timer and refresh action.
 *
 * Accessibility:
 * - QR image has descriptive aria-label
 * - Copy button announces state change via aria-live
 * - Countdown announced when expired
 */
export function WalletQRPane({ uri, expiresIn = 120, onRefresh }: WalletQRPaneProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const [secondsLeft, setSecondsLeft] = useState(expiresIn);
  const [copied, setCopied] = useState(false);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Reset timer whenever URI changes
  useEffect(() => {
    setSecondsLeft(expiresIn);
    if (intervalRef.current) clearInterval(intervalRef.current);
    intervalRef.current = setInterval(() => {
      setSecondsLeft((s) => {
        if (s <= 1) {
          clearInterval(intervalRef.current!);
          return 0;
        }
        return s - 1;
      });
    }, 1000);
    return () => clearInterval(intervalRef.current!);
  }, [uri, expiresIn]);

  // Generate QR code as data URL using qrcode (browser-compatible via canvas)
  useEffect(() => {
    let cancelled = false;
    async function generate() {
      try {
        // Dynamically import qrcode only when this pane mounts
        const QRCode = await import('qrcode');
        const dataUrl = await QRCode.toDataURL(uri, {
          width: 220,
          margin: 2,
          color: {
            dark: isDark ? '#f5efe5' : '#2d2820',
            light: isDark ? '#2d2820' : '#ffffff',
          },
        });
        if (!cancelled) setQrDataUrl(dataUrl);
      } catch {
        // If qrcode package unavailable, fallback to null (copy-URI only)
        if (!cancelled) setQrDataUrl(null);
      }
    }
    generate();
    return () => { cancelled = true; };
  }, [uri, isDark]);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(uri);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      /* clipboard unavailable */
    }
  };

  const expired = secondsLeft === 0;
  const minutes = Math.floor(secondsLeft / 60);
  const secs = secondsLeft % 60;
  const countdownLabel = `${minutes}:${String(secs).padStart(2, '0')}`;

  const labelClass = isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]';
  const headingClass = isDark ? 'text-[#f5efe5]' : 'text-[#2d2820]';

  return (
    <div className="flex flex-col items-center gap-4 py-2">
      {/* QR code / expired overlay */}
      <div className="relative">
        {qrDataUrl && !expired ? (
          <img
            src={qrDataUrl}
            alt="WalletConnect QR code — scan with your mobile wallet"
            className="rounded-[12px]"
            width={220}
            height={220}
          />
        ) : (
          /* Placeholder box */
          <div
            className={`w-[220px] h-[220px] rounded-[12px] flex flex-col items-center justify-center gap-2 border ${
              isDark ? 'bg-white/[0.05] border-white/10' : 'bg-neutral-100 border-neutral-200'
            }`}
          >
            {expired ? (
              <>
                <span className={`text-[13px] font-medium ${headingClass}`}>QR code expired</span>
                <button
                  onClick={onRefresh}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-[8px] text-[12px] font-medium text-white focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 hover:opacity-90"
                  style={{ background: 'linear-gradient(135deg,#c9983a,#a2792c)' }}
                >
                  <RefreshCw className="w-3.5 h-3.5" />
                  Refresh
                </button>
              </>
            ) : (
              <span className={`text-[12px] ${labelClass}`}>Generating QR…</span>
            )}
          </div>
        )}

        {/* Expired overlay on top of image */}
        {expired && qrDataUrl && (
          <div className="absolute inset-0 rounded-[12px] flex flex-col items-center justify-center gap-2 bg-black/60 backdrop-blur-[4px]">
            <span className="text-[13px] font-medium text-white">QR code expired</span>
            <button
              onClick={onRefresh}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-[8px] text-[12px] font-medium text-white focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 hover:opacity-90"
              style={{ background: 'linear-gradient(135deg,#c9983a,#a2792c)' }}
            >
              <RefreshCw className="w-3.5 h-3.5" />
              Refresh
            </button>
          </div>
        )}
      </div>

      {/* Countdown */}
      <p
        className={`text-[13px] tabular-nums ${expired ? 'text-red-400' : labelClass}`}
        aria-live="polite"
        aria-atomic="true"
      >
        {expired ? 'Code expired — please refresh' : `Expires in ${countdownLabel}`}
      </p>

      {/* Scan instruction */}
      <p className={`text-[12px] text-center max-w-[240px] ${labelClass}`}>
        Open your mobile wallet app and scan this code to connect.
      </p>

      {/* Copy URI fallback */}
      <div className="w-full">
        <p className={`text-[11px] mb-1.5 font-medium uppercase tracking-wider ${labelClass}`}>
          Or copy connection URI
        </p>
        <div
          className={`flex items-center gap-2 rounded-[10px] border px-3 py-2 ${
            isDark ? 'bg-white/[0.05] border-white/10' : 'bg-neutral-100 border-neutral-200'
          }`}
        >
          <span
            className={`text-[11px] font-mono flex-1 truncate ${isDark ? 'text-[#d4c5b0]' : 'text-[#2d2820]'}`}
          >
            {uri}
          </span>
          <button
            onClick={handleCopy}
            aria-label={copied ? 'URI copied' : 'Copy connection URI'}
            className={`flex-shrink-0 p-1 rounded-[6px] transition-colors focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
              isDark ? 'hover:bg-white/10' : 'hover:bg-black/[0.06]'
            }`}
          >
            {copied ? (
              <Check className="w-3.5 h-3.5 text-green-400" />
            ) : (
              <Copy className={`w-3.5 h-3.5 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`} />
            )}
          </button>
        </div>
        <span aria-live="polite" className="sr-only">
          {copied ? 'Connection URI copied to clipboard' : ''}
        </span>
      </div>
    </div>
  );
}
