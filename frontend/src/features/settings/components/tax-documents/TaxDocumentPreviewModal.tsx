import { useEffect, useRef } from 'react';
import { X, Download, FileText } from 'lucide-react';
import { TaxDocument } from '../../types';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

interface TaxDocumentPreviewModalProps {
  document: TaxDocument;
  onClose: () => void;
  onDownload: (doc: TaxDocument) => void;
}

/**
 * Modal for previewing a tax document PDF before download.
 *
 * Accessibility:
 * - role="dialog", aria-modal="true", aria-labelledby
 * - Focus trapped within modal; Escape closes
 * - Backdrop click closes
 */
export function TaxDocumentPreviewModal({
  document,
  onClose,
  onDownload,
}: TaxDocumentPreviewModalProps) {
  const { theme } = useTheme();
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);

  // Save previously focused element and move focus into modal
  useEffect(() => {
    previousFocusRef.current = window.document.activeElement as HTMLElement;
    closeButtonRef.current?.focus();
    window.document.body.style.overflow = 'hidden';
    return () => {
      window.document.body.style.overflow = '';
      previousFocusRef.current?.focus();
    };
  }, []);

  // Focus trap
  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key !== 'Tab') return;
    const focusable = e.currentTarget.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey ? window.document.activeElement === first : window.document.activeElement === last) {
      e.preventDefault();
      (e.shiftKey ? last : first).focus();
    }
  };

  const isDark = theme === 'dark';
  const formattedEarnings =
    document.totalEarnings != null
      ? new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(
          document.totalEarnings
        )
      : '—';

  return (
    /* Backdrop */
    <div
      className="fixed inset-0 z-[10000] flex items-center justify-center p-4"
      style={{ backgroundColor: 'rgba(0,0,0,0.50)' }}
      onClick={onClose}
      aria-hidden="true"
    >
      {/* Dialog */}
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="tax-preview-title"
        className={`relative w-full max-w-[560px] max-h-[90vh] overflow-y-auto rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.24)] backdrop-blur-[40px] transition-colors ${
          isDark
            ? 'bg-[#2d2820]/[0.95] border-white/10 text-[#f5efe5]'
            : 'bg-white/[0.96] border-white/20 text-[#2d2820]'
        }`}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-6 pb-4">
          <h2
            id="tax-preview-title"
            className="text-[18px] font-semibold"
          >
            Tax Document Preview — {document.year}
          </h2>
          <button
            ref={closeButtonRef}
            onClick={onClose}
            aria-label="Close preview"
            className={`p-2 rounded-[10px] transition-colors focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50 ${
              isDark ? 'hover:bg-white/10' : 'hover:bg-black/[0.06]'
            }`}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* PDF preview area */}
        <div className="px-6 pb-4">
          {document.pdfUrl ? (
            <iframe
              src={document.pdfUrl}
              title={`Tax document ${document.year} PDF preview`}
              className="w-full rounded-[14px] border"
              style={{
                height: '340px',
                borderColor: isDark ? 'rgba(255,255,255,0.1)' : 'rgba(0,0,0,0.08)',
              }}
            />
          ) : (
            /* Fallback: branded summary card when no PDF URL */
            <div
              className={`rounded-[14px] border p-6 space-y-4 ${
                isDark ? 'bg-white/[0.05] border-white/10' : 'bg-[#fef7e6]/60 border-[#c9983a]/20'
              }`}
            >
              {/* Branded header */}
              <div className="flex items-center gap-3">
                <div
                  className="w-10 h-10 rounded-full flex items-center justify-center"
                  style={{ background: 'linear-gradient(135deg,#c9983a,#a2792c)' }}
                >
                  <FileText className="w-5 h-5 text-white" />
                </div>
                <div>
                  <p className="text-[15px] font-semibold">Grainlify</p>
                  <p className={`text-[12px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
                    Annual Tax Summary
                  </p>
                </div>
              </div>

              {/* Earnings table */}
              <div className="space-y-2 text-[13px]">
                {[
                  ['Tax Year', String(document.year)],
                  ['Total Earnings', formattedEarnings],
                  ['Stellar Address', document.stellarAddress ?? '—'],
                  [
                    'Generated',
                    document.generatedAt
                      ? new Date(document.generatedAt).toLocaleDateString('en-US', {
                          year: 'numeric',
                          month: 'long',
                          day: 'numeric',
                        })
                      : '—',
                  ],
                ].map(([label, value]) => (
                  <div key={label} className="flex justify-between gap-4">
                    <span className={isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}>{label}</span>
                    <span className="font-medium truncate max-w-[60%] text-right">{value}</span>
                  </div>
                ))}
              </div>

              {/* Disclaimer footer */}
              <p className={`text-[11px] leading-relaxed pt-2 border-t ${
                isDark ? 'text-[#b8a898] border-white/10' : 'text-[#9a8b7a] border-black/[0.08]'
              }`}>
                This document is generated for informational purposes only. Please consult a
                qualified tax professional regarding your reporting obligations.
              </p>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className={`flex gap-3 px-6 py-4 border-t ${isDark ? 'border-white/10' : 'border-black/[0.06]'}`}>
          <button
            onClick={() => onDownload(document)}
            className="flex items-center gap-2 px-5 py-2.5 rounded-[12px] text-[14px] font-medium text-white transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50 hover:opacity-90 active:scale-[0.98]"
            style={{ background: 'linear-gradient(135deg,#c9983a,#a2792c)' }}
          >
            <Download className="w-4 h-4" />
            Download PDF
          </button>
          <button
            onClick={onClose}
            className={`px-5 py-2.5 rounded-[12px] text-[14px] font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-[#c9983a]/50 ${
              isDark
                ? 'bg-white/[0.08] text-[#d4c5b0] hover:bg-white/[0.12]'
                : 'bg-black/[0.05] text-[#6b5d4d] hover:bg-black/[0.09]'
            }`}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
