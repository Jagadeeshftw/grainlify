interface ToggleSwitchProps {
  enabled: boolean;
  onChange: (value: boolean) => void;
  label?: string;
  disabled?: boolean;
}

export function ToggleSwitch({ enabled, onChange, label, disabled = false }: ToggleSwitchProps) {
  return (
    <button
      role="switch"
      aria-checked={enabled}
      aria-label={label}
      aria-disabled={disabled}
      disabled={disabled}
      onClick={() => !disabled && onChange(!enabled)}
      onKeyDown={(e) => {
        if ((e.key === ' ' || e.key === 'Enter') && !disabled) {
          e.preventDefault();
          onChange(!enabled);
        }
      }}
      className={[
        'relative w-11 h-6 rounded-full transition-all duration-300',
        'focus:outline-none focus-visible:ring-[3px] focus-visible:ring-[#c9983a]/35',
        enabled
          ? 'bg-gradient-to-r from-[#c9983a] to-[#a67c2e] shadow-[0_2px_8px_rgba(162,121,44,0.4)]'
          : 'bg-white/[0.15] border border-white/25',
        disabled ? 'opacity-40 cursor-not-allowed' : 'cursor-pointer',
      ].join(' ')}
    >
      <div
        className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-md transition-all duration-300 ${
          enabled ? 'left-[22px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}