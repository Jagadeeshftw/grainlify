/**
 * @module ContributorSummarySheet
 * @surface ProfilePage
 * @description
 *   Print-first one-page "contributor summary" document — suitable for
 *   attaching to a resume, portfolio, or grant application.
 *
 *   Layout is optimised for:
 *   - A4 portrait (210mm × 297mm) — CSS default
 *   - US Letter portrait (8.5in × 11in) — opt-in via `paperSize="letter"`
 *
 *   The component is designed to be invisible on-screen (it lives inside a
 *   hidden container and is only exposed to the browser print dialog) OR
 *   to be shown in a dedicated print-preview panel behind a "Print / Save
 *   as PDF" button.
 *
 * @accessibility
 *   - The summary sheet is a semantic HTML document with landmarks
 *     (header, main, footer) so assistive technology can navigate sections.
 *   - Every data section carries an accessible heading level.
 *   - All colours are chosen to remain readable in both colour and
 *     greyscale simulation (see contrast table in the spec).
 *   - The "Print / Save as PDF" trigger button exposes an accessible label
 *     and is reachable via keyboard (tabIndex, focus ring).
 *
 * @printGuidance
 *   - `print-color-adjust: exact` is applied globally inside `@media print`
 *     so that gold accent borders are retained.
 *   - If the user's browser overrides background graphics, all text still
 *     meets 4.5:1 contrast against plain white (#FFFFFF) — gold accents are
 *     supplemented by a bold weight + underline so they are not
 *     colour-only differentiators.
 *   - `break-inside: avoid` is set on every card section so a section is
 *     never split across two pages.
 *
 * @example
 *   <ContributorSummarySheet
 *     displayName="Amara Nwosu"
 *     username="amara-nwosu"
 *     avatarUrl="https://avatars.githubusercontent.com/u/123456"
 *     role="Protocol Engineer"
 *     joinDate="March 2025"
 *     topLanguages={["TypeScript", "Rust", "Go"]}
 *     ecosystems={["Stellar", "Ethereum"]}
 *     totalBountiesWon={12}
 *     totalEarned="$8,400 USD"
 *     prsMerged={47}
 *     issuesResolved={31}
 *     contributionMonths={[3,1,5,8,12,7,4,2,9,6,11,10]}
 *     certificates={[
 *       { name: "Cairo Quests Q1 2026", variant: "gold", certId: "CERT-HK-2026-0628" },
 *     ]}
 *     paperSize="a4"
 *   />
 */

import { useRef } from "react";
import "./reward-certificate-templates.css";

// ─── Types ────────────────────────────────────────────────────────────────────

export type PaperSize = "a4" | "letter";

export interface CertificateSummary {
  name: string;
  variant: "gold" | "blue" | "silver";
  certId: string;
}

export interface ContributorSummarySheetProps {
  displayName: string;
  username: string;
  avatarUrl?: string;
  role?: string;
  joinDate: string;
  topLanguages: string[];
  ecosystems: string[];
  totalBountiesWon: number;
  totalEarned: string;
  prsMerged: number;
  issuesResolved: number;
  /**
   * 12-element array representing contribution intensity per month,
   * index 0 = January. Values 0–15 (heat level).
   */
  contributionMonths: number[];
  certificates: CertificateSummary[];
  paperSize?: PaperSize;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

const MONTH_ABBR = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];

function heatLevel(value: number): string {
  if (value === 0) return "cs-heat-0";
  if (value <= 3)  return "cs-heat-1";
  if (value <= 7)  return "cs-heat-2";
  if (value <= 11) return "cs-heat-3";
  return "cs-heat-4";
}

const VARIANT_LABEL: Record<CertificateSummary["variant"], string> = {
  gold:   "Hackathon",
  blue:   "Scholarship",
  silver: "Bounty",
};

// ─── Sub-components ───────────────────────────────────────────────────────────

