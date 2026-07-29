/**
 * PRLinkBadge — compact badge + hover/focus preview card for IssueCard.
 *
 * Design spec: design/specs/pr-linking-badge-issuecard.md
 * Issue: #1520
 */

import {
  useRef,
  useState,
  useCallback,
  useEffect,
  useId,
  KeyboardEvent,
} from 'react';
import {
  GitPullRequest,
  GitMerge,
  GitPullRequestClosed,
  GitPullRequestDraft,
  ExternalLink,
} from 'lucide-react';
import { useTheme } from '../../contexts/ThemeContext';
import { LinkedPR } from '../../../features/maintainers/types';

// ─── Types ───────────────────────────────────────────────────────────────────

export interface PRLinkBadgeProps {
  issueId: string;
  linkedPRs?: LinkedPR[];
  linkedPRsLoading?: boolean;
}

type BadgeState = 'unlinked' | 'pr-open' | 'pr-merged' | 'pr-closed' | 'pr-draft' | 'multi-pr';

// ─── Helpers ─────────────────────────────────────────────────────────────────

function deriveBadgeState(prs: LinkedPR[] | undefined): BadgeState {
  if (!prs || prs.length === 0) return 'unlinked';
  if (prs.length > 1) return 'multi-pr';
  switch (prs[0].status) {
    case 'open':   return 'pr-open';
    case 'merged': return 'pr-merged';
    case 'closed': return 'pr-closed';
    case 'draft':  return 'pr-draft';
    default:       return 'pr-open';
  }
}

function getBadgeLabel(state: BadgeState, count: number): string {
  switch (state) {
    case 'pr-open':   return 'PR Open';
    case 'pr-merged': return 'Merged';
    case 'pr-closed': return 'Closed';
    case 'pr-draft':  return 'Draft';
    case 'multi-pr':  return `${count} PRs`;
    default:          return '';
  }
}

function getAriaLabel(state: BadgeState, count: number): string {
  switch (state) {
    case 'pr-open':   return '1 linked pull request — open';
    case 'pr-merged': return '1 linked pull request — merged';
    case 'pr-closed': return '1 linked pull request — closed';
    case 'pr-draft':  return '1 linked pull request — draft';
    case 'multi-pr':  return `${count} linked pull requests`;
    default:          return '';
  }
}

interface BadgeColors {
  bg: string;
  border: string;
  text: string;
}

function getBadgeColors(state: BadgeState, isDark: boolean): BadgeColors {
  switch (state) {
    case 'pr-open':
      return {
        bg:     isDark ? 'bg-[#22c55e]/20' : 'bg-[#22c55e]/15',
        border: 'border-[#22c55e]/30',
        text:   isDark ? 'text-[#22c55e]'  : 'text-[#16a34a]',
      };
    case 'pr-merged':
      return {
        bg:     'bg-[#8b5cf6]/20',
        border: 'border-[#8b5cf6]/30',
        text:   'text-[#8b5cf6]',
      };
    case 'pr-closed':
      return {
        bg:     isDark ? 'bg-[#ef4444]/20' : 'bg-[#ef4444]/15',
        border: 'border-[#ef4444]/30',
        text:   isDark ? 'text-[#ef4444]'  : 'text-[#dc2626]',
      };
    case 'pr-draft':
      return {
        bg:     isDark ? 'bg-[#a8a29e]/20' : 'bg-[#a8a29e]/15',
        border: 'border-[#a8a29e]/30',
        text:   isDark ? 'text-[#a8a29e]'  : 'text-[#78716c]',
      };
    case 'multi-pr':
      return {
        bg:     isDark ? 'bg-[#c9983a]/20' : 'bg-[#c9983a]/15',
        border: 'border-[#c9983a]/30',
        text:   isDark ? 'text-[#c9983a]'  : 'text-[#a67c2e]',
      };
    default:
      return { bg: '', border: '', text: '' };
  }
}

function BadgeIcon({ state, className }: { state: BadgeState; className?: string }) {
  const props = { 'aria-hidden': true as const, className: className ?? 'w-3 h-3' };
  switch (state) {
    case 'pr-merged': return <GitMerge {...props} />;
    case 'pr-closed': return <GitPullRequestClosed {...props} />;
    case 'pr-draft':  return <GitPullRequestDraft {...props} />;
    default:          return <GitPullRequest {...props} />;
  }
}

// ─── Status pill used inside preview card ─────────────────────────────────

