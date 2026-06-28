/**
 * DisputeResolutionPanel — Arbiter interface for reviewing and deciding
 * escrow dispute outcomes (release / refund / split payout).
 *
 * Design contract (issue #1391):
 *  - Dispute detail view: claim, counter-claim, evidence links, timeline
 *  - Split-payout slider (0–100 % to contributor) with live XLM preview
 *  - Irreversibility confirmation screen with glassmorphism warning
 *  - Responsive: 1440 px desktop + 768 px tablet
 *  - WCAG 2.1 AA: keyboard operable, focus management, ARIA live regions
 */
import { useState, useRef, useCallback, type KeyboardEvent } from "react";
import {
  AlertTriangle,
  CheckCircle,
  ChevronRight,
  ExternalLink,
  RefreshCw,
  Scale,
  X,
} from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface EvidenceLink {
  label: string;
  url: string;
}

export interface DisputeEvent {
  timestamp: string;
  actor: string;
  action: string;
}

export interface Dispute {
  id: string;
  programId: string;
  bountyTitle: string;
  totalAmountXlm: number;
  contributor: string;
  maintainer: string;
  claim: string;
  counterClaim: string;
  evidence: EvidenceLink[];
  timeline: DisputeEvent[];
  status: "open" | "resolved";
}

export type Decision = "release" | "refund" | "split";

export interface ArbiterDecision {
  disputeId: string;
  decision: Decision;
  /** 0–100: percentage going to contributor. 100 = full release, 0 = full refund */
  contributorPct: number;
}

interface Props {
  dispute: Dispute;
  /** Called with the final decision; parent is responsible for on-chain submit. */
  onDecide: (decision: ArbiterDecision) => void;
  onClose: () => void;
}

// ─── Sub-components ───────────────────────────────────────────────────────────

