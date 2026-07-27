/**
 * EmptyState Component
 *
 * Cohesive illustration system for all empty surfaces in Grainlify.
 * Uses the grainlify gold/warm-neutral palette with accessible SVG spot illustrations.
 *
 * Layout: illustration (120px) + headline + subtext + optional CTA button, centered.
 * Each illustration has a text-only fallback via <title> and role="img".
 *
 * @see design/specs/empty-state-illustration-system.md
 */

// ─── Illustration variants ────────────────────────────────────────────────────

export type EmptyStateVariant =
  | "no-results-search"
  | "no-bounties"
  | "no-contributions"
  | "no-notifications"
  | "no-leaderboard"
  | "no-programs"
  | "no-payout-history"
  | "no-ecosystems";

// ─── Props ────────────────────────────────────────────────────────────────────

export interface EmptyStateProps {
  variant: EmptyStateVariant;
  headline?: string;
  subtext?: string;
  ctaLabel?: string;
  onCta?: () => void;
  className?: string;
  isDark?: boolean;
}

// ─── Default copy per variant ─────────────────────────────────────────────────

const DEFAULTS: Record<EmptyStateVariant, { headline: string; subtext: string }> = {
  "no-results-search": {
    headline: "No results found",
    subtext: "Try different keywords or remove some filters.",
  },
  "no-bounties": {
    headline: "No bounties yet",
    subtext: "Bounties posted by maintainers will appear here.",
  },
  "no-contributions": {
    headline: "No contributions yet",
    subtext: "Merged pull requests and closed issues will show up here.",
  },
  "no-notifications": {
    headline: "All caught up",
    subtext: "New activity on your projects will appear here.",
  },
  "no-leaderboard": {
    headline: "Leaderboard is empty",
    subtext: "Rankings appear once contributors start earning points.",
  },
  "no-programs": {
    headline: "No programs found",
    subtext: "Grant programs created by ecosystems will be listed here.",
  },
  "no-payout-history": {
    headline: "No payouts yet",
    subtext: "Completed payouts from bounties and programs will appear here.",
  },
  "no-ecosystems": {
    headline: "No ecosystems yet",
    subtext: "Registered ecosystems funding open-source work will show here.",
  },
};

// ─── SVG illustrations ────────────────────────────────────────────────────────
// All illustrations:
//   • viewBox="0 0 120 120", rendered at 120×120px
//   • role="img" with descriptive <title>
//   • Gold (#c9983a) + warm-neutral palette from design-tokens.json
//   • Text-only fallback via aria-label on the wrapping <figure>

interface IllustrationProps {
  isDark: boolean;
}

function NoResultsSearch({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-results-search">
      <title id="es-title-no-results-search">No search results illustration</title>
      {/* Search lens circle */}
      <circle cx="52" cy="52" r="28" fill={fill} stroke={stroke} strokeWidth="3" />
      {/* Inner cross */}
      <line x1="43" y1="43" x2="61" y2="61" stroke={stroke} strokeWidth="3" strokeLinecap="round" />
      <line x1="61" y1="43" x2="43" y2="61" stroke={stroke} strokeWidth="3" strokeLinecap="round" />
      {/* Handle */}
      <line x1="72" y1="72" x2="88" y2="88" stroke={stroke} strokeWidth="4" strokeLinecap="round" />
      {/* Decorative dots */}
      <circle cx="22" cy="30" r="3" fill={neutral} />
      <circle cx="95" cy="38" r="2" fill={neutral} />
      <circle cx="30" cy="88" r="2.5" fill={neutral} />
    </svg>
  );
}

function NoBounties({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-bounties">
      <title id="es-title-no-bounties">No bounties illustration</title>
      {/* Coin / token shape */}
      <circle cx="60" cy="58" r="30" fill={fill} stroke={stroke} strokeWidth="3" />
      <circle cx="60" cy="58" r="22" stroke={stroke} strokeWidth="1.5" strokeDasharray="4 3" />
      {/* Dollar/reward symbol */}
      <text x="60" y="65" textAnchor="middle" fontSize="22" fontWeight="700" fill={stroke} fontFamily="system-ui, sans-serif">
        ✕
      </text>
      {/* Stars around */}
      <circle cx="22" cy="40" r="3" fill={neutral} />
      <circle cx="98" cy="45" r="2" fill={neutral} />
      <circle cx="32" cy="92" r="2.5" fill={neutral} />
      <circle cx="90" cy="86" r="3" fill={neutral} />
    </svg>
  );
}