function StatusPill({ status, isDark }: { status: LinkedPR['status']; isDark: boolean }) {
  const colors = getBadgeColors(
    status === 'open'   ? 'pr-open'   :
    status === 'merged' ? 'pr-merged' :
    status === 'closed' ? 'pr-closed' : 'pr-draft',
    isDark,
  );
  const label =
    status === 'open'   ? 'Open'   :
    status === 'merged' ? 'Merged' :
    status === 'closed' ? 'Closed' : 'Draft';

  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold border ${colors.bg} ${colors.border} ${colors.text}`}
    >
      {label}
    </span>
  );
}

// ─── Author avatar with initials fallback ─────────────────────────────────

function AuthorAvatar({ name, avatarUrl }: { name: string; avatarUrl?: string }) {
  const [failed, setFailed] = useState(false);
  const initials = name.slice(0, 2).toUpperCase();

  if (!avatarUrl || failed) {
    return (
      <span className="w-5 h-5 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border border-[#c9983a]/40 flex items-center justify-center text-[8px] font-bold text-[#c9983a] flex-shrink-0">
        {initials}
      </span>
    );
  }

  return (
    <img
      src={avatarUrl || `https://github.com/${name}.png?size=20`}
      alt={name}
      className="w-5 h-5 rounded-full border border-[#c9983a]/40 flex-shrink-0"
      onError={() => setFailed(true)}
    />
  );
}

// ─── Loading skeleton ───────────────────────────────────────────────────────