/** Single evidence link pill */
function EvidencePill({
  link,
  dark,
}: {
  link: EvidenceLink;
  dark: boolean;
}) {
  return (
    <a
      href={link.url}
      target="_blank"
      rel="noopener noreferrer"
      className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-[13px] border transition-all hover:scale-[1.02] focus:outline-2 focus:outline-offset-2 ${
        dark
          ? "bg-white/5 border-white/10 text-[#c9983a] hover:bg-white/10 focus:outline-[#f1b400]"
          : "bg-black/5 border-black/10 text-[#a2792c] hover:bg-black/10 focus:outline-[#a2792c]"
      }`}
    >
      {link.label}
      <ExternalLink className="w-3 h-3" aria-hidden="true" />
    </a>
  );
}

/** Timeline item row */
function TimelineItem({
  event,
  dark,
  isLast,
}: {
  event: DisputeEvent;
  dark: boolean;
  isLast: boolean;
}) {
  return (
    <li className="flex gap-4">
      <div className="flex flex-col items-center">
        <div
          className={`w-2.5 h-2.5 rounded-full mt-1 flex-shrink-0 ${
            dark ? "bg-[#c9983a]" : "bg-[#a2792c]"
          }`}
          aria-hidden="true"
        />
        {!isLast && (
          <div
            className={`w-px flex-1 mt-1 ${dark ? "bg-white/10" : "bg-black/10"}`}
            aria-hidden="true"
          />
        )}
      </div>
      <div className="pb-4">
        <p className={`text-[12px] ${dark ? "text-[#b8a898]/60" : "text-[#6b5d4d]/60"}`}>
          {event.timestamp}
        </p>
        <p className={`text-[14px] font-medium ${dark ? "text-[#f5efe5]" : "text-[#2d2820]"}`}>
          {event.actor}
        </p>
        <p className={`text-[13px] ${dark ? "text-[#b8a898]/80" : "text-[#6b5d4d]/80"}`}>
          {event.action}
        </p>
      </div>
    </li>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

/**
 * DisputeResolutionPanel
 *
 * Three-step flow:
 *  1. Detail view — read claim / counter-claim / evidence / timeline
 *  2. Decision panel — choose release / refund / split; configure split %
 *  3. Confirmation screen — irreversibility warning before on-chain submit
 */
export function DisputeResolutionPanel({ dispute, onDecide, onClose }: Props) {
  const { theme } = useTheme();
  const dark = theme === "dark";

  type Step = "detail" | "decide" | "confirm";
  const [step, setStep] = useState<Step>("detail");
  const [decision, setDecision] = useState<Decision>("release");
  const [contributorPct, setContributorPct] = useState(100);

  const confirmBtnRef = useRef<HTMLButtonElement>(null);
  const sliderRef = useRef<HTMLInputElement>(null);

  const contributorXlm = ((dispute.totalAmountXlm * contributorPct) / 100).toFixed(2);
  const maintainerXlm = (dispute.totalAmountXlm - parseFloat(contributorXlm)).toFixed(2);

  // Sync decision type with slider position
  const handlePctChange = useCallback((pct: number) => {
    setContributorPct(pct);
    if (pct === 100) setDecision("release");
    else if (pct === 0) setDecision("refund");
    else setDecision("split");
  }, []);

  const handleDecisionButton = (d: Decision) => {
    setDecision(d);
    if (d === "release") setContributorPct(100);
    else if (d === "refund") setContributorPct(0);
    // split: keep current pct or default to 50
    else if (d === "split" && (contributorPct === 0 || contributorPct === 100))
      setContributorPct(50);
  };

  const handleConfirm = () => {
    onDecide({ disputeId: dispute.id, decision, contributorPct });
  };

  // Slider keyboard: arrow keys change by 1, PgUp/PgDn by 10
  const handleSliderKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "PageUp") { e.preventDefault(); handlePctChange(Math.min(100, contributorPct + 10)); }
    if (e.key === "PageDown") { e.preventDefault(); handlePctChange(Math.max(0, contributorPct - 10)); }
  };

  // ── shared style tokens ──────────────────────────────────────────────────
  const surface = dark
    ? "bg-[#2d2820]/95 border-white/15"
    : "bg-white/95 border-black/10";
  const text = dark ? "text-[#f5efe5]" : "text-[#2d2820]";
  const muted = dark ? "text-[#b8a898]/70" : "text-[#6b5d4d]/70";
  const card = dark ? "bg-white/5 border-white/8" : "bg-black/3 border-black/8";
  const gold = dark ? "text-[#f1b400]" : "text-[#a2792c]";
  const goldBg = "bg-[#c9983a]";
  const outline = dark ? "focus:outline-[#f1b400]" : "focus:outline-[#a2792c]";

  // ── STEP 1: Detail view ──────────────────────────────────────────────────
  const DetailView = (
    <div className="flex flex-col gap-6" data-testid="dispute-detail">
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className={`text-[12px] uppercase tracking-wider font-semibold mb-1 ${gold}`}>
            Open Dispute · {dispute.programId}
          </p>
          <h2 id="dispute-title" className={`text-2xl font-bold ${text}`}>
            {dispute.bountyTitle}
          </h2>
          <p className={`text-[13px] mt-1 ${muted}`}>
            Escrow: <span className="font-mono">{dispute.totalAmountXlm} XLM</span>
          </p>
        </div>
        <button
          onClick={onClose}
          aria-label="Close dispute panel"
          className={`w-9 h-9 rounded-full flex items-center justify-center flex-shrink-0 transition hover:scale-105 focus:outline-2 focus:outline-offset-2 ${dark ? "bg-white/5 hover:bg-white/10 text-white/60 focus:outline-[#f1b400]" : "bg-black/5 hover:bg-black/10 text-black/60 focus:outline-[#a2792c]"}`}
          data-testid="close-button"
        >
          <X className="w-4 h-4" aria-hidden="true" />
        </button>
      </div>

      {/* Claim / Counter-claim */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className={`rounded-2xl border p-5 ${card}`} data-testid="claim-panel">
          <p className={`text-[11px] uppercase tracking-wider font-bold mb-2 text-green-400`}>
            Contributor Claim
          </p>
          <p className={`text-[13px] leading-relaxed ${dark ? "text-[#d4c5b0]" : "text-[#4a3d2e]"}`}>
            {dispute.claim}
          </p>
          <p className={`mt-3 text-[12px] font-mono ${muted}`}>{dispute.contributor}</p>
        </div>
        <div className={`rounded-2xl border p-5 ${card}`} data-testid="counter-claim-panel">
          <p className={`text-[11px] uppercase tracking-wider font-bold mb-2 text-red-400`}>
            Maintainer Counter-claim
          </p>
          <p className={`text-[13px] leading-relaxed ${dark ? "text-[#d4c5b0]" : "text-[#4a3d2e]"}`}>
            {dispute.counterClaim}
          </p>
          <p className={`mt-3 text-[12px] font-mono ${muted}`}>{dispute.maintainer}</p>
        </div>
      </div>

      {/* Evidence */}
      {dispute.evidence.length > 0 && (
        <div data-testid="evidence-section">
          <p className={`text-[12px] uppercase tracking-wider font-semibold mb-3 ${muted}`}>
            Attached Evidence
          </p>
          <div className="flex flex-wrap gap-2">
            {dispute.evidence.map((link) => (
              <EvidencePill key={link.url} link={link} dark={dark} />
            ))}
          </div>
        </div>
      )}

      {/* Timeline */}
      <div data-testid="timeline-section">
        <p className={`text-[12px] uppercase tracking-wider font-semibold mb-4 ${muted}`}>
          Timeline
        </p>
        <ul aria-label="Dispute timeline" className="list-none">
          {dispute.timeline.map((event, i) => (
            <TimelineItem
              key={i}
              event={event}
              dark={dark}
              isLast={i === dispute.timeline.length - 1}
            />
          ))}
        </ul>
      </div>

      <button
        onClick={() => setStep("decide")}
        className={`w-full py-3 rounded-2xl font-semibold text-white transition hover:scale-[1.01] focus:outline-2 focus:outline-offset-2 ${goldBg} hover:bg-[#d4a645] ${outline}`}
        data-testid="proceed-to-decide"
      >
        Review as Arbiter
        <ChevronRight className="inline w-4 h-4 ml-1" aria-hidden="true" />
      </button>
    </div>
  );

  // ── STEP 2: Decision panel ───────────────────────────────────────────────
  const DecideView = (
    <div className="flex flex-col gap-6" data-testid="decision-panel">
      <div>
        <button
          onClick={() => setStep("detail")}
          className={`text-[13px] mb-4 hover:underline focus:outline-2 focus:outline-offset-1 ${gold} ${outline}`}
        >
          ← Back to dispute
        </button>
        <h2 className={`text-2xl font-bold ${text}`}>Arbiter Decision</h2>
        <p className={`text-[13px] mt-1 ${muted}`}>
          Choose how to distribute the escrow of{" "}
          <strong>{dispute.totalAmountXlm} XLM</strong>.
        </p>
      </div>

      {/* Decision buttons */}
      <div className="grid grid-cols-3 gap-3" role="group" aria-label="Decision type">
        {(["release", "split", "refund"] as Decision[]).map((d) => {
          const labels: Record<Decision, string> = {
            release: "Full Release",
            split: "Split",
            refund: "Full Refund",
          };
          const icons: Record<Decision, React.ReactNode> = {
            release: <CheckCircle className="w-4 h-4" aria-hidden="true" />,
            split: <Scale className="w-4 h-4" aria-hidden="true" />,
            refund: <RefreshCw className="w-4 h-4" aria-hidden="true" />,
          };
          const active = decision === d;
          return (
            <button
              key={d}
              onClick={() => handleDecisionButton(d)}
              aria-pressed={active}
              className={`flex flex-col items-center gap-2 py-4 px-3 rounded-2xl border transition-all focus:outline-2 focus:outline-offset-2 ${outline} ${
                active
                  ? dark
                    ? "bg-[#c9983a]/20 border-[#c9983a] text-[#f1b400]"
                    : "bg-[#c9983a]/15 border-[#a2792c] text-[#a2792c]"
                  : dark
                  ? "bg-white/5 border-white/10 text-white/60 hover:bg-white/10"
                  : "bg-black/3 border-black/10 text-black/50 hover:bg-black/8"
              }`}
              data-testid={`decision-btn-${d}`}
            >
              {icons[d]}
              <span className="text-[13px] font-semibold">{labels[d]}</span>
            </button>
          );
        })}
      </div>

      {/* Split slider */}
      <div
        className={`rounded-2xl border p-6 ${card} transition-opacity ${decision === "split" || true ? "opacity-100" : "opacity-40"}`}
        data-testid="split-slider-panel"
      >
        <div className="flex items-center justify-between mb-4">
          <p className={`text-[13px] font-semibold ${text}`}>Payout Split</p>
          <span
            aria-live="polite"
            aria-atomic="true"
            className={`text-[13px] font-mono ${gold}`}
            data-testid="split-live-label"
          >
            {contributorPct}% → contributor
          </span>
        </div>

        {/* Slider */}
        <div className="relative mb-4">
          <input
            ref={sliderRef}
            type="range"
            min={0}
            max={100}
            step={1}
            value={contributorPct}
            onChange={(e) => handlePctChange(Number(e.target.value))}
            onKeyDown={handleSliderKey}
            aria-label="Contributor payout percentage"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={contributorPct}
            aria-valuetext={`${contributorPct}% to contributor, ${100 - contributorPct}% to maintainer`}
            className={`w-full h-2 rounded-full appearance-none cursor-pointer focus:outline-2 focus:outline-offset-4 ${outline}`}
            style={{
              background: `linear-gradient(to right, #c9983a ${contributorPct}%, ${dark ? "rgba(255,255,255,0.1)" : "rgba(0,0,0,0.1)"} ${contributorPct}%)`,
            }}
            data-testid="split-slider"
          />
        </div>

        {/* Live preview */}
        <div className="grid grid-cols-2 gap-4" aria-label="Payout preview">
          <div
            className={`rounded-xl p-4 text-center border ${dark ? "bg-[#1a5e2a]/30 border-green-500/20" : "bg-green-50 border-green-200"}`}
            data-testid="contributor-preview"
          >
            <p className={`text-[11px] uppercase tracking-wider mb-1 text-green-400`}>
              Contributor
            </p>
            <p
              className={`text-xl font-bold ${gold}`}
              aria-live="polite"
              aria-atomic="true"
            >
              {contributorXlm} XLM
            </p>
            <p className={`text-[12px] font-mono ${muted}`}>{dispute.contributor}</p>
          </div>
          <div
            className={`rounded-xl p-4 text-center border ${dark ? "bg-[#5e1a1a]/30 border-red-500/20" : "bg-red-50 border-red-200"}`}
            data-testid="maintainer-preview"
          >
            <p className="text-[11px] uppercase tracking-wider mb-1 text-red-400">
              Maintainer
            </p>
            <p
              className={`text-xl font-bold ${dark ? "text-red-400" : "text-red-600"}`}
              aria-live="polite"
              aria-atomic="true"
            >
              {maintainerXlm} XLM
            </p>
            <p className={`text-[12px] font-mono ${muted}`}>{dispute.maintainer}</p>
          </div>
        </div>
      </div>

      <button
        onClick={() => setStep("confirm")}
        className={`w-full py-3 rounded-2xl font-semibold text-white transition hover:scale-[1.01] focus:outline-2 focus:outline-offset-2 ${goldBg} hover:bg-[#d4a645] ${outline}`}
        data-testid="proceed-to-confirm"
      >
        Review Decision
        <ChevronRight className="inline w-4 h-4 ml-1" aria-hidden="true" />
      </button>
    </div>
  );

  // ── STEP 3: Irreversibility confirmation ─────────────────────────────────
  const ConfirmView = (
    <div className="flex flex-col gap-6" data-testid="confirm-screen">
      <button
        onClick={() => setStep("decide")}
        className={`text-[13px] hover:underline focus:outline-2 focus:outline-offset-1 ${gold} ${outline}`}
      >
        ← Change decision
      </button>

      {/* High-emphasis warning */}
      <div
        className={`relative overflow-hidden rounded-2xl border p-6 ${
          dark
            ? "bg-red-950/60 border-red-500/40"
            : "bg-red-50 border-red-300"
        }`}
        style={{ backdropFilter: "blur(20px)" }}
        role="alert"
        aria-live="assertive"
        data-testid="irreversibility-warning"
      >
        <div className="flex items-start gap-4">
          <div className="w-10 h-10 rounded-full bg-red-500/20 flex items-center justify-center flex-shrink-0">
            <AlertTriangle className="w-5 h-5 text-red-400" aria-hidden="true" />
          </div>
          <div>
            <h3 className="font-bold text-red-400 text-[15px] mb-1">
              This action is irreversible
            </h3>
            <p className={`text-[13px] leading-relaxed ${dark ? "text-red-200/80" : "text-red-700/80"}`}>
              Submitting this decision will trigger an on-chain transaction on the
              Stellar network. Once confirmed, funds cannot be recovered or
              redistributed. Verify all evidence before proceeding.
            </p>
          </div>
        </div>
        {/* Glassmorphism overlay shimmer */}
        <div
          className="absolute inset-0 pointer-events-none rounded-2xl"
          style={{
            background:
              "linear-gradient(135deg, rgba(239,68,68,0.06) 0%, transparent 60%)",
          }}
          aria-hidden="true"
        />
      </div>

      {/* Decision summary */}
      <div className={`rounded-2xl border p-5 ${card}`} data-testid="decision-summary">
        <p className={`text-[11px] uppercase tracking-wider font-semibold mb-3 ${muted}`}>
          Your Decision
        </p>
        <div className="flex items-center gap-3 mb-4">
          <Scale className={`w-5 h-5 ${gold}`} aria-hidden="true" />
          <span className={`text-[15px] font-bold ${text}`}>
            {decision === "release"
              ? "Full Release to Contributor"
              : decision === "refund"
              ? "Full Refund to Maintainer"
              : `Split: ${contributorPct}% / ${100 - contributorPct}%`}
          </span>
        </div>
        <div className="grid grid-cols-2 gap-3 text-[13px]">
          <div>
            <p className={muted}>Contributor receives</p>
            <p className={`font-bold text-[15px] ${gold}`}>{contributorXlm} XLM</p>
          </div>
          <div>
            <p className={muted}>Maintainer receives</p>
            <p className={`font-bold text-[15px] ${dark ? "text-[#f5efe5]" : "text-[#2d2820]"}`}>
              {maintainerXlm} XLM
            </p>
          </div>
        </div>
      </div>

      <button
        ref={confirmBtnRef}
        onClick={handleConfirm}
        className={`w-full py-3 rounded-2xl font-semibold text-white bg-red-600 hover:bg-red-700 transition hover:scale-[1.01] focus:outline-2 focus:outline-offset-2 focus:outline-red-400`}
        data-testid="confirm-decision-button"
      >
        Confirm &amp; Submit On-chain
      </button>
    </div>
  );

  return (
    <div
      className="fixed inset-0 z-[300] flex items-start justify-center pt-[5vh] px-4 overflow-y-auto"
      role="presentation"
    >
      {/* Backdrop */}
      <div
        className={`fixed inset-0 ${dark ? "bg-black/75" : "bg-black/45"}`}
        style={{ backdropFilter: "blur(16px)" }}
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Panel */}
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="dispute-title"
        className={`relative w-full max-w-[860px] rounded-[28px] border shadow-2xl p-8 mb-8 ${surface}`}
        style={{ backdropFilter: "blur(80px)" }}
        data-testid="dispute-panel"
      >
        {step === "detail" && DetailView}
        {step === "decide" && DecideView}
        {step === "confirm" && ConfirmView}
      </div>
    </div>
  );
}
