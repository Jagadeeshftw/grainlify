/**
 * @module OGImageTemplate
 * @surface ProfilePage
 * @description
 *   Renders the Open Graph preview image template for a Grainlify contributor
 *   profile card.
 *
 *   Export sizes:
 *     - 1200×630px  (standard OG / Facebook / LinkedIn)
 *     - 800×418px   (Twitter / X summary_large_image card)
 *
 *   Dark glassmorphism aesthetic with Grainlify gold accents.
 *   Avatar image is validated before render; fallback initials avatar is shown
 *   when no image is available or the src fails to load.
 *
 * @security
 *   - Avatar src validated against CDN allowlist (same as ProjectDetailPage).
 *   - All string inputs are truncated to prevent layout overflow.
 *   - No innerHTML usage anywhere in this file.
 *
 * @example
 *   <ContributorOGImage
 *     displayName="Amara Nwosu"
 *     avatarSrc="https://cdn.grainlify.io/avatars/amara.jpg"
 *     role="Protocol Engineer"
 *     totalEarned="$6 200"
 *     prsMerged={47}
 *     ecosystems={["Stellar", "Ethereum"]}
 *   />
 */

import React from "react";
import PropTypes from "prop-types";

// ─── Constants ────────────────────────────────────────────────────────────────

const ALLOWED_AVATAR_ORIGINS = [
  "https://cdn.grainlify.io",
  "https://assets.grainlify.io",
  "https://avatars.githubusercontent.com",
];

const MAX_DISPLAY_NAME_LEN = 36;
const MAX_ROLE_LEN = 48;
const MAX_ECOSYSTEM_LABEL_LEN = 16;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Derives up-to-two initials from a display name for the fallback avatar.
 * @param {string} name
 * @returns {string}
 */
