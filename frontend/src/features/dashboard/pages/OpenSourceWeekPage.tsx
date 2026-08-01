import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Calendar, List, LayoutGrid, MapPin, Clock } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { isDarkVariant } from "../../../shared/contexts/ThemeContext";
import { getOpenSourceWeekEvents } from "../../../shared/api/client";
import { EmptyState } from "../../../shared/components/EmptyState";
import {
  SessionTagChip,
  deriveSessionType,
  type SessionType,
} from "../components/SessionTagChip";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ViewMode = "calendar" | "list";

interface OSWEvent {
  id: string;
  title: string;
  description: string | null;
  location: string | null;
  status: string;
  start_at: string;
  end_at: string;
}

interface FormattedEvent extends OSWEvent {
  sessionType: SessionType;
  startDate: string;
  endDate: string;
  startTime: string;
  endTime: string;
  statusLabel: string;
  startMs: number;
  endMs: number;
  dayKey: string; // "YYYY-MM-DD"
}

interface OpenSourceWeekPageProps {
  onEventClick: (id: string, name: string) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const fmtDate = (iso: string) =>
  new Date(iso).toLocaleDateString(undefined, {
    weekday: "short",
    day: "2-digit",
    month: "short",
  });

const fmtTime = (iso: string) =>
  new Date(iso).toLocaleTimeString(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  });

const fmtDayKey = (iso: string): string => {
  const d = new Date(iso);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
};

