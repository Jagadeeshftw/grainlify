import { useState, useRef, useEffect } from 'react';
import {
  FileText,
  Download,
  Loader2,
  ChevronDown,
  AlertCircle,
  Clock,
  CheckCircle2,
} from 'lucide-react';
import { TaxDocument, TaxDocumentStatus, TaxDocumentYearRange } from '../../types';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { TaxDocumentPreviewModal } from './TaxDocumentPreviewModal';

// ---------------------------------------------------------------------------
// Mock data — replace with API call when backend is ready
// ---------------------------------------------------------------------------
const MOCK_DOCUMENTS: TaxDocument[] = [
  {
    year: 2024,
    status: 'available',
    generatedAt: '2025-01-31T00:00:00Z',
    pdfUrl: null, // real apps would supply a pre-signed URL
    totalEarnings: 12450.0,
    stellarAddress: 'GCKFBEIYV2U22IO2BJ4KVJOIP7XPWQGQFKKWXR62YQCZYHUFBR5T5OX',
  },
  {
    year: 2023,
    status: 'available',
    generatedAt: '2024-01-31T00:00:00Z',
    pdfUrl: null,
    totalEarnings: 8200.5,
    stellarAddress: 'GCKFBEIYV2U22IO2BJ4KVJOIP7XPWQGQFKKWXR62YQCZYHUFBR5T5OX',
  },
  {
    year: 2022,
    status: 'pending',
    generatedAt: null,
    pdfUrl: null,
    totalEarnings: null,
    stellarAddress: null,
  },
];

const CURRENT_YEAR = new Date().getFullYear();
const MIN_YEAR = 2021;