export function initials(name) {
  if (!name) return "?";
  const parts = name.trim().split(/\s+/);
  if (parts.length === 1) return parts[0][0].toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

/**
 * Validates an avatar src against the CDN allowlist or accepts relative paths.
 * Returns null when the src is unsafe (caller renders initials avatar instead).
 * @param {string|undefined} src
 * @returns {string|null}
 */
export function validateAvatarSrc(src) {
  if (!src) return null;
  if (src.startsWith("/") || src.startsWith("./")) return src;
  try {
    const { origin } = new URL(src);
    if (ALLOWED_AVATAR_ORIGINS.includes(origin)) return src;
  } catch {
    // Malformed URL
  }
  return null;
}

/**
 * Truncates a string to `max` characters with an ellipsis when clipped.
 * @param {string} str
 * @param {number} max
 * @returns {string}
 */
export function truncate(str, max) {
  if (!str) return "";
  return str.length > max ? `${str.slice(0, max - 1)}…` : str;
}

// ─── Sub-component: initials fallback avatar ──────────────────────────────────

/**
 * FallbackAvatar renders a styled circle with the contributor's initials.
 * Used when no valid avatar image is available.
 * @param {{ name: string }} props
 */
function FallbackAvatar({ name }) {
  return (
    <div className="og-avatar og-avatar--fallback" aria-hidden="true">
      <span className="og-avatar-initials">{initials(name)}</span>
    </div>
  );
}

FallbackAvatar.propTypes = { name: PropTypes.string.isRequired };

// ─── Component ────────────────────────────────────────────────────────────────

/**
 * ContributorOGImage — Open Graph template for a Grainlify contributor profile.
 *
 * @param {object}   props
 * @param {string}   props.displayName      Contributor's display name.
 * @param {string}   [props.avatarSrc]      URL/path of avatar image.
 * @param {string}   [props.role]           Self-described role or title.
 * @param {string}   props.totalEarned      Lifetime earnings (e.g. "$6 200").
 * @param {number}   props.prsMerged        Total pull requests merged.
 * @param {string[]} [props.ecosystems]     Ecosystems contributed to (max 3 shown).
 * @param {"1200"|"800"} [props.width]      Export width variant.
 */
export function ContributorOGImage({
  displayName,
  avatarSrc,
  role,
  totalEarned,
  prsMerged,
  ecosystems = [],
  width = "1200",
}) {
  const isWide = width === "1200";
  const safeName = truncate(displayName, MAX_DISPLAY_NAME_LEN);
  const safeRole = truncate(role, MAX_ROLE_LEN);
  const safeAvatarSrc = validateAvatarSrc(avatarSrc);
  const visibleEcosystems = ecosystems
    .slice(0, 3)
    .map((e) => truncate(e, MAX_ECOSYSTEM_LABEL_LEN));

  return (
    <div
      className="og-root og-root--profile"
      style={{
        width: isWide ? 1200 : 800,
        height: isWide ? 630 : 418,
      }}
      data-testid="contributor-og-image"
      aria-label={`Open Graph preview for contributor ${safeName}`}
    >
      {/* ── Background gradient ── */}
      <div className="og-bg og-bg--profile" aria-hidden="true" />
      <div className="og-grain" aria-hidden="true" />

      {/* ── Branding header ── */}
      <header className="og-header">
        <span className="og-wordmark">grainlify</span>
        <span className="og-surface-label">Contributor</span>
      </header>

      {/* ── Main content area ── */}
      <main className="og-body og-body--profile">
        {/* Dynamic region: avatar */}
        <div className="og-avatar-wrap" data-testid="contributor-avatar">
          {safeAvatarSrc ? (
            <img
              src={safeAvatarSrc}
              alt={`Avatar of ${safeName}`}
              className="og-avatar"
              onError={(e) => {
                // Swap to initials fallback on load error
                e.currentTarget.replaceWith(
                  Object.assign(document.createElement("div"), {
                    className: "og-avatar og-avatar--fallback",
                    textContent: initials(displayName),
                  })
                );
              }}
            />
          ) : (
            <FallbackAvatar name={displayName} />
          )}
        </div>

        {/* Dynamic region: name and role */}
        <div className="og-identity">
          <h1
            className={`og-contributor-name ${isWide ? "og-contributor-name--wide" : "og-contributor-name--compact"}`}
            data-testid="contributor-name"
          >
            {safeName}
          </h1>
          {safeRole && (
            <p className="og-role" data-testid="contributor-role">
              {safeRole}
            </p>
          )}

          {/* Dynamic region: ecosystem badges */}
          {visibleEcosystems.length > 0 && (
            <ul className="og-ecosystem-list" data-testid="ecosystem-list">
              {visibleEcosystems.map((eco) => (
                <li key={eco} className="og-ecosystem-chip">
                  {eco}
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* Dynamic region: stats */}
        <div className="og-stats-cluster" data-testid="contributor-stats">
          <div className="og-stat-item">
            <span className="og-stat-value">{totalEarned}</span>
            <span className="og-stat-label">total earned</span>
          </div>
          <div className="og-stat-divider" aria-hidden="true" />
          <div className="og-stat-item">
            <span className="og-stat-value">{prsMerged}</span>
            <span className="og-stat-label">PRs merged</span>
          </div>
        </div>
      </main>

      {/* ── Footer ── */}
      <footer className="og-footer">
        <div className="og-divider" aria-hidden="true" />
        <div className="og-footer-row">
          <span className="og-footer-tag">Open-source grant execution</span>
        </div>
      </footer>

      <div className="og-gold-rule" aria-hidden="true" />
    </div>
  );
}

ContributorOGImage.propTypes = {
  displayName: PropTypes.string.isRequired,
  avatarSrc: PropTypes.string,
  role: PropTypes.string,
  totalEarned: PropTypes.string.isRequired,
  prsMerged: PropTypes.number.isRequired,
  ecosystems: PropTypes.arrayOf(PropTypes.string),
  width: PropTypes.oneOf(["1200", "800"]),
};

export default ContributorOGImage;