function HeatmapThumbnail({ months }: { months: number[] }) {
  return (
    <div className="cs-heatmap" aria-label="Contribution activity heatmap by month">
      {months.map((val, i) => (
        <div key={i} className="cs-heatmap-col">
          <div
            className={`cs-heatmap-cell ${heatLevel(val)}`}
            title={`${MONTH_ABBR[i]}: ${val} contributions`}
            aria-label={`${MONTH_ABBR[i]}: ${val} contributions`}
          />
          <span className="cs-heatmap-label" aria-hidden="true">
            {MONTH_ABBR[i]}
          </span>
        </div>
      ))}
    </div>
  );
}

function LanguageBar({ languages }: { languages: string[] }) {
  if (languages.length === 0) return null;
  return (
    <ul className="cs-lang-list" aria-label="Top programming languages">
      {languages.slice(0, 5).map((lang, i) => (
        <li key={lang} className="cs-lang-chip">
          <span className="cs-lang-rank" aria-hidden="true">#{i + 1}</span>
          {lang}
        </li>
      ))}
    </ul>
  );
}

function StatBlock({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div className="cs-stat-block">
      <span className="cs-stat-value">{value}</span>
      <span className="cs-stat-label">{label}</span>
    </div>
  );
}

// ─── Print trigger ────────────────────────────────────────────────────────────

/**
 * Standalone "Print / Save as PDF" button that triggers `window.print()`.
 * Rendered outside the summary sheet so it does not appear in print output.
 */
