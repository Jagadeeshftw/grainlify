/**
 * SessionTagChip — color-coded tag chip for OSW session types.
 *
 * Session types: "workshop" | "panel" | "office-hours"
 *
 * Color contract (WCAG 1.4.3 AA — all combos ≥ 4.5:1):
 *
 * Workshop
 *   Light: text #6d5530 on rgba(201,152,58,0.15) → 5.8:1 ✅
 *   Dark:  text #e8c77f on rgba(201,152,58,0.20) → 6.4:1 ✅
 *
 * Panel
 *   Light: text #1e3a8a on rgba(59,130,246,0.15) over #e8dfd0 → 6.77:1 ✅
 *   Dark:  text #93c5fd on rgba(59,130,246,0.20) over #1a1714 → 7.76:1 ✅
 *
 * Office Hours
 *   Light: text #14532d on rgba(34,197,94,0.15) over #e8dfd0 → 6.25:1 ✅
 *   Dark:  text #86efac on rgba(34,197,94,0.20) over #1a1714 → 8.91:1 ✅
 *
 * @see design/specs/open-source-week-agenda.md §3
 */

export type SessionType = 'workshop' | 'panel' | 'office-hours';

interface SessionTagChipProps {
  type: SessionType;
  isDark: boolean;
  /** When true, renders a smaller inline variant without the dot (for tight spaces) */
  compact?: boolean;
}

const TYPE_META: Record<
  SessionType,
  {
    label: string;
    dotLight: string;
    dotDark: string;
    textLight: string;
    textDark: string;
    bgLight: string;
    bgDark: string;
    borderLight: string;
    borderDark: string;
  }
> = {
  workshop: {
    label: 'Workshop',
    dotLight: '#c9983a',
    dotDark: '#e8c77f',
    textLight: '#6d5530',
    textDark: '#e8c77f',
    bgLight: 'rgba(201,152,58,0.15)',
    bgDark: 'rgba(201,152,58,0.20)',
    borderLight: 'rgba(201,152,58,0.30)',
    borderDark: 'rgba(201,152,58,0.40)',
  },
  panel: {
    label: 'Panel',
    dotLight: '#1e40af',
    dotDark: '#93c5fd',
    textLight: '#1e3a8a',
    textDark: '#93c5fd',
    bgLight: 'rgba(59,130,246,0.15)',
    bgDark: 'rgba(59,130,246,0.20)',
    borderLight: 'rgba(59,130,246,0.30)',
    borderDark: 'rgba(59,130,246,0.40)',
  },
  'office-hours': {
    label: 'Office Hours',
    dotLight: '#15803d',
    dotDark: '#86efac',
    textLight: '#14532d',
    textDark: '#86efac',
    bgLight: 'rgba(34,197,94,0.15)',
    bgDark: 'rgba(34,197,94,0.20)',
    borderLight: 'rgba(34,197,94,0.30)',
    borderDark: 'rgba(34,197,94,0.40)',
  },
};

export function SessionTagChip({ type, isDark, compact = false }: SessionTagChipProps) {
  const meta = TYPE_META[type];

  const bg = isDark ? meta.bgDark : meta.bgLight;
  const border = isDark ? meta.borderDark : meta.borderLight;
  const text = isDark ? meta.textDark : meta.textLight;
  const dot = isDark ? meta.dotDark : meta.dotLight;

  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-[14px] font-semibold border leading-none select-none ${
        compact ? 'px-2 py-0.5 text-[10px]' : 'px-3 py-1 text-[11px]'
      }`}
      style={{ background: bg, borderColor: border, color: text }}
    >
      {/* Color dot — non-decorative: provides visual type affordance paired with text label */}
      {!compact && (
        <span
          aria-hidden="true"
          className="w-1.5 h-1.5 rounded-full flex-shrink-0"
          style={{ backgroundColor: dot }}
        />
      )}
      {meta.label}
    </span>
  );
}

/**
 * Derives a SessionType from an event title when the server does not
 * supply an explicit session_type field.
 *
 * Priority: "panel" > "office" > "workshop" (default).
 */
export function deriveSessionType(title: string): SessionType {
  const t = title.toLowerCase();
  if (t.includes('panel')) return 'panel';
  if (t.includes('office')) return 'office-hours';
  // Default to workshop — most common session type
  return 'workshop';
}
