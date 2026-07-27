import React, { useEffect, useMemo, useState } from 'react';
import { ArrowLeft, Calendar, MapPin, Clock } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { isDarkVariant } from '../../../shared/contexts/ThemeContext';
import { getOpenSourceWeekEvent } from '../../../shared/api/client';
import {
  SessionTagChip,
  deriveSessionType,
} from '../components/SessionTagChip';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface OSWEvent {
  id: string;
  title: string;
  description: string | null;
  location: string | null;
  status: string;
  start_at: string;
  end_at: string;
}

interface OpenSourceWeekDetailPageProps {
  eventId: string;
  eventName: string;
  onBack: () => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const fmtDate = (iso: string) =>
  new Date(iso).toLocaleDateString(undefined, {
    weekday: 'long',
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  });

const fmtTime = (iso: string) =>
  new Date(iso).toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
  });

function getStatusLabel(status: string): string {
  if (status === 'upcoming') return 'Upcoming';
  if (status === 'running') return 'Live now';
  if (status === 'completed') return 'Ended';
  return 'Draft';
}

function isStartingSoon(startMs: number, nowMs: number): boolean {
  const diffMin = (startMs - nowMs) / 60_000;
  return diffMin > 0 && diffMin <= 30;
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

/** Pulsing gold live indicator */
function LiveBadge({ isDark }: { isDark: boolean }) {
  return (
    <span
      className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-[14px] text-[12px] font-semibold border ${
        isDark
          ? 'bg-[rgba(201,152,58,0.20)] border-[rgba(201,152,58,0.45)] text-[#e8c77f]'
          : 'bg-[rgba(201,152,58,0.15)] border-[rgba(201,152,58,0.40)] text-[#6d5530]'
      }`}
    >
      {/* Pulsing dot — .reduced-motion suppresses animation via theme.css */}
      <span
        aria-hidden="true"
        className={`w-2 h-2 rounded-full osw-live-indicator ${
          isDark ? 'bg-[#f1b400]' : 'bg-[#c9983a]'
        }`}
      />
      Live now
      <span className="sr-only">Live session in progress</span>
    </span>
  );
}

/** Skeleton for the detail hero card */
function DetailSkeleton({ isDark }: { isDark: boolean }) {
  const shimmer = isDark ? 'bg-white/10' : 'bg-black/10';
  return (
    <div
      aria-hidden="true"
      className={`backdrop-blur-[40px] rounded-[20px] border p-8 animate-pulse ${
        isDark ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
      }`}
    >
      <div className={`h-6 w-2/3 rounded mb-4 ${shimmer}`} />
      <div className={`h-4 w-1/2 rounded mb-2 ${shimmer}`} />
      <div className={`h-4 w-3/4 rounded ${shimmer}`} />
    </div>
  );
}

/** Error / not-found card */
function DetailError({ message, isDark }: { message: string; isDark: boolean }) {
  return (
    <div
      role="alert"
      className={`backdrop-blur-[40px] rounded-[20px] border p-8 ${
        isDark
          ? 'bg-white/[0.08] border-white/10 text-[#d4d4d4]'
          : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
      }`}
    >
      <p className="text-[14px]">{message}</p>
    </div>
  );
}

/** Glassmorphism detail card wrapper */
function DetailCard({
  isDark,
  children,
  className = '',
}: {
  isDark: boolean;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`backdrop-blur-[40px] rounded-[20px] border p-6 transition-colors ${
        isDark ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
      } ${className}`}
    >
      {children}
    </div>
  );
}

/** Section label (small uppercase heading) */
function SectionLabel({
  isDark,
  children,
}: {
  isDark: boolean;
  children: React.ReactNode;
}) {
  return (
    <h3
      className={`text-[11px] font-semibold uppercase tracking-wide mb-3 ${
        isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
      }`}
    >
      {children}
    </h3>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function OpenSourceWeekDetailPage({
  eventId,
  eventName,
  onBack,
}: OpenSourceWeekDetailPageProps) {
  const { theme } = useTheme();
  const isDark = isDarkVariant(theme);

  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [event, setEvent] = useState<OSWEvent | null>(null);
  const [nowMs, setNowMs] = useState(() => Date.now());

  // Tick for live/soon badges
  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 30_000);
    return () => clearInterval(id);
  }, []);

  // Fetch
  useEffect(() => {
    let mounted = true;
    setIsLoading(true);
    setError(null);
    getOpenSourceWeekEvent(eventId)
      .then((res) => {
        if (mounted) setEvent(res.event);
      })
      .catch((e) => {
        if (mounted) {
          setEvent(null);
          setError(e instanceof Error ? e.message : 'Failed to load event');
        }
      })
      .finally(() => {
        if (mounted) setIsLoading(false);
      });
    return () => { mounted = false; };
  }, [eventId]);

  const view = useMemo(() => {
    if (!event) return null;
    const start = new Date(event.start_at);
    const end = new Date(event.end_at);
    return {
      title: event.title,
      sessionType: deriveSessionType(event.title),
      location: event.location || 'TBA',
      startDate: fmtDate(event.start_at),
      startTime: fmtTime(event.start_at),
      endDate: fmtDate(event.end_at),
      endTime: fmtTime(event.end_at),
      description:
        event.description ||
        'Details will appear here once the admin configures this event.',
      statusLabel: getStatusLabel(event.status),
      isLive: event.status === 'running',
      isDraft: event.status === 'draft',
      isCompleted: event.status === 'completed',
      isSoon: isStartingSoon(start.getTime(), nowMs),
      startMs: start.getTime(),
    };
  }, [event, nowMs]);

  // Token aliases
  const textPrimary = isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]';
  const textSecondary = isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]';
  const borderSub = isDark ? 'border-white/10' : 'border-black/[0.08]';

  return (
    <div className="space-y-6">
      {/* ── Back button ───────────────────────────────────── */}
      <div className="flex items-center justify-between">
        <button
          onClick={onBack}
          aria-label="Back to Open-Source Week"
          className={`flex items-center gap-2 px-4 py-2 rounded-[12px] backdrop-blur-[30px] border transition-all focus-visible:outline-2 focus-visible:outline-offset-2 ${
            isDark
              ? 'bg-white/[0.08] border-white/10 text-[#f5f5f5] hover:bg-white/[0.12]'
              : 'bg-white/[0.15] border-white/25 text-[#2d2820] hover:bg-white/[0.20]'
          }`}
        >
          <ArrowLeft className="w-4 h-4" aria-hidden="true" />
          <span className="text-[14px] font-medium">Back to Open-Source Week</span>
        </button>
        <div />
      </div>

      {/* ── Loading ───────────────────────────────────────── */}
      {isLoading && <DetailSkeleton isDark={isDark} />}

      {/* ── Error ─────────────────────────────────────────── */}
      {!isLoading && (error || !event) && (
        <DetailError
          message={error || 'Event not found.'}
          isDark={isDark}
        />
      )}

      {/* ── Content ───────────────────────────────────────── */}
      {!isLoading && view && event && (
        <main>
          <div className="grid grid-cols-1 lg:grid-cols-6 gap-6">
            {/* ── Left sidebar ── */}
            <aside className="lg:col-span-2 space-y-4">
              {/* Hero identity card */}
              <DetailCard isDark={isDark}>
                <div className="flex items-start gap-3">
                  <div className="w-12 h-12 rounded-[14px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-lg border border-white/20 shrink-0">
                    <Calendar className="w-6 h-6 text-white" aria-hidden="true" />
                  </div>
                  <div className="min-w-0">
                    <h1
                      className={`text-[17px] font-bold leading-snug mb-2 ${textPrimary} ${
                        view.isDraft ? 'italic' : ''
                      }`}
                    >
                      {view.title}
                    </h1>
                    <div className="flex flex-wrap items-center gap-2">
                      {/* Session type chip */}
                      <SessionTagChip
                        type={view.sessionType}
                        isDark={isDark}
                      />
                      {/* Status treatment */}
                      {view.isLive && <LiveBadge isDark={isDark} />}
                      {view.isSoon && !view.isLive && (
                        <span
                          aria-label={`Starting soon`}
                          className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-[14px] text-[11px] font-semibold border ${
                            isDark
                              ? 'bg-[rgba(241,180,0,0.15)] border-[rgba(241,180,0,0.35)] text-[#fbbf24]'
                              : 'bg-[rgba(241,180,0,0.12)] border-[rgba(180,83,9,0.30)] text-[#b45309]'
                          }`}
                        >
                          <Clock className="w-3 h-3" aria-hidden="true" />
                          Starting soon
                        </span>
                      )}
                      {view.isCompleted && (
                        <span
                          className={`px-2.5 py-1 rounded-[14px] text-[11px] font-semibold border ${
                            isDark
                              ? 'bg-white/10 border-white/20 text-[#d4d4d4]'
                              : 'bg-black/[0.08] border-black/15 text-[#7a6b5a]'
                          }`}
                        >
                          Ended
                        </span>
                      )}
                      {view.isDraft && (
                        <span
                          className={`px-2.5 py-1 rounded-[14px] text-[11px] font-semibold border italic ${
                            isDark
                              ? 'bg-white/10 border-white/20 text-[#d4d4d4]'
                              : 'bg-black/[0.08] border-black/15 text-[#7a6b5a]'
                          }`}
                        >
                          Draft
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              </DetailCard>

              {/* Date card */}
              <DetailCard isDark={isDark}>
                <SectionLabel isDark={isDark}>Date &amp; Time</SectionLabel>
                <div className="space-y-4">
                  {/* Start */}
                  <div>
                    <div className={`text-[11px] mb-0.5 ${textSecondary}`}>Starts</div>
                    <div className={`text-[15px] font-bold ${textPrimary}`}>
                      {view.startDate}
                    </div>
                    <div className={`text-[13px] flex items-center gap-1.5 mt-0.5 ${textSecondary}`}>
                      <Clock className="w-3.5 h-3.5" aria-hidden="true" />
                      {view.startTime}
                    </div>
                  </div>
                  <div className={`h-px ${isDark ? 'bg-white/10' : 'bg-black/[0.08]'}`} />
                  {/* End */}
                  <div>
                    <div className={`text-[11px] mb-0.5 ${textSecondary}`}>Ends</div>
                    <div className={`text-[15px] font-bold ${textPrimary}`}>
                      {view.endDate}
                    </div>
                    <div className={`text-[13px] flex items-center gap-1.5 mt-0.5 ${textSecondary}`}>
                      <Clock className="w-3.5 h-3.5" aria-hidden="true" />
                      {view.endTime}
                    </div>
                  </div>
                </div>
              </DetailCard>

              {/* Location card */}
              <DetailCard isDark={isDark}>
                <SectionLabel isDark={isDark}>Location</SectionLabel>
                <div className={`flex items-start gap-2 ${textPrimary}`}>
                  <MapPin
                    className="w-4 h-4 text-[#c9983a] shrink-0 mt-0.5"
                    aria-hidden="true"
                  />
                  <span className="text-[15px] font-medium">{view.location}</span>
                </div>
              </DetailCard>
            </aside>

            {/* ── Main content ── */}
            <div className="lg:col-span-4 space-y-6">
              {/* Overview / description */}
              <div
                className={`backdrop-blur-[40px] rounded-[20px] border-2 p-8 transition-colors ${
                  isDark
                    ? 'bg-white/[0.05] border-[#c9983a]/40'
                    : 'bg-white/[0.10] border-[#c9983a]/40'
                }`}
              >
                <h2
                  className={`text-[18px] font-bold mb-4 ${
                    isDark ? 'text-[#e8c77f]' : 'text-[#6d5530]'
                  }`}
                >
                  Overview
                </h2>
                <div
                  className={`p-4 rounded-[14px] border ${
                    isDark
                      ? 'bg-white/[0.05] border-white/10'
                      : 'bg-white/[0.10] border-white/25'
                  }`}
                >
                  <p className={`text-[14px] leading-relaxed ${textSecondary}`}>
                    {view.description}
                  </p>
                </div>
              </div>

              {/* Session meta row */}
              <DetailCard isDark={isDark}>
                <SectionLabel isDark={isDark}>Session Details</SectionLabel>
                <dl className={`grid grid-cols-1 sm:grid-cols-3 gap-4 divide-y sm:divide-y-0 sm:divide-x ${borderSub}`}>
                  <div className="sm:pr-4">
                    <dt className={`text-[11px] mb-1 ${textSecondary}`}>Session type</dt>
                    <dd>
                      <SessionTagChip type={view.sessionType} isDark={isDark} />
                    </dd>
                  </div>
                  <div className="pt-4 sm:pt-0 sm:px-4">
                    <dt className={`text-[11px] mb-1 ${textSecondary}`}>Duration</dt>
                    <dd className={`text-[14px] font-semibold ${textPrimary}`}>
                      {(() => {
                        const mins = Math.round(
                          (new Date(event.end_at).getTime() - new Date(event.start_at).getTime()) /
                            60_000
                        );
                        if (mins < 60) return `${mins} min`;
                        const h = Math.floor(mins / 60);
                        const m = mins % 60;
                        return m ? `${h}h ${m}m` : `${h}h`;
                      })()}
                    </dd>
                  </div>
                  <div className="pt-4 sm:pt-0 sm:pl-4">
                    <dt className={`text-[11px] mb-1 ${textSecondary}`}>Status</dt>
                    <dd className={`text-[14px] font-semibold ${textPrimary}`}>
                      {view.statusLabel}
                    </dd>
                  </div>
                </dl>
              </DetailCard>
            </div>
          </div>
        </main>
      )}
    </div>
  );
}
