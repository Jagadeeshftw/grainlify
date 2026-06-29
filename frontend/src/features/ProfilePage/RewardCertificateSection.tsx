/**
 * @module RewardCertificateSection
 * @surface ProfilePage
 * @description
 *   Renders the completed program certificate trigger actions and handles
 *   the lifecycle of the responsive certificate preview modal.
 * 
 * @accessibility
 *   - Implements Radix-style focus trapping inside the modal.
 *   - Restores focus to the triggering element upon modal closure.
 *   - Supports ESC key close action.
 *   - High-contrast visual focus states for keyboard users.
 */

import { useState, useEffect, useRef } from "react";
import { Download, Eye, X, Copy, ExternalLink, ShieldCheck, Loader2 } from "lucide-react";
import RewardCertificateTemplate from "./RewardCertificateTemplate";

// ─── Interfaces ──────────────────────────────────────────────────────────────

export interface CompletedProgram {
  id: string;
  name: string;
  amount: string;
  issueDate: string;
  certId: string;
  stellarTxHash: string;
  variant: "gold" | "blue" | "silver";
  sponsorLogoUrl?: string;
  kycVerified: boolean;
  txSuccess: boolean;
}

export interface RewardCertificateSectionProps {
  completedPrograms: CompletedProgram[];
  onDownloadPdf: (programId: string) => Promise<void>;
}

// ─── Component ────────────────────────────────────────────────────────────────