function BadgeSkeleton({ isDark }: { isDark: boolean }) {
  return (
    <span
      aria-label="Loading pull request data"
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-[6px] border animate-pulse ${
        isDark
          ? 'bg-white/10 border-white/15'
          : 'bg-black/5  border-black/10'
      }`}
    >
      <span className={`w-3 h-3 rounded ${isDark ? 'bg-white/20' : 'bg-black/10'}`} />
      <span className={`w-10 h-2.5 rounded ${isDark ? 'bg-white/20' : 'bg-black/10'}`} />
    </span>
  );
}

// ─── Preview card content ──────────────────────────────────────────────────

interface PreviewCardProps {
  id: string;
  prs: LinkedPR[];
  isDark: boolean;
  isVisible: boolean;
  onClose: () => void;
}

function PreviewCard({ id, prs, isDark, isVisible, onClose }: PreviewCardProps) {
  const MAX_LIST = 5;
  const overflow = prs.length > MAX_LIST ? prs.length - MAX_LIST : 0;
  const visible = prs.slice(0, MAX_LIST);

  return (
    <div
      id={id}
      role="tooltip"
      aria-live="polite"
      style={{
        visibility: isVisible ? 'visible' : 'hidden',
        opacity:    isVisible ? 1 : 0,
        transition: 'opacity 120ms ease, visibility 0ms',
      }}
      className={`
        absolute z-50 top-full left-0 mt-2
        min-w-[240px] max-w-[320px]
        rounded-[12px] border p-4
        backdrop-blur-[25px] shadow-lg
        ${isDark
          ? 'bg-white/[0.10] border-white/20 text-[#e8dfd0]'
          : 'bg-white/[0.85] border-white/35 text-[#2d2820]'}
      `}
    >
      {prs.length === 1 ? (
        /* ── Single PR preview ── */
        <SinglePRPreview pr={prs[0]} isDark={isDark} onClose={onClose} />
      ) : (
        /* ── Multi-PR preview ── */
        <div>
          <p className={`text-[12px] font-bold mb-3 ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
            {prs.length} Pull Requests linked
          </p>
          <ul className="space-y-2">
            {visible.map((pr) => (
              <li key={pr.id} className="flex items-center gap-2">
                <GitPullRequest aria-hidden className={`w-3 h-3 flex-shrink-0 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`} />
                <span className={`text-[11px] font-semibold truncate flex-1 ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
                  #{pr.number} {pr.title}
                </span>
                <StatusPill status={pr.status} isDark={isDark} />
                <AuthorAvatar name={pr.author.name} avatarUrl={pr.author.avatar} />
              </li>
            ))}
          </ul>
          {overflow > 0 && (
            <p className={`text-[11px] mt-2 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              + {overflow} more
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function SinglePRPreview({
  pr,
  isDark,
  onClose,
}: {
  pr: LinkedPR;
  isDark: boolean;
  onClose: () => void;
}) {
  return (
    <div>
      {/* Title row */}
      <div className="flex items-start gap-2 mb-3">
        <GitPullRequest aria-hidden className={`w-3.5 h-3.5 mt-0.5 flex-shrink-0 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`} />
        <p className={`text-[13px] font-semibold leading-snug line-clamp-2 ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
          #{pr.number} {pr.title}
        </p>
      </div>

      {/* Author row */}
      <div className="flex items-center gap-1.5 mb-2">
        <AuthorAvatar name={pr.author.name} avatarUrl={pr.author.avatar} />
        <span className={`text-[11px] font-semibold ${isDark ? 'text-[#d4d4d4]' : 'text-[#4a3f2f]'}`}>
          {pr.author.name}
        </span>
      </div>

      {/* Status + time row */}
      <div className="flex items-center gap-2 mb-3">
        <StatusPill status={pr.status} isDark={isDark} />
        <span className={`text-[11px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
          {pr.statusDetail}
        </span>
      </div>

      {/* GitHub link */}
      {pr.url && (
        <a
          href={pr.url}
          target="_blank"
          rel="noopener noreferrer"
          onClick={onClose}
          className={`inline-flex items-center gap-1 text-[11px] font-semibold rounded px-1 py-0.5 transition-colors
            focus:outline-none focus:ring-2 focus:ring-[#f1b400] focus:ring-offset-1
            ${isDark
              ? 'text-[#c9983a] hover:text-[#e8c77f]'
              : 'text-[#a67c2e] hover:text-[#c9983a]'}`}
        >
          Open on GitHub
          <ExternalLink aria-hidden className="w-3 h-3" />
        </a>
      )}
    </div>
  );
}

// ─── Main component ─────────────────────────────────────────────────────────

export function PRLinkBadge({ issueId, linkedPRs, linkedPRsLoading }: PRLinkBadgeProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const [isOpen, setIsOpen] = useState(false);
  const badgeRef  = useRef<HTMLButtonElement>(null);
  const previewRef = useRef<HTMLDivElement>(null);
  const openTimer  = useRef<ReturnType<typeof setTimeout> | null>(null);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Unique id for aria-controls / aria-describedby / id
  const uid = useId();
  const previewId = `pr-preview-${issueId}-${uid.replace(/:/g, '')}`;

  const state = deriveBadgeState(linkedPRs);
  const count = linkedPRs?.length ?? 0;

  // ── Don't render if unlinked and not loading ──
  if (state === 'unlinked' && !linkedPRsLoading) return null;

  const colors = getBadgeColors(state, isDark);

  // ── Open / close helpers ──────────────────────────────────────────────────

  const clearTimers = useCallback(() => {
    if (openTimer.current)  clearTimeout(openTimer.current);
    if (closeTimer.current) clearTimeout(closeTimer.current);
  }, []);

  const scheduleOpen = useCallback(() => {
    clearTimers();
    openTimer.current = setTimeout(() => setIsOpen(true), 150);
  }, [clearTimers]);

  const scheduleClose = useCallback(() => {
    clearTimers();
    closeTimer.current = setTimeout(() => setIsOpen(false), 100);
  }, [clearTimers]);

  const forceClose = useCallback(() => {
    clearTimers();
    setIsOpen(false);
  }, [clearTimers]);

  // ── Keyboard handler for badge ────────────────────────────────────────────

  const handleBadgeKeyDown = useCallback((e: KeyboardEvent<HTMLButtonElement>) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setIsOpen((prev) => !prev);
    }
    if (e.key === 'Escape') {
      forceClose();
      badgeRef.current?.focus();
    }
  }, [forceClose]);

  // ── Keyboard handler for preview ──────────────────────────────────────────

  const handlePreviewKeyDown = useCallback((e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      forceClose();
      badgeRef.current?.focus();
    }
  }, [forceClose]);

  // ── Close on outside click ────────────────────────────────────────────────

  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: MouseEvent) => {
      const target = e.target as Node;
      if (
        badgeRef.current && !badgeRef.current.contains(target) &&
        previewRef.current && !previewRef.current.contains(target)
      ) {
        forceClose();
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [isOpen, forceClose]);

  // ── Loading skeleton ──────────────────────────────────────────────────────

  if (linkedPRsLoading) {
    return <BadgeSkeleton isDark={isDark} />;
  }

  // ── Rendered badge + preview ──────────────────────────────────────────────

  return (
    <span className="relative inline-flex">
      <button
        ref={badgeRef}
        type="button"
        aria-label={getAriaLabel(state, count)}
        aria-expanded={isOpen}
        aria-controls={previewId}
        aria-describedby={previewId}
        onClick={() => setIsOpen((prev) => !prev)}
        onMouseEnter={scheduleOpen}
        onMouseLeave={scheduleClose}
        onFocus={scheduleOpen}
        onBlur={scheduleClose}
        onKeyDown={handleBadgeKeyDown}
        className={`
          inline-flex items-center gap-1 px-2 py-0.5
          rounded-[6px] border text-[10px] font-semibold
          transition-all cursor-pointer
          focus:outline-none focus:ring-2 focus:ring-[#f1b400] focus:ring-offset-1
          ${colors.bg} ${colors.border} ${colors.text}
        `}
      >
        <BadgeIcon state={state} className="w-3 h-3 flex-shrink-0" />
        <span className="hidden sm:inline">{getBadgeLabel(state, count)}</span>
      </button>

      {/* Preview panel — always in DOM when badge present */}
      <div
        ref={previewRef}
        onMouseEnter={clearTimers}
        onMouseLeave={scheduleClose}
        onKeyDown={handlePreviewKeyDown}
      >
        {linkedPRs && linkedPRs.length > 0 && (
          <PreviewCard
            id={previewId}
            prs={linkedPRs}
            isDark={isDark}
            isVisible={isOpen}
            onClose={forceClose}
          />
        )}
      </div>
    </span>
  );
}
