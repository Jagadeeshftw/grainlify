import { useEffect, useRef, useState } from 'react';
import { FilterChip } from './FilterChip';

interface ActiveFilterChipsProps {
  filters: string[];
  onRemove: (filter: string) => void;
  isDark: boolean;
  /** Number of chips shown before collapsing the rest into a "+N more" chip. */
  maxVisible?: number;
  ariaLabel?: string;
  /** Called when the last remaining chip is removed, so the caller can redirect focus. */
  onAllRemoved?: () => void;
}

export function ActiveFilterChips({
  filters,
  onRemove,
  isDark,
  maxVisible = 6,
  ariaLabel = 'Active filters',
  onAllRemoved,
}: ActiveFilterChipsProps) {
  const [expanded, setExpanded] = useState(false);
  const removedIndexRef = useRef<number | null>(null);
  const buttonRefs = useRef<Map<string, HTMLButtonElement>>(new Map());

  // Removing a chip unmounts its button, so the browser drops focus to <body>
  // instead of advancing it. Re-focus the chip that now sits at the same
  // position (or the new last chip) once the filters list settles.
  useEffect(() => {
    const removedAt = removedIndexRef.current;
    removedIndexRef.current = null;
    if (removedAt === null) return;

    if (filters.length === 0) {
      onAllRemoved?.();
      return;
    }

    const nextIndex = Math.min(removedAt, filters.length - 1);
    const nextFilter = filters[nextIndex];
    buttonRefs.current.get(nextFilter)?.focus();
  }, [filters, onAllRemoved]);

  if (filters.length === 0) return null;

  const overflowCount = filters.length - maxVisible;
  const isCollapsed = overflowCount > 0 && !expanded;
  const visibleFilters = isCollapsed ? filters.slice(0, maxVisible) : filters;

  const handleRemove = (filter: string) => {
    removedIndexRef.current = filters.indexOf(filter);
    onRemove(filter);
  };

  const chipButtonTheme = isDark
    ? 'bg-white/[0.08] border-white/15 text-[#d4c5b0] hover:bg-white/[0.12]'
    : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]';

  return (
    // role="list" is redundant on a plain <ul>, but VoiceOver drops list
    // semantics once list-style is removed (list-none) — this keeps it announced.
    <ul role="list" aria-label={ariaLabel} className="flex flex-wrap items-center gap-2 list-none">
      {visibleFilters.map((filter) => (
        <FilterChip
          key={filter}
          label={filter}
          isDark={isDark}
          onRemove={() => handleRemove(filter)}
          buttonRef={(el) => {
            if (el) buttonRefs.current.set(filter, el);
            else buttonRefs.current.delete(filter);
          }}
        />
      ))}

      {isCollapsed && (
        <li>
          <button
            type="button"
            onClick={() => setExpanded(true)}
            aria-label={`Show ${overflowCount} more active filter${overflowCount === 1 ? '' : 's'}`}
            className={`inline-flex items-center px-3 py-1 rounded-full border text-[12px] font-semibold transition-colors outline-2 outline-offset-2 outline-transparent focus-visible:outline-[#f1b400] ${chipButtonTheme}`}
          >
            +{overflowCount} more
          </button>
        </li>
      )}

      {expanded && filters.length > maxVisible && (
        <li>
          <button
            type="button"
            onClick={() => setExpanded(false)}
            className={`inline-flex items-center px-3 py-1 rounded-full border text-[12px] font-semibold transition-colors outline-2 outline-offset-2 outline-transparent focus-visible:outline-[#f1b400] ${chipButtonTheme}`}
          >
            Show less
          </button>
        </li>
      )}
    </ul>
  );
}
