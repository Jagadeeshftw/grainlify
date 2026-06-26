/**
 * @module OGImageTemplate
 * @surface EcosystemDetailPage
 * @description
 *   Renders the Open Graph preview image template for a Grainlify ecosystem card.
 *
 *   Export sizes:
 *     - 1200×630px  (standard OG / Facebook / LinkedIn)
 *     - 800×418px   (Twitter / X summary_large_image card)
 *
 *   The ecosystem card is the most data-rich of the three surfaces, showing
 *   aggregate funding, active project count, and contributor count.
 *   Glassmorphism background with a wider gradient sweep to imply scale.
 *
 * @security
 *   - Ecosystem logo src validated against CDN allowlist.
 *   - All string props truncated before render.
 *   - Currency and numeric inputs are sanitised through `formatStat()`.
 *   - No dangerouslySetInnerHTML.
 *
 * @example
 *   <EcosystemOGImage
 *     ecosystemName="Stellar"
 *     logoSrc="https://cdn.grainlify.io/ecosystems/stellar.svg"
 *     totalFunding="$2.4M"
 *     activeProjects={132}
 *     contributorCount={1840}
 *     tagline="Powering cross-border payments"
 *   />
 */

import React from "react";
import PropTypes from "prop-types";

// ─── Constants ────────────────────────────────────────────────────────────────

const ALLOWED_LOGO_ORIGINS = [
  "https://cdn.grainlify.io",
  "https://assets.grainlify.io",
];

const MAX_ECOSYSTEM_NAME_LEN = 32;
const MAX_TAGLINE_LEN = 72;
const MAX_FUNDING_LEN = 12;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Truncates a string to `max` characters with a trailing ellipsis.
 * @param {string} str
 * @param {number} max
 * @returns {string}
 */
export function truncate(str, max) {
  if (!str) return "";
  return str.length > max ? `${str.slice(0, max - 1)}…` : str;
}

/**
 * Validates an ecosystem logo src against the CDN allowlist.
 * Returns a placeholder path on failure.
 * @param {string|undefined} src
 * @returns {string}
 */
export function validateLogoSrc(src) {
  if (!src) return "/logos/ecosystem-placeholder.svg";
  if (src.startsWith("/") || src.startsWith("./")) return src;
  try {
    const { origin } = new URL(src);
    if (ALLOWED_LOGO_ORIGINS.includes(origin)) return src;
  } catch {
    // Malformed URL
  }
  return "/logos/ecosystem-placeholder.svg";
}

/**
 * Formats a numeric stat for display — coerces to string and truncates.
 * Accepts pre-formatted strings (e.g. "$2.4M") unchanged.
 * @param {string|number} value
 * @param {number} maxLen
 * @returns {string}
 */
export function formatStat(value, maxLen = 12) {
  if (value === null || value === undefined) return "–";
  return truncate(String(value), maxLen);
}

// ─── Component ────────────────────────────────────────────────────────────────

/**
 * EcosystemOGImage — Open Graph template for a Grainlify ecosystem card.
 *
 * @param {object}  props
 * @param {string}  props.ecosystemName    Display name of the ecosystem.
 * @param {string}  [props.logoSrc]        URL/path of the ecosystem logo.
 * @param {string}  props.totalFunding     Aggregate funding stat (e.g. "$2.4M").
 * @param {number}  props.activeProjects   Number of active projects.
 * @param {number}  props.contributorCount Total contributor count.
 * @param {string}  [props.tagline]        Short ecosystem descriptor.
 * @param {"1200"|"800"} [props.width]     Export width variant.
 */
export function EcosystemOGImage({
  ecosystemName,
  logoSrc,
  totalFunding,
  activeProjects,
  contributorCount,
  tagline,
  width = "1200",
}) {
  const isWide = width === "1200";
  const safeName = truncate(ecosystemName, MAX_ECOSYSTEM_NAME_LEN);
  const safeTagline = truncate(tagline, MAX_TAGLINE_LEN);
  const safeFunding = formatStat(totalFunding, MAX_FUNDING_LEN);
  const safeLogoSrc = validateLogoSrc(logoSrc);

  return (
    <div
      className="og-root og-root--ecosystem"
      style={{
        width: isWide ? 1200 : 800,
        height: isWide ? 630 : 418,
      }}
      data-testid="ecosystem-og-image"
      aria-label={`Open Graph preview for ecosystem ${safeName}`}
    >
      {/* ── Background: wider radial sweep for ecosystem scale ── */}
      <div className="og-bg og-bg--ecosystem" aria-hidden="true" />
      <div className="og-grain" aria-hidden="true" />

      {/* ── Header ── */}
      <header className="og-header">
        <span className="og-wordmark">grainlify</span>
        <span className="og-surface-label">Ecosystem</span>
      </header>

      {/* ── Main: logo + name + tagline ── */}
      <main className="og-body og-body--ecosystem">
        {/* Dynamic region: ecosystem logo */}
        <div className="og-eco-logo-wrap" data-testid="ecosystem-logo">
          <img
            src={safeLogoSrc}
            alt={`${safeName} logo`}
            className="og-eco-logo"
            onError={(e) => {
              e.currentTarget.src = "/logos/ecosystem-placeholder.svg";
            }}
          />
        </div>

        {/* Dynamic region: name */}
        <h1
          className={`og-ecosystem-name ${isWide ? "og-ecosystem-name--wide" : "og-ecosystem-name--compact"}`}
          data-testid="ecosystem-name"
        >
          {safeName}
        </h1>

        {/* Dynamic region: tagline */}
        {safeTagline && (
          <p className="og-tagline" data-testid="ecosystem-tagline">
            {safeTagline}
          </p>
        )}

        {/* Dynamic region: three-stat cluster */}
        <div className="og-eco-stats" data-testid="ecosystem-stats">
          <div className="og-eco-stat">
            <span className="og-stat-value og-stat-value--gold">{safeFunding}</span>
            <span className="og-stat-label">total funding</span>
          </div>
          <div className="og-stat-divider" aria-hidden="true" />
          <div className="og-eco-stat">
            <span className="og-stat-value">{activeProjects}</span>
            <span className="og-stat-label">active projects</span>
          </div>
          <div className="og-stat-divider" aria-hidden="true" />
          <div className="og-eco-stat">
            <span className="og-stat-value">{contributorCount.toLocaleString()}</span>
            <span className="og-stat-label">contributors</span>
          </div>
        </div>
      </main>

      {/* ── Footer ── */}
      <footer className="og-footer">
        <div className="og-divider" aria-hidden="true" />
        <div className="og-footer-row">
          <span className="og-footer-tag">Grant execution infrastructure</span>
        </div>
      </footer>

      {/* ── Gold accent line ── */}
      <div className="og-gold-rule" aria-hidden="true" />
    </div>
  );
}

EcosystemOGImage.propTypes = {
  ecosystemName: PropTypes.string.isRequired,
  logoSrc: PropTypes.string,
  totalFunding: PropTypes.string.isRequired,
  activeProjects: PropTypes.number.isRequired,
  contributorCount: PropTypes.number.isRequired,
  tagline: PropTypes.string,
  width: PropTypes.oneOf(["1200", "800"]),
};

export default EcosystemOGImage;