function NoContributions({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-contributions">
      <title id="es-title-no-contributions">No contributions illustration</title>
      {/* Git branch base */}
      <circle cx="38" cy="82" r="8" fill={fill} stroke={stroke} strokeWidth="2.5" />
      <circle cx="38" cy="38" r="8" fill={fill} stroke={stroke} strokeWidth="2.5" />
      <circle cx="82" cy="56" r="8" fill={fill} stroke={stroke} strokeWidth="2.5" />
      {/* Branch lines */}
      <line x1="38" y1="46" x2="38" y2="74" stroke={stroke} strokeWidth="2.5" strokeLinecap="round" />
      <path d="M38 46 Q38 38 56 38 Q74 38 82 48" stroke={stroke} strokeWidth="2.5" strokeLinecap="round" fill="none" />
      {/* Empty circle on branch tip */}
      <circle cx="82" cy="56" r="4" fill="none" stroke={neutral} strokeWidth="2" strokeDasharray="3 2" />
      {/* Decorative */}
      <circle cx="20" cy="60" r="2" fill={neutral} />
      <circle cx="100" cy="30" r="2.5" fill={neutral} />
    </svg>
  );
}

function NoNotifications({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-notifications">
      <title id="es-title-no-notifications">No notifications illustration</title>
      {/* Bell body */}
      <path d="M60 20 C42 20 30 34 30 52 L30 68 L20 76 L100 76 L90 68 L90 52 C90 34 78 20 60 20Z" fill={fill} stroke={stroke} strokeWidth="3" strokeLinejoin="round" />
      {/* Bell clapper */}
      <path d="M52 76 Q52 86 60 86 Q68 86 68 76" stroke={stroke} strokeWidth="2.5" fill="none" strokeLinecap="round" />
      {/* Zzz — silent indicator */}
      <text x="74" y="40" fontSize="13" fontWeight="700" fill={neutral} fontFamily="system-ui, sans-serif">
        z
      </text>
      <text x="82" y="30" fontSize="10" fontWeight="700" fill={neutral} fontFamily="system-ui, sans-serif">
        z
      </text>
      <text x="89" y="22" fontSize="8" fontWeight="700" fill={neutral} fontFamily="system-ui, sans-serif">
        z
      </text>
    </svg>
  );
}