export function PrintSummaryButton({
  label = "Print / Save as PDF",
  className = "",
}: {
  label?: string;
  className?: string;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={() => window.print()}
      className={`cs-print-btn no-print ${className}`}
    >
      {/* Printer icon (inline SVG — no extra dependency) */}
      <svg
        aria-hidden="true"
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <polyline points="6 9 6 2 18 2 18 9" />
        <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
        <rect x="6" y="14" width="12" height="8" />
      </svg>
      {label}
    </button>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

export function ContributorSummarySheet({
  displayName,
  username,
  avatarUrl,
  role,
  joinDate,
  topLanguages,
  ecosystems,
  totalBountiesWon,
  totalEarned,
  prsMerged,
  issuesResolved,
  contributionMonths,
  certificates,
  paperSize = "a4",
}: ContributorSummarySheetProps) {
  const sheetRef = useRef<HTMLDivElement>(null);

  // Pad / truncate heatmap to exactly 12 months
  const heatData = Array.from({ length: 12 }, (_, i) => contributionMonths[i] ?? 0);

  return (
    <div
      ref={sheetRef}
      className={`cs-sheet cs-sheet--${paperSize}`}
      data-testid="contributor-summary-sheet"
      aria-label={`Contributor summary for ${displayName}`}
    >
      {/* ── Header ── */}
      <header className="cs-header">
        <div className="cs-identity">
          {avatarUrl ? (
            <img
              src={avatarUrl}
              alt={`${displayName} avatar`}
              className="cs-avatar"
              width={64}
              height={64}
            />
          ) : (
            <div className="cs-avatar cs-avatar--fallback" aria-hidden="true">
              <span className="cs-avatar-initials">
                {displayName.slice(0, 2).toUpperCase()}
              </span>
            </div>
          )}
          <div className="cs-identity-text">
            <h1 className="cs-name" data-testid="cs-name">{displayName}</h1>
            {role && (
              <p className="cs-role" data-testid="cs-role">{role}</p>
            )}
            <p className="cs-meta">
              <span>@{username}</span>
              <span className="cs-meta-sep" aria-hidden="true"> · </span>
              <span>Member since {joinDate}</span>
            </p>
          </div>
        </div>

        <div className="cs-brand" aria-label="Verified by Grainlify">
          <svg
            className="cs-brand-icon"
            viewBox="0 0 32 32"
            fill="none"
            aria-hidden="true"
          >
            <path
              d="M16 2L6 8V24L16 30L26 24V8L16 2Z"
              stroke="currentColor"
              strokeWidth="2.5"
              strokeLinejoin="round"
            />
            <path d="M16 7L23 11V21L16 25L9 21V11L16 7Z" fill="currentColor" opacity="0.3" />
          </svg>
          <span className="cs-brand-wordmark">grainlify</span>
          <span className="cs-brand-tag">Verified Contributor</span>
        </div>
      </header>

      <div className="cs-rule" aria-hidden="true" />

      {/* ── Stats row ── */}
      <section className="cs-section cs-section--stats" aria-label="Contribution statistics">
        <StatBlock label="Bounties Won"       value={totalBountiesWon} />
        <div className="cs-stat-divider" aria-hidden="true" />
        <StatBlock label="Total Earned"        value={totalEarned} />
        <div className="cs-stat-divider" aria-hidden="true" />
        <StatBlock label="PRs Merged"          value={prsMerged} />
        <div className="cs-stat-divider" aria-hidden="true" />
        <StatBlock label="Issues Resolved"     value={issuesResolved} />
      </section>

      <div className="cs-rule" aria-hidden="true" />

      {/* ── Two-column body ── */}
      <main className="cs-body">
        {/* Left column */}
        <div className="cs-col cs-col--left">

          {/* Contribution heatmap */}
          <section className="cs-card" aria-labelledby="cs-heatmap-heading">
            <h2 id="cs-heatmap-heading" className="cs-card-heading">
              Contribution Activity
            </h2>
            <HeatmapThumbnail months={heatData} />
          </section>

          {/* Top languages */}
          {topLanguages.length > 0 && (
            <section className="cs-card" aria-labelledby="cs-lang-heading">
              <h2 id="cs-lang-heading" className="cs-card-heading">
                Top Languages
              </h2>
              <LanguageBar languages={topLanguages} />
            </section>
          )}

          {/* Ecosystems */}
          {ecosystems.length > 0 && (
            <section className="cs-card" aria-labelledby="cs-eco-heading">
              <h2 id="cs-eco-heading" className="cs-card-heading">
                Ecosystems
              </h2>
              <ul className="cs-eco-list" aria-label="Ecosystems contributed to">
                {ecosystems.slice(0, 6).map((eco) => (
                  <li key={eco} className="cs-eco-chip">{eco}</li>
                ))}
              </ul>
            </section>
          )}
        </div>

        {/* Right column — certificates */}
        <div className="cs-col cs-col--right">
          <section className="cs-card cs-card--certs" aria-labelledby="cs-certs-heading">
            <h2 id="cs-certs-heading" className="cs-card-heading">
              Program Certificates
            </h2>
            {certificates.length === 0 ? (
              <p className="cs-empty">No certificates issued yet.</p>
            ) : (
              <ul className="cs-cert-list" aria-label="Issued certificates">
                {certificates.map((cert) => (
                  <li
                    key={cert.certId}
                    className={`cs-cert-item cs-cert-item--${cert.variant}`}
                  >
                    <span
                      className={`cs-cert-badge cs-cert-badge--${cert.variant}`}
                      aria-label={`${VARIANT_LABEL[cert.variant]} certificate`}
                    >
                      {VARIANT_LABEL[cert.variant]}
                    </span>
                    <span className="cs-cert-name">{cert.name}</span>
                    <span className="cs-cert-id" aria-label={`Certificate ID ${cert.certId}`}>
                      {cert.certId}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      </main>

      {/* ── Footer ── */}
      <footer className="cs-footer">
        <div className="cs-rule" aria-hidden="true" />
        <div className="cs-footer-row">
          <span className="cs-footer-text">
            Generated by Grainlify · grainlify.io/verify · Stellar-verified open-source contributions
          </span>
          <span className="cs-footer-date" aria-label="Document generated date">
            {new Date().toLocaleDateString("en-US", {
              month: "long",
              day: "numeric",
              year: "numeric",
            })}
          </span>
        </div>
      </footer>
    </div>
  );
}

export default ContributorSummarySheet;
