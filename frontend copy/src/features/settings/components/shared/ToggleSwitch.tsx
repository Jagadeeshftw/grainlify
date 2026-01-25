interface ToggleSwitchProps {
  enabled: boolean;
  onChange: (value: boolean) => void;
}

export function ToggleSwitch({ enabled, onChange }: ToggleSwitchProps) {
  return (
    <button
      onClick={() => onChange(!enabled)}
      className={`relative w-11 h-6 rounded-full transition-all duration-300 ${
        enabled 
          ? 'bg-gradient-to-r from-[#c9983a] to-[#a67c2e] shadow-[0_2px_8px_rgba(162,121,44,0.4)]' 
          : 'bg-white/[0.15] border border-white/25'
      }`}
    >
      <div
        className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-md transition-all duration-300 ${
          enabled ? 'left-[22px]' : 'left-0.5'
        }`}
      />
    </button>
  );
}
