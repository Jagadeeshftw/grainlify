import { X } from 'lucide-react';
import type { KeyboardEvent } from 'react';

interface FilterChipProps {
  label: string;
  onRemove: () => void;
  isDark: boolean;
  /** Exposes the remove button's DOM node so a parent list can manage focus after removal. */
  buttonRef?: (el: HTMLButtonElement | null) => void;
}

export function FilterChip({ label, onRemove, isDark, buttonRef }: FilterChipProps) {
  const handleKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    // Backspace/Delete is a common shorthand for "remove this chip" so keyboard
    // users don't have to rely solely on Enter/Space once the remove button is focused.
    if (e.key === 'Backspace' || e.key === 'Delete') {
      e.preventDefault();
      onRemove();
    }
  };

  return (
    <li
      className={`inline-flex items-center gap-1 pl-3 pr-1 py-1 rounded-full border text-[12px] font-medium leading-none max-w-full ${
        isDark
          ? 'bg-[#c9983a]/20 border-[#c9983a]/40 text-[#e8c77f]'
          : 'bg-[#c9983a]/15 border-[#c9983a]/35 text-[#8b6527]'
      }`}
    >
      <span className="truncate max-w-[160px]">{label}</span>
      <button
        ref={buttonRef}
        type="button"
        onClick={onRemove}
        onKeyDown={handleKeyDown}
        aria-label={`Remove ${label} filter`}
        className={`flex items-center justify-center w-6 h-6 rounded-full shrink-0 transition-colors outline-2 outline-offset-1 outline-transparent focus-visible:outline-[#f1b400] ${
          isDark
            ? 'hover:bg-[#c9983a]/40 active:bg-[#c9983a]/50 text-[#e8c77f]'
            : 'hover:bg-[#c9983a]/30 active:bg-[#c9983a]/40 text-[#8b6527]'
        }`}
      >
        <X className="w-3 h-3" strokeWidth={2.5} aria-hidden="true" />
      </button>
    </li>
  );
}