// ---------------------------------------------------------------------------
// Status badge
// ---------------------------------------------------------------------------
function StatusBadge({ status, theme }: { status: TaxDocumentStatus; theme: string }) {
  const isDark = theme === 'dark';
  const cfg: Record<TaxDocumentStatus, { icon: React.ReactNode; label: string; classes: string }> = {
    available: {
      icon: <CheckCircle2 className="w-3.5 h-3.5" />,
      label: 'Available',
      classes: isDark
        ? 'bg-green-900/30 text-green-400 border-green-800/40'
        : 'bg-green-50 text-green-700 border-green-200',
    },
    pending: {
      icon: <Clock className="w-3.5 h-3.5" />,
      label: 'Pending',
      classes: isDark
        ? 'bg-yellow-900/30 text-yellow-400 border-yellow-800/40'
        : 'bg-yellow-50 text-yellow-700 border-yellow-200',
    },
    'not-applicable': {
      icon: <AlertCircle className="w-3.5 h-3.5" />,
      label: 'Not Applicable',
      classes: isDark
        ? 'bg-white/[0.05] text-[#b8a898] border-white/10'
        : 'bg-neutral-100 text-neutral-500 border-neutral-200',
    },
  };
  const { icon, label, classes } = cfg[status];
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[12px] font-medium border ${classes}`}
    >
      {icon}
      {label}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Year range selector (dropdown)
// ---------------------------------------------------------------------------
function YearRangeSelector({
  range,
  onChange,
  theme,
}: {
  range: TaxDocumentYearRange;
  onChange: (r: TaxDocumentYearRange) => void;
  theme: string;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const isDark = theme === 'dark';

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  const years = Array.from({ length: CURRENT_YEAR - MIN_YEAR + 1 }, (_, i) => CURRENT_YEAR - i);

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label="Select year range"
        onClick={() => setOpen(!open)}
        className={`flex items-center gap-2 px-4 py-2.5 rounded-[12px] border text-[13px] font-medium backdrop-blur-[30px] transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
          isDark
            ? 'bg-white/[0.08] border-white/15 text-[#f5efe5] hover:bg-white/[0.12]'
            : 'bg-white/[0.15] border-white/25 text-[#2d2820] hover:bg-white/[0.25]'
        }`}
      >
        <span>{range.from === range.to ? String(range.from) : `${range.from} – ${range.to}`}</span>
        <ChevronDown
          className={`w-4 h-4 transition-transform duration-150 ${open ? 'rotate-180' : ''} ${
            isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}
        />
      </button>

      {open && (
        <div
          role="listbox"
          aria-label="Year options"
          className={`absolute right-0 mt-2 w-44 rounded-[16px] border shadow-[0_8px_32px_rgba(0,0,0,0.18)] backdrop-blur-[40px] z-50 overflow-hidden ${
            isDark ? 'bg-[#2d2820]/[0.95] border-white/10' : 'bg-white border-black/[0.08]'
          }`}
        >
          <div
            className={`px-3 py-2 text-[11px] font-medium uppercase tracking-wider border-b ${
              isDark ? 'text-[#b8a898] border-white/10' : 'text-[#9a8b7a] border-black/[0.06]'
            }`}
          >
            From
          </div>
          {years.map((y) => {
            const disabled = y > range.to;
            return (
              <button
                key={`from-${y}`}
                role="option"
                aria-selected={range.from === y}
                disabled={disabled}
                onClick={() => {
                  onChange({ from: y, to: Math.max(y, range.to) });
                  setOpen(false);
                }}
                className={`w-full text-left px-4 py-2 text-[13px] transition-colors focus:outline-none ${
                  range.from === y
                    ? 'text-[#c9983a] font-semibold'
                    : disabled
                      ? isDark
                        ? 'text-[#6b5d4d] cursor-not-allowed'
                        : 'text-[#c9b8a0] cursor-not-allowed'
                      : isDark
                        ? 'text-[#d4c5b0] hover:bg-white/[0.08]'
                        : 'text-[#2d2820] hover:bg-black/[0.04]'
                }`}
              >
                {y}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main TaxDocumentsTab
// ---------------------------------------------------------------------------
export function TaxDocumentsTab() {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const [range, setRange] = useState<TaxDocumentYearRange>({
    from: MIN_YEAR,
    to: CURRENT_YEAR,
  });
  const [previewDoc, setPreviewDoc] = useState<TaxDocument | null>(null);
  const [downloading, setDownloading] = useState<number | null>(null); // year being downloaded

  const filteredDocs = MOCK_DOCUMENTS.filter(
    (d) => d.year >= range.from && d.year <= range.to
  );

  // Simulate download
  const handleDownload = async (doc: TaxDocument) => {
    setDownloading(doc.year);
    setPreviewDoc(null);
    // Simulate async download delay
    await new Promise((r) => setTimeout(r, 1200));
    setDownloading(null);
    // In production: trigger file download via doc.pdfUrl
  };

  const cardClass = `backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] transition-colors ${
    isDark
      ? 'bg-[#2d2820]/[0.4] border-white/10'
      : 'bg-white/[0.12] border-white/20'
  }`;

  const labelClass = isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]';
  const headingClass = isDark ? 'text-[#f5efe5]' : 'text-[#2d2820]';

  return (
    <>
      <div className="space-y-6">
        {/* Section header */}
        <div className={`${cardClass} p-6`}>
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
            <div>
              <h2 className={`text-[18px] font-semibold ${headingClass}`}>Tax Documents</h2>
              <p className={`text-[14px] mt-1 ${labelClass}`}>
                Annual tax summaries (1099-style) for contributors who earned above reporting
                thresholds.
              </p>
            </div>
            <YearRangeSelector range={range} onChange={setRange} theme={theme} />
          </div>
        </div>

        {/* Document list */}
        <div className={`${cardClass} divide-y ${isDark ? 'divide-white/[0.07]' : 'divide-black/[0.05]'}`}>
          {filteredDocs.length === 0 ? (
            /* Empty state */
            <div className="flex flex-col items-center justify-center py-16 px-6 text-center">
              <div
                className={`w-14 h-14 rounded-full flex items-center justify-center mb-4 ${
                  isDark ? 'bg-white/[0.06]' : 'bg-[#c9983a]/[0.1]'
                }`}
              >
                <FileText className={`w-7 h-7 ${isDark ? 'text-[#b8a898]' : 'text-[#c9983a]'}`} />
              </div>
              <h3 className={`text-[16px] font-semibold mb-2 ${headingClass}`}>
                No documents found
              </h3>
              <p className={`text-[14px] max-w-[340px] ${labelClass}`}>
                No tax documents are available for the selected year range. Documents are generated
                for contributors who earn above the reporting threshold ($600 USD).
              </p>
            </div>
          ) : (
            filteredDocs.map((doc) => {
              const isDownloading = downloading === doc.year;
              const canDownload = doc.status === 'available';

              return (
                <div
                  key={doc.year}
                  className="flex flex-col sm:flex-row sm:items-center gap-4 p-5"
                >
                  {/* Icon + year info */}
                  <div className="flex items-center gap-4 flex-1 min-w-0">
                    <div
                      className={`w-10 h-10 rounded-[12px] flex items-center justify-center flex-shrink-0 ${
                        canDownload
                          ? 'bg-[#c9983a]/[0.15]'
                          : isDark
                            ? 'bg-white/[0.06]'
                            : 'bg-neutral-100'
                      }`}
                    >
                      <FileText
                        className={`w-5 h-5 ${
                          canDownload
                            ? 'text-[#c9983a]'
                            : isDark
                              ? 'text-[#b8a898]'
                              : 'text-neutral-400'
                        }`}
                      />
                    </div>
                    <div className="min-w-0">
                      <p className={`text-[15px] font-semibold ${headingClass}`}>
                        Tax Year {doc.year}
                      </p>
                      {doc.generatedAt && (
                        <p className={`text-[12px] mt-0.5 ${labelClass}`}>
                          Generated{' '}
                          {new Date(doc.generatedAt).toLocaleDateString('en-US', {
                            month: 'short',
                            day: 'numeric',
                            year: 'numeric',
                          })}
                        </p>
                      )}
                      {doc.totalEarnings != null && (
                        <p className={`text-[12px] ${labelClass}`}>
                          {new Intl.NumberFormat('en-US', {
                            style: 'currency',
                            currency: 'USD',
                          }).format(doc.totalEarnings)}{' '}
                          total earnings
                        </p>
                      )}
                    </div>
                  </div>

                  {/* Status + actions */}
                  <div className="flex items-center gap-3 flex-shrink-0">
                    <StatusBadge status={doc.status} theme={theme} />

                    {canDownload && (
                      <>
                        {/* Preview button */}
                        <button
                          onClick={() => setPreviewDoc(doc)}
                          aria-label={`Preview tax document for ${doc.year}`}
                          className={`px-4 py-2 rounded-[10px] text-[13px] font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
                            isDark
                              ? 'bg-white/[0.08] text-[#d4c5b0] hover:bg-white/[0.13]'
                              : 'bg-black/[0.05] text-[#6b5d4d] hover:bg-black/[0.09]'
                          }`}
                        >
                          Preview
                        </button>

                        {/* Download button */}
                        <button
                          onClick={() => handleDownload(doc)}
                          disabled={isDownloading}
                          aria-label={
                            isDownloading
                              ? `Downloading tax document for ${doc.year}`
                              : `Download tax document for ${doc.year}`
                          }
                          className="flex items-center gap-1.5 px-4 py-2 rounded-[10px] text-[13px] font-medium text-white transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 hover:opacity-90 active:scale-[0.97] disabled:opacity-60 disabled:cursor-not-allowed"
                          style={{ background: 'linear-gradient(135deg,#c9983a,#a2792c)' }}
                        >
                          {isDownloading ? (
                            <Loader2 className="w-4 h-4 animate-spin" />
                          ) : (
                            <Download className="w-4 h-4" />
                          )}
                          {isDownloading ? 'Downloading…' : 'Download'}
                        </button>
                      </>
                    )}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Below-threshold notice */}
        <div
          className={`rounded-[16px] border p-4 flex gap-3 ${
            isDark
              ? 'bg-white/[0.04] border-white/[0.08] text-[#b8a898]'
              : 'bg-[#fef7e6]/60 border-[#c9983a]/20 text-[#7a6b5a]'
          }`}
        >
          <AlertCircle
            className={`w-4 h-4 mt-0.5 flex-shrink-0 ${isDark ? 'text-[#b8a898]' : 'text-[#c9983a]'}`}
          />
          <p className="text-[13px] leading-relaxed">
            Tax documents are issued to contributors whose annual earnings exceed $600 USD. If you
            do not see a document for a given year, your earnings were below the reporting
            threshold.
          </p>
        </div>
      </div>

      {/* PDF preview modal */}
      {previewDoc && (
        <TaxDocumentPreviewModal
          document={previewDoc}
          onClose={() => setPreviewDoc(null)}
          onDownload={handleDownload}
        />
      )}
    </>
  );
}