function NoLeaderboard({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-leaderboard">
      <title id="es-title-no-leaderboard">No leaderboard entries illustration</title>
      {/* Podium bars */}
      <rect x="18" y="64" width="24" height="28" rx="3" fill={fill} stroke={stroke} strokeWidth="2.5" />
      <rect x="48" y="48" width="24" height="44" rx="3" fill={fill} stroke={stroke} strokeWidth="2.5" />
      <rect x="78" y="72" width="24" height="20" rx="3" fill={fill} stroke={stroke} strokeWidth="2.5" />
      {/* Question marks above each bar instead of numbers */}
      <text x="30" y="58" textAnchor="middle" fontSize="13" fontWeight="700" fill={neutral} fontFamily="system-ui, sans-serif">
        ?
      </text>
      <text x="60" y="42" textAnchor="middle" fontSize="13" fontWeight="700" fill={neutral} fontFamily="system-ui, sans-serif">
        ?
      </text>
      <text x="90" y="66" textAnchor="middle" fontSize="13" fontWeight="700" fill={neutral} fontFamily="system-ui, sans-serif">
        ?
      </text>
      {/* Ground line */}
      <line x1="14" y1="92" x2="106" y2="92" stroke={stroke} strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

function NoPrograms({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-programs">
      <title id="es-title-no-programs">No programs illustration</title>
      {/* Document/clipboard body */}
      <rect x="28" y="26" width="64" height="76" rx="6" fill={fill} stroke={stroke} strokeWidth="2.5" />
      {/* Clip at top */}
      <rect x="44" y="20" width="32" height="14" rx="4" fill={fill} stroke={stroke} strokeWidth="2" />
      {/* Empty lines */}
      <line x1="40" y1="52" x2="80" y2="52" stroke={neutral} strokeWidth="2.5" strokeLinecap="round" />
      <line x1="40" y1="64" x2="72" y2="64" stroke={neutral} strokeWidth="2.5" strokeLinecap="round" />
      <line x1="40" y1="76" x2="66" y2="76" stroke={neutral} strokeWidth="2.5" strokeLinecap="round" />
      {/* Dashed placeholder line */}
      <line x1="40" y1="88" x2="60" y2="88" stroke={neutral} strokeWidth="2" strokeLinecap="round" strokeDasharray="3 3" />
    </svg>
  );
}

function NoPayoutHistory({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-payout-history">
      <title id="es-title-no-payout-history">No payout history illustration</title>
      {/* Wallet body */}
      <rect x="20" y="38" width="80" height="56" rx="8" fill={fill} stroke={stroke} strokeWidth="2.5" />
      {/* Wallet flap */}
      <path d="M20 54 L100 54" stroke={stroke} strokeWidth="2" />
      {/* Coin slot */}
      <rect x="72" y="62" width="24" height="22" rx="5" fill={fill} stroke={stroke} strokeWidth="2" />
      <circle cx="84" cy="73" r="5" stroke={neutral} strokeWidth="2" fill="none" strokeDasharray="3 2" />
      {/* Card strip on top */}
      <rect x="28" y="26" width="50" height="18" rx="5" fill={fill} stroke={stroke} strokeWidth="2" />
      {/* Empty lines */}
      <line x1="32" y1="72" x2="62" y2="72" stroke={neutral} strokeWidth="2" strokeLinecap="round" />
      <line x1="32" y1="82" x2="54" y2="82" stroke={neutral} strokeWidth="2" strokeLinecap="round" strokeDasharray="3 2" />
    </svg>
  );
}

function NoEcosystems({ isDark }: IllustrationProps) {
  const stroke = isDark ? "#c9983a" : "#a67c2e";
  const fill = isDark ? "rgba(201,152,58,0.12)" : "rgba(201,152,58,0.10)";
  const neutral = isDark ? "rgba(255,255,255,0.18)" : "rgba(44,36,28,0.14)";
  return (
    <svg width="120" height="120" viewBox="0 0 120 120" fill="none" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="es-title-no-ecosystems">
      <title id="es-title-no-ecosystems">No ecosystems illustration</title>
      {/* Globe outline */}
      <circle cx="60" cy="60" r="34" fill={fill} stroke={stroke} strokeWidth="2.5" />
      {/* Meridian lines */}
      <ellipse cx="60" cy="60" rx="16" ry="34" stroke={stroke} strokeWidth="1.5" strokeDasharray="4 3" fill="none" />
      {/* Latitude lines */}
      <line x1="26" y1="60" x2="94" y2="60" stroke={stroke} strokeWidth="1.5" strokeDasharray="4 3" />
      <line x1="30" y1="44" x2="90" y2="44" stroke={neutral} strokeWidth="1" strokeDasharray="3 3" />
      <line x1="30" y1="76" x2="90" y2="76" stroke={neutral} strokeWidth="1" strokeDasharray="3 3" />
      {/* Disconnected node dots — "empty network" feel */}
      <circle cx="60" cy="26" r="4" fill={fill} stroke={stroke} strokeWidth="2" />
      <circle cx="94" cy="60" r="4" fill={fill} stroke={neutral} strokeWidth="2" strokeDasharray="2 1" />
      <circle cx="26" cy="60" r="4" fill={fill} stroke={neutral} strokeWidth="2" strokeDasharray="2 1" />
    </svg>
  );
}

// ─── Illustration registry ────────────────────────────────────────────────────

const ILLUSTRATIONS: Record<EmptyStateVariant, (props: IllustrationProps) => JSX.Element> = {
  "no-results-search": NoResultsSearch,
  "no-bounties": NoBounties,
  "no-contributions": NoContributions,
  "no-notifications": NoNotifications,
  "no-leaderboard": NoLeaderboard,
  "no-programs": NoPrograms,
  "no-payout-history": NoPayoutHistory,
  "no-ecosystems": NoEcosystems,
};

// ─── Main component ───────────────────────────────────────────────────────────

/**
 * EmptyState
 *
 * Usage:
 *   <EmptyState
 *     variant="no-bounties"
 *     ctaLabel="Browse programs"
 *     onCta={() => navigate('/programs')}
 *   />
 */
export function EmptyState({ variant, headline, subtext, ctaLabel, onCta, className = "", isDark = false }: EmptyStateProps) {
  const defaults = DEFAULTS[variant];
  const resolvedHeadline = headline ?? defaults.headline;
  const resolvedSubtext = subtext ?? defaults.subtext;

  const Illustration = ILLUSTRATIONS[variant];

  return (
    <div
      role="status"
      aria-label={resolvedHeadline}
      className={["flex flex-col items-center justify-center text-center px-6 py-12 w-full", className].filter(Boolean).join(" ")}
    >
      {/* Illustration — 120px fixed, text-only fallback via aria-labelledby on the SVG */}
      <figure aria-label={resolvedHeadline} className="mb-6 w-[120px] h-[120px] flex-shrink-0">
        <Illustration isDark={isDark} />
        {/* Text-only fallback for reduced-data / print — hidden visually */}
        <figcaption className="sr-only">{resolvedHeadline}</figcaption>
      </figure>

      {/* Headline */}
      <h3 className={["text-[18px] font-bold leading-snug mb-2", isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"].join(" ")}>{resolvedHeadline}</h3>

      {/* Subtext */}
      <p className={["text-[14px] leading-relaxed max-w-[280px]", isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"].join(" ")}>{resolvedSubtext}</p>

      {/* CTA button — 44px touch target, gold accent */}
      {ctaLabel && onCta && (
        <button
          onClick={onCta}
          type="button"
          className={[
            // Sizing — min 44px height for touch target spec
            "mt-6 min-h-[44px] px-5 py-2.5 rounded-[12px]",
            "text-[14px] font-semibold leading-none",
            "transition-all duration-150",
            // Colours
            isDark
              ? "bg-[#c9983a] text-white hover:bg-[#e8c77f] hover:text-[#2d2820] active:bg-[#a67c2e]"
              : "bg-[#a67c2e] text-white hover:bg-[#c9983a] active:bg-[#8b6527]",
            // Focus ring — gold, 2px, per dark-mode-spec.md
            "focus-visible:outline-2 focus-visible:outline-offset-2",
            isDark ? "focus-visible:outline-[#f1b400]" : "focus-visible:outline-[#a2792c]",
          ].join(" ")}
        >
          {ctaLabel}
        </button>
      )}
    </div>
  );
}