const fmtDayLabel = (dayKey: string): string => {
  const [y, m, d] = dayKey.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString(undefined, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
};

function getStatusLabel(status: string): string {
  if (status === "upcoming") return "Upcoming";
  if (status === "running") return "Live now";
  if (status === "completed") return "Ended";
  return "Draft";
}

function isStartingSoon(startMs: number, nowMs: number): boolean {
  const diffMin = (startMs - nowMs) / 60_000;
  return diffMin > 0 && diffMin <= 30;
}

// ---------------------------------------------------------------------------
// Skeleton loader (single card)
// ---------------------------------------------------------------------------

function EventCardSkeleton({ isDark }: { isDark: boolean }) {
  return (
    <div
      aria-hidden="true"
      className={`backdrop-blur-[40px] rounded-[24px] border p-6 sm:p-8 shadow-[0_8px_32px_rgba(0,0,0,0.08)] animate-pulse ${
        isDark
          ? "bg-white/[0.08] border-white/10"
          : "bg-white/[0.15] border-white/25"
      }`}
    >
      <div className="flex flex-col sm:flex-row items-start justify-between gap-4">
        <div className="flex items-start gap-4 w-full">
          <div
            className={`w-12 h-12 rounded-[16px] shrink-0 ${
              isDark ? "bg-white/10" : "bg-black/10"
            }`}
          />
          <div className="space-y-3 flex-1">
            <div
              className={`h-5 w-3/4 rounded ${
                isDark ? "bg-white/10" : "bg-black/10"
              }`}
            />
            <div
              className={`h-6 w-24 rounded-[14px] ${
                isDark ? "bg-white/10" : "bg-black/10"
              }`}
            />
          </div>
        </div>
        <div
          className={`h-10 w-full sm:w-40 rounded-[14px] ${
            isDark ? "bg-white/10" : "bg-black/10"
          }`}
        />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Live indicator dot (pulsing gold ring, respects reduced-motion)
// ---------------------------------------------------------------------------

function LiveDot({ isDark }: { isDark: boolean }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      {/* Pulsing dot — animation disabled in .reduced-motion via theme.css */}
      <span
        aria-hidden="true"
        className={`w-2 h-2 rounded-full osw-live-indicator ${
          isDark ? "bg-[#f1b400]" : "bg-[#c9983a]"
        }`}
      />
      <span className={`text-[11px] font-semibold ${isDark ? "text-[#f1b400]" : "text-[#c9983a]"}`}>
        Live now
      </span>
      {/* Screen-reader label */}
      <span className="sr-only">Live session in progress</span>
    </span>
  );
}

// ---------------------------------------------------------------------------
// "Starting Soon" badge
// ---------------------------------------------------------------------------

function StartingSoonBadge({
  startMs,
  nowMs,
  isDark,
}: {
  startMs: number;
  nowMs: number;
  isDark: boolean;
}) {
  const diffMin = Math.ceil((startMs - nowMs) / 60_000);
  const label = `Starts in ${diffMin} min`;
  return (
    <span
      aria-label={`Starts in ${diffMin} minutes`}
      className={`inline-flex items-center gap-1 px-2.5 py-1 rounded-[14px] text-[11px] font-semibold border ${
        isDark
          ? "bg-[rgba(241,180,0,0.15)] border-[rgba(241,180,0,0.35)] text-[#fbbf24]"
          : "bg-[rgba(241,180,0,0.12)] border-[rgba(180,83,9,0.30)] text-[#b45309]"
      }`}
    >
      <Clock className="w-3 h-3" aria-hidden="true" />
      {label}
    </span>
  );
}

// ---------------------------------------------------------------------------
// List-view event card
// ---------------------------------------------------------------------------

interface EventCardProps {
  event: FormattedEvent;
  isDark: boolean;
  nowMs: number;
  onClick: () => void;
}

function EventCard({ event, isDark, nowMs, onClick }: EventCardProps) {
  const isLive = event.status === "running";
  const isDraft = event.status === "draft";
  const isCompleted = event.status === "completed";
  const soon = isStartingSoon(event.startMs, nowMs);

  const ariaLabel = `${event.startTime} – ${event.endTime}, ${event.title}, ${event.sessionType}`;

  return (
    <li role="listitem">
      <article
        tabIndex={isDraft ? -1 : 0}
        aria-label={ariaLabel}
        onClick={isDraft ? undefined : onClick}
        onKeyDown={(e) => {
          if (!isDraft && (e.key === "Enter" || e.key === " ")) {
            e.preventDefault();
            onClick();
          }
        }}
        className={`backdrop-blur-[40px] rounded-[24px] border p-6 sm:p-8 shadow-[0_8px_32px_rgba(0,0,0,0.08)] transition-all ${
          isDraft
            ? "cursor-default opacity-70"
            : "cursor-pointer focus-visible:outline-2 focus-visible:outline-offset-2"
        } ${
          isCompleted ? "opacity-60" : ""
        } ${
          isLive
            ? isDark
              ? "bg-white/[0.08] border-[rgba(201,152,58,0.45)] shadow-[0_0_0_2px_rgba(201,152,58,0.35),0_8px_32px_rgba(0,0,0,0.08)]"
              : "bg-white/[0.15] border-[rgba(201,152,58,0.50)] shadow-[0_0_0_2px_rgba(201,152,58,0.25),0_8px_32px_rgba(0,0,0,0.08)]"
            : isDark
            ? "bg-white/[0.08] border-white/10 hover:bg-white/[0.12] hover:shadow-[0_8px_24px_rgba(201,152,58,0.15)]"
            : "bg-white/[0.15] border-white/25 hover:bg-white/[0.20] hover:shadow-[0_8px_24px_rgba(0,0,0,0.12)]"
        }`}
      >
        {/* Card header */}
        <div className="flex flex-col sm:flex-row items-start justify-between mb-4 gap-3">
          <div className="flex items-start gap-4 flex-1 min-w-0">
            {/* Icon */}
            <div className="w-12 h-12 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-md border border-white/10 shrink-0">
              <Calendar className="w-6 h-6 text-white" aria-hidden="true" />
            </div>
            {/* Title + status */}
            <div className="flex-1 min-w-0">
              <h3
                className={`text-[17px] sm:text-[20px] font-bold mb-1.5 leading-snug ${
                  isDraft ? "italic" : ""
                } ${isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
              >
                {event.title}
              </h3>
              <div className="flex flex-wrap items-center gap-2">
                <SessionTagChip
                  type={event.sessionType}
                  isDark={isDark}
                />
                {isLive && <LiveDot isDark={isDark} />}
                {soon && !isLive && (
                  <StartingSoonBadge
                    startMs={event.startMs}
                    nowMs={nowMs}
                    isDark={isDark}
                  />
                )}
                {isCompleted && (
                  <span
                    className={`px-2.5 py-1 rounded-[14px] text-[11px] font-semibold border ${
                      isDark
                        ? "bg-white/10 border-white/20 text-[#d4d4d4]"
                        : "bg-black/[0.08] border-black/15 text-[#7a6b5a]"
                    }`}
                  >
                    Ended
                  </span>
                )}
                {isDraft && (
                  <span
                    className={`px-2.5 py-1 rounded-[14px] text-[11px] font-semibold border italic ${
                      isDark
                        ? "bg-white/10 border-white/20 text-[#d4d4d4]"
                        : "bg-black/[0.08] border-black/15 text-[#7a6b5a]"
                    }`}
                  >
                    Draft
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* View details CTA (hidden for draft) */}
          {!isDraft && (
            <button
              tabIndex={-1}
              aria-hidden="true"
              onClick={(e) => {
                e.stopPropagation();
                onClick();
              }}
              className="shrink-0 px-5 py-2.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[14px] font-semibold text-[13px] shadow-[0_4px_16px_rgba(162,121,44,0.3)] hover:shadow-[0_6px_20px_rgba(162,121,44,0.4)] transition-all border border-white/10 whitespace-nowrap"
            >
              View Details
            </button>
          )}
        </div>

        {/* Meta row */}
        <div
          className={`flex flex-wrap items-center gap-x-5 gap-y-1.5 pt-4 border-t ${
            isDark ? "border-white/10" : "border-black/[0.08]"
          }`}
        >
          {/* Date + time */}
          <div className={`flex items-center gap-1.5 text-[13px] ${isDark ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}>
            <Calendar className="w-3.5 h-3.5 shrink-0" aria-hidden="true" />
            <span>
              {event.startDate} · {event.startTime} – {event.endTime}
            </span>
          </div>

          {/* Location */}
          {event.location && (
            <div className={`flex items-center gap-1.5 text-[13px] ${isDark ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}>
              <MapPin className="w-3.5 h-3.5 shrink-0 text-[#c9983a]" aria-hidden="true" />
              <span>{event.location}</span>
            </div>
          )}
        </div>
      </article>
    </li>
  );
}

// ---------------------------------------------------------------------------
// Calendar Grid — session block
// ---------------------------------------------------------------------------

interface SessionBlockProps {
  event: FormattedEvent;
  isDark: boolean;
  nowMs: number;
  onClick: () => void;
}

function SessionBlock({ event, isDark, nowMs, onClick }: SessionBlockProps) {
  const isLive = event.status === "running";
  const isDraft = event.status === "draft";
  const isCompleted = event.status === "completed";
  const soon = isStartingSoon(event.startMs, nowMs);

  // Border-left accent color per session type
  const accentColor: Record<SessionType, string> = {
    workshop: "#c9983a",
    panel: isDark ? "#93c5fd" : "#2563eb",
    "office-hours": isDark ? "#86efac" : "#16a34a",
  };
  const accent = accentColor[event.sessionType];

  const ariaLabel = `${event.startTime} – ${event.endTime}, ${event.title}, ${event.sessionType}`;

  return (
    <button
      aria-label={ariaLabel}
      aria-disabled={isDraft}
      tabIndex={isDraft ? -1 : 0}
      onClick={isDraft ? undefined : onClick}
      className={`w-full text-left rounded-[14px] p-2.5 border transition-all focus-visible:outline-2 focus-visible:outline-offset-2 ${
        isDraft ? "cursor-default opacity-60 pointer-events-none" : "cursor-pointer"
      } ${isCompleted ? "opacity-60" : ""} ${
        isLive
          ? isDark
            ? "bg-[rgba(201,152,58,0.15)] border-[rgba(201,152,58,0.45)]"
            : "bg-[rgba(201,152,58,0.10)] border-[rgba(201,152,58,0.50)]"
          : isDark
          ? "bg-white/[0.06] border-white/10 hover:bg-white/[0.12]"
          : "bg-white/[0.20] border-white/30 hover:bg-white/[0.30]"
      }`}
      style={{ borderLeftWidth: "3px", borderLeftColor: accent }}
    >
      {/* Tag chip */}
      <SessionTagChip type={event.sessionType} isDark={isDark} compact />

      {/* Title */}
      <p
        className={`mt-1 text-[12px] font-semibold leading-tight line-clamp-2 ${
          isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"
        }`}
      >
        {event.title}
      </p>

      {/* Time + location */}
      <p
        className={`mt-1 text-[10px] leading-tight ${
          isDark ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
        }`}
      >
        {event.startTime}–{event.endTime}
        {event.location ? ` · ${event.location}` : ""}
      </p>

      {/* Live badge */}
      {isLive && <LiveDot isDark={isDark} />}
      {soon && !isLive && (
        <StartingSoonBadge startMs={event.startMs} nowMs={nowMs} isDark={isDark} />
      )}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Calendar Grid view
// ---------------------------------------------------------------------------

interface CalendarGridProps {
  events: FormattedEvent[];
  isDark: boolean;
  nowMs: number;
  onEventClick: (id: string, name: string) => void;
}

function CalendarGrid({ events, isDark, nowMs, onEventClick }: CalendarGridProps) {
  // Collect all unique day keys, sorted
  const dayKeys = useMemo(() => {
    const keys = new Set(events.map((e) => e.dayKey));
    return Array.from(keys).sort();
  }, [events]);

  // Determine time slot range (whole hours)
  const { minHour, maxHour } = useMemo(() => {
    let min = 9;
    let max = 18;
    for (const e of events) {
      const sh = new Date(e.start_at).getHours();
      const eh = new Date(e.end_at).getHours() + (new Date(e.end_at).getMinutes() > 0 ? 1 : 0);
      if (sh < min) min = sh;
      if (eh > max) max = eh;
    }
    return { minHour: Math.max(6, min), maxHour: Math.min(23, max) };
  }, [events]);

  const hours = useMemo(
    () => Array.from({ length: maxHour - minHour + 1 }, (_, i) => minHour + i),
    [minHour, maxHour]
  );

  // Group events by dayKey
  const byDay = useMemo(() => {
    const map = new Map<string, FormattedEvent[]>();
    for (const dk of dayKeys) map.set(dk, []);
    for (const e of events) map.get(e.dayKey)?.push(e);
    return map;
  }, [events, dayKeys]);

  // Compute which hour-slot an event occupies (simplified: first hour)
  const eventsByDayHour = useMemo(() => {
    const map = new Map<string, FormattedEvent[]>(); // key: "dayKey:hour"
    for (const e of events) {
      const hour = new Date(e.start_at).getHours();
      const key = `${e.dayKey}:${hour}`;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(e);
    }
    return map;
  }, [events]);

  const headerTextClass = isDark ? "text-[#d4d4d4]" : "text-[#7a6b5a]";
  const borderClass = isDark ? "border-white/10" : "border-black/[0.08]";
  const timeCellClass = isDark ? "text-[#d4d4d4] bg-[rgba(26,23,20,0.6)]" : "text-[#7a6b5a] bg-[rgba(255,255,255,0.5)]";

  return (
    <div
      role="grid"
      aria-label="Open-Source Week schedule"
      aria-rowcount={hours.length + 1}
      aria-colcount={dayKeys.length + 1}
      className="overflow-x-auto custom-scrollbar rounded-[20px]"
    >
      <div
        className={`min-w-[600px] border rounded-[20px] overflow-hidden ${
          isDark ? "border-white/10" : "border-black/[0.08]"
        }`}
      >
        {/* Header row: Time label + day columns */}
        <div role="row" className={`grid border-b ${borderClass}`} style={{ gridTemplateColumns: `80px repeat(${dayKeys.length}, minmax(160px, 1fr))` }}>
          <div
            role="columnheader"
            aria-label="Time"
            className={`px-3 py-3 text-[11px] font-semibold uppercase ${timeCellClass} sticky left-0 z-10`}
          >
            Time
          </div>
          {dayKeys.map((dk) => (
            <div
              key={dk}
              role="columnheader"
              className={`px-3 py-3 text-[12px] font-semibold text-center border-l ${borderClass} ${headerTextClass} ${
                isDark ? "bg-white/[0.04]" : "bg-white/[0.10]"
              }`}
            >
              {fmtDayLabel(dk)}
            </div>
          ))}
        </div>

        {/* Time rows */}
        {hours.map((hour, rowIdx) => {
          const timeLabel = `${String(hour).padStart(2, "0")}:00`;
          return (
            <div
              key={hour}
              role="row"
              aria-label={timeLabel}
              className={`grid border-b ${borderClass} last:border-b-0`}
              style={{ gridTemplateColumns: `80px repeat(${dayKeys.length}, minmax(160px, 1fr))` }}
            >
              {/* Sticky time cell */}
              <div
                role="rowheader"
                className={`px-3 py-2 text-[11px] font-mono sticky left-0 z-10 flex items-start pt-2.5 ${timeCellClass}`}
              >
                {timeLabel}
              </div>

              {/* Day cells */}
              {dayKeys.map((dk) => {
                const cellKey = `${dk}:${hour}`;
                const cellEvents = eventsByDayHour.get(cellKey) ?? [];
                return (
                  <div
                    key={dk}
                    role="gridcell"
                    className={`min-h-[72px] px-2 py-2 border-l ${borderClass} space-y-1.5 ${
                      isDark ? "bg-white/[0.02]" : "bg-white/[0.04]"
                    }`}
                  >
                    {cellEvents.length === 0 ? null : (
                      cellEvents.map((ev) => (
                        <SessionBlock
                          key={ev.id}
                          event={ev}
                          isDark={isDark}
                          nowMs={nowMs}
                          onClick={() => onEventClick(ev.id, ev.title)}
                        />
                      ))
                    )}
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// View toggle
// ---------------------------------------------------------------------------

interface ViewToggleProps {
  view: ViewMode;
  onChange: (v: ViewMode) => void;
  isDark: boolean;
}

function ViewToggle({ view, onChange, isDark }: ViewToggleProps) {
  const btnRef = useRef<HTMLButtonElement>(null);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
        e.preventDefault();
        onChange(view === "calendar" ? "list" : "calendar");
      }
    },
    [view, onChange]
  );

  const base = `flex items-center gap-1.5 px-3.5 py-2 rounded-[12px] text-[13px] font-semibold transition-all border focus-visible:outline-2 focus-visible:outline-offset-2`;
  const active = `bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white border-transparent shadow-[0_4px_14px_rgba(162,121,44,0.35)]`;
  const inactive = isDark
    ? `bg-white/[0.06] border-white/10 text-[#d4d4d4] hover:bg-white/[0.10]`
    : `bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.25]`;

  return (
    <div
      role="group"
      aria-label="View mode"
      className={`flex items-center gap-1 p-1 rounded-[14px] border ${
        isDark ? "bg-white/[0.06] border-white/10" : "bg-white/[0.15] border-white/25"
      }`}
      onKeyDown={handleKeyDown}
    >
      <button
        ref={btnRef}
        role="button"
        aria-pressed={view === "calendar"}
        aria-label="Calendar view"
        className={`${base} ${view === "calendar" ? active : inactive}`}
        onClick={() => onChange("calendar")}
      >
        <LayoutGrid className="w-3.5 h-3.5" aria-hidden="true" />
        <span>Calendar</span>
      </button>
      <button
        role="button"
        aria-pressed={view === "list"}
        aria-label="List view"
        className={`${base} ${view === "list" ? active : inactive}`}
        onClick={() => onChange("list")}
      >
        <List className="w-3.5 h-3.5" aria-hidden="true" />
        <span>List</span>
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page component
// ---------------------------------------------------------------------------

export function OpenSourceWeekPage({ onEventClick }: OpenSourceWeekPageProps) {
  const { theme } = useTheme();
  const isDark = isDarkVariant(theme);

  const [isLoading, setIsLoading] = useState(true);
  const [rawEvents, setRawEvents] = useState<OSWEvent[]>([]);
  const [view, setView] = useState<ViewMode>("list");
  const [nowMs, setNowMs] = useState(() => Date.now());

  // Tick clock for "starting soon" / "live" badges
  useEffect(() => {
    const id = setInterval(() => setNowMs(Date.now()), 30_000);
    return () => clearInterval(id);
  }, []);

  // Detect narrow viewport → force list view
  useEffect(() => {
    const mq = window.matchMedia("(max-width: 767px)");
    const handle = (e: MediaQueryListEvent | MediaQueryList) => {
      if (e.matches) setView("list");
    };
    handle(mq);
    mq.addEventListener("change", handle);
    return () => mq.removeEventListener("change", handle);
  }, []);

  // Fetch events
  useEffect(() => {
    let mounted = true;
    setIsLoading(true);
    getOpenSourceWeekEvents()
      .then((res) => {
        if (mounted) setRawEvents(res.events ?? []);
      })
      .catch(() => {
        if (mounted) setRawEvents([]);
      })
      .finally(() => {
        if (mounted) setIsLoading(false);
      });
    return () => { mounted = false; };
  }, []);

  // Derive formatted events
  const events = useMemo<FormattedEvent[]>(() => {
    return rawEvents
      .map((e) => ({
        ...e,
        sessionType: deriveSessionType(e.title),
        startDate: fmtDate(e.start_at),
        endDate: fmtDate(e.end_at),
        startTime: fmtTime(e.start_at),
        endTime: fmtTime(e.end_at),
        statusLabel: getStatusLabel(e.status),
        startMs: new Date(e.start_at).getTime(),
        endMs: new Date(e.end_at).getTime(),
        dayKey: fmtDayKey(e.start_at),
      }))
      .sort((a, b) => a.startMs - b.startMs);
  }, [rawEvents]);

  // Group events by day for list view
  const groupedByDay = useMemo(() => {
    const map = new Map<string, FormattedEvent[]>();
    for (const e of events) {
      if (!map.has(e.dayKey)) map.set(e.dayKey, []);
      map.get(e.dayKey)!.push(e);
    }
    return map;
  }, [events]);

  // Announce view-mode changes to screen readers
  const [announcement, setAnnouncement] = useState("");
  const handleViewChange = useCallback((v: ViewMode) => {
    setView(v);
    setAnnouncement(`Switched to ${v === "calendar" ? "calendar" : "list"} view`);
    setTimeout(() => setAnnouncement(""), 1000);
  }, []);

  // Whether we're on a narrow viewport (toggle hidden)
  const [isNarrow, setIsNarrow] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth < 768 : false
  );
  useEffect(() => {
    const mq = window.matchMedia("(max-width: 767px)");
    const handle = (e: MediaQueryListEvent | MediaQueryList) => setIsNarrow(e.matches);
    handle(mq);
    mq.addEventListener("change", handle);
    return () => mq.removeEventListener("change", handle);
  }, []);

  const textPrimary = isDark ? "text-[#f5f5f5]" : "text-[#2d2820]";
  const textSecondary = isDark ? "text-[#d4d4d4]" : "text-[#7a6b5a]";

  return (
    <div className="space-y-6">
      {/* ARIA live region for screen-reader announcements */}
      <div
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
        role="status"
      >
        {announcement}
      </div>

      {/* ── Page header ──────────────────────────────────────── */}
      <header className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          {/* Icon */}
          <div className="w-14 h-14 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15 shrink-0">
            <Calendar className="w-7 h-7 text-white" aria-hidden="true" />
          </div>
          <div>
            <h1 className={`text-[24px] sm:text-[28px] font-bold ${textPrimary}`}>
              Open-Source Week
            </h1>
            <p className={`text-[14px] mt-0.5 ${textSecondary}`}>
              Gear-round Hack — a week for developers focused on rewarding.
            </p>
          </div>
        </div>

        {/* View toggle — hidden below 768 px */}
        {!isNarrow && (
          <ViewToggle
            view={view}
            onChange={handleViewChange}
            isDark={isDark}
          />
        )}
      </header>

      {/* ── Content ──────────────────────────────────────────── */}
      <section aria-label="Open-Source Week agenda">
        {isLoading ? (
          <div className="space-y-4">
            {[0, 1, 2].map((i) => (
              <EventCardSkeleton key={i} isDark={isDark} />
            ))}
          </div>
        ) : events.length === 0 ? (
          <EmptyState
            variant="no-programs"
            isDark={isDark}
            headline="No Open-Source Week events yet"
            subtext="Once an admin creates an event, it will show up here."
          />
        ) : view === "calendar" ? (
          <CalendarGrid
            events={events}
            isDark={isDark}
            nowMs={nowMs}
            onEventClick={onEventClick}
          />
        ) : (
          /* ── List view ── */
          <div className="space-y-6">
            {Array.from(groupedByDay.entries()).map(([dayKey, dayEvents]) => (
              <div key={dayKey}>
                {/* Day header */}
                <div
                  className={`mb-3 flex items-center gap-3 text-[12px] font-semibold uppercase tracking-wide ${textSecondary}`}
                >
                  <span>{fmtDayLabel(dayKey)}</span>
                  <span
                    className={`flex-1 h-px ${isDark ? "bg-white/10" : "bg-black/[0.08]"}`}
                    aria-hidden="true"
                  />
                </div>
                <ul
                  role="list"
                  aria-label={`Events on ${fmtDayLabel(dayKey)}`}
                  className="space-y-4"
                >
                  {dayEvents.map((ev) => (
                    <EventCard
                      key={ev.id}
                      event={ev}
                      isDark={isDark}
                      nowMs={nowMs}
                      onClick={() => onEventClick(ev.id, ev.title)}
                    />
                  ))}
                </ul>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