export function RewardCertificateSection({
  completedPrograms = [],
  onDownloadPdf,
}: RewardCertificateSectionProps) {
  const [selectedProgram, setSelectedProgram] = useState<CompletedProgram | null>(null);
  const [isGenerating, setIsGenerating] = useState<string | null>(null); // programId
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  // Refs for accessibility focus management
  const triggerRefs = useRef<{ [key: string]: HTMLButtonElement | null }>({});
  const closeButtonRef = useRef<HTMLButtonElement | null>(null);
  const modalContainerRef = useRef<HTMLDivElement | null>(null);

  // Close modal handler
  const handleCloseModal = () => {
    const closedProgramId = selectedProgram?.id;
    setSelectedProgram(null);
    // Restore focus to the element that triggered the modal
    if (closedProgramId && triggerRefs.current[closedProgramId]) {
      triggerRefs.current[closedProgramId]?.focus();
    }
  };

  // Keyboard navigation inside modal (ESC key & Focus Trap)
  useEffect(() => {
    if (!selectedProgram) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // ESC key closes modal
      if (e.key === "Escape") {
        handleCloseModal();
        return;
      }

      // Focus trapping logic
      if (e.key === "Tab" && modalContainerRef.current) {
        const focusableElements = modalContainerRef.current.querySelectorAll(
          'button, [href], input, select, textarea, [tabindex="0"]'
        );
        const firstElement = focusableElements[0] as HTMLElement;
        const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;

        if (e.shiftKey) {
          // Shift + Tab -> loop to last element
          if (document.activeElement === firstElement) {
            lastElement.focus();
            e.preventDefault();
          }
        } else {
          // Tab -> loop to first element
          if (document.activeElement === lastElement) {
            firstElement.focus();
            e.preventDefault();
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    // Focus close button initially on open
    setTimeout(() => {
      closeButtonRef.current?.focus();
    }, 50);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [selectedProgram]);

  // Handle PDF trigger with loading simulation
  const handleDownload = async (program: CompletedProgram) => {
    if (!program.kycVerified || !program.txSuccess) return;
    setIsGenerating(program.id);
    try {
      await onDownloadPdf(program.id);
      showToast("Certificate downloaded successfully");
    } catch {
      showToast("Download failed. Please try again.");
    } finally {
      setIsGenerating(null);
    }
  };

  // Copy Verification Link
  const handleCopyLink = (certId: string) => {
    const url = `https://grainlify.io/verify/${certId}`;
    navigator.clipboard.writeText(url).then(() => {
      showToast("Verification URL copied to clipboard");
    });
  };

  // Trigger Toast Alert
  const showToast = (message: string) => {
    setToastMessage(message);
    setTimeout(() => setToastMessage(null), 3000);
  };

  return (
    <div className="cert-section-container">
      {/* Toast Alert */}
      {toastMessage && (
        <div
          className="cert-toast"
          role="alert"
          aria-live="assertive"
        >
          <ShieldCheck className="w-5 h-5 text-[#22c55e]" />
          <span>{toastMessage}</span>
        </div>
      )}

      <h2 className="cert-section-heading">Completed Program Certificates</h2>
      
      {completedPrograms.length === 0 ? (
        <div className="cert-empty-state">
          <p>No completed programs available for certificate downloads yet.</p>
        </div>
      ) : (
        <div className="cert-programs-list">
          {completedPrograms.map((program) => {
            const isAvailable = program.kycVerified && program.txSuccess;
            const isCurrentGenerating = isGenerating === program.id;

            return (
              <div
                key={program.id}
                className={`cert-program-row ${!isAvailable ? "cert-program-row--disabled" : ""}`}
              >
                <div className="cert-program-info">
                  <h3 className="cert-row-title">{program.name}</h3>
                  <div className="cert-row-meta">
                    <span>Awarded: <strong>{program.amount}</strong></span>
                    <span className="cert-meta-dot" aria-hidden="true">•</span>
                    <span>Issued: {program.issueDate}</span>
                  </div>
                </div>

                <div className="cert-row-actions">
                  {/* Preview Action Trigger */}
                  <button
                    ref={(el) => (triggerRefs.current[program.id] = el)}
                    onClick={() => isAvailable && setSelectedProgram(program)}
                    disabled={!isAvailable}
                    className="cert-btn cert-btn--icon"
                    aria-label={`Preview Certificate for ${program.name}`}
                    title={isAvailable ? "Preview Certificate" : "Preview Unavailable"}
                  >
                    <Eye className="w-5 h-5" />
                    <span className="sr-only">Preview</span>
                  </button>

                  {/* Download Action Trigger */}
                  <button
                    onClick={() => isAvailable && handleDownload(program)}
                    disabled={!isAvailable || isCurrentGenerating}
                    className="cert-btn cert-btn--primary"
                    aria-label={`Download Certificate for ${program.name}`}
                  >
                    {isCurrentGenerating ? (
                      <>
                        <Loader2 className="w-4 h-4 animate-spin" />
                        <span>Generating...</span>
                      </>
                    ) : (
                      <>
                        <Download className="w-4 h-4" />
                        <span>Download</span>
                      </>
                    )}
                  </button>
                </div>

                {/* Unavailability Tooltip / Status Display */}
                {!isAvailable && (
                  <div className="cert-status-badge">
                    {!program.kycVerified ? (
                      <span className="cert-badge cert-badge--kyc" title="Please complete KYC profile checklist">
                        KYC Pending
                      </span>
                    ) : (
                      <span className="cert-badge cert-badge--tx" title="On-chain ledger hash pending execution">
                        Tx Processing
                      </span>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* ─── Responsive Certificate Preview Modal ─── */}
      {selectedProgram && (
        <div
          className="cert-modal-backdrop"
          onClick={handleCloseModal}
          role="presentation"
        >
          <div
            ref={modalContainerRef}
            className="cert-modal-container"
            onClick={(e) => e.stopPropagation()}
            role="dialog"
            aria-modal="true"
            aria-labelledby="cert-modal-title"
          >
            {/* Modal Header */}
            <header className="cert-modal-header">
              <h3 id="cert-modal-title" className="cert-modal-heading">
                Certificate Preview
              </h3>
              <button
                ref={closeButtonRef}
                onClick={handleCloseModal}
                className="cert-modal-close"
                aria-label="Close Preview Dialog"
              >
                <X className="w-6 h-6" />
              </button>
            </header>

            {/* Modal Content Pane */}
            <div className="cert-modal-body">
              {/* Left Column: Scaled Certificate Preview */}
              <div className="cert-modal-preview-pane">
                <div className="cert-preview-wrapper">
                  <RewardCertificateTemplate
                    displayName={selectedProgram.displayName}
                    programName={selectedProgram.name}
                    amount={selectedProgram.amount}
                    issueDate={selectedProgram.issueDate}
                    certId={selectedProgram.certId}
                    stellarTxHash={selectedProgram.stellarTxHash}
                    variant={selectedProgram.variant}
                    sponsorLogoUrl={selectedProgram.sponsorLogoUrl}
                  />
                </div>
              </div>

              {/* Right Column: Actions Pane */}
              <div className="cert-modal-actions-pane">
                <div className="cert-meta-details">
                  <h4 className="cert-details-heading">{selectedProgram.name}</h4>
                  <p className="cert-details-recipient">Recipient: {selectedProgram.displayName}</p>
                  
                  <div className="cert-details-grid">
                    <div className="cert-grid-item">
                      <span className="cert-grid-label">Credential ID</span>
                      <span className="cert-grid-value cert-code">{selectedProgram.certId}</span>
                    </div>
                    <div className="cert-grid-item">
                      <span className="cert-grid-label">Issue Date</span>
                      <span className="cert-grid-value">{selectedProgram.issueDate}</span>
                    </div>
                  </div>
                </div>

                <div className="cert-action-buttons">
                  <button
                    onClick={() => handleDownload(selectedProgram)}
                    className="cert-modal-btn cert-modal-btn--primary"
                  >
                    <Download className="w-5 h-5" />
                    <span>Download PDF</span>
                  </button>

                  <button
                    onClick={() => handleCopyLink(selectedProgram.certId)}
                    className="cert-modal-btn cert-modal-btn--secondary"
                  >
                    <Copy className="w-5 h-5" />
                    <span>Copy Verification URL</span>
                  </button>

                  <a
                    href={`https://stellar.expert/explorer/public/tx/${selectedProgram.stellarTxHash}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="cert-modal-btn cert-modal-btn--link"
                  >
                    <ExternalLink className="w-5 h-5" />
                    <span>Verify on Stellar Ledger</span>
                  </a>
                </div>
              </div>
            </div>

            {/* Mobile Sticky Footer Action */}
            <footer className="cert-modal-mobile-footer">
              <button
                onClick={() => handleDownload(selectedProgram)}
                className="cert-mobile-sticky-btn"
              >
                <Download className="w-5 h-5" />
                <span>Download PDF</span>
              </button>
            </footer>
          </div>
        </div>
      )}
    </div>
  );
};

export default RewardCertificateSection;
