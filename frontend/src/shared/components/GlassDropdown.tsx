import { ChevronDown } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';

interface GlassDropdownProps<T extends string> {
  value: T;
  onChange: (value: T) => void;
  options: T[];
  isOpen: boolean;
  onToggle: () => void;
  onClose: () => void;
}

export function GlassDropdown<T extends string>({ 
  value, 
  onChange, 
  options, 
  isOpen, 
  onToggle, 
  onClose 
}: GlassDropdownProps<T>) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const handleSelect = (option: T) => {
    onChange(option);
    onClose();
  };

  return (
    <div className="relative">
      {/* Dropdown Button */}
      <button 
        className={`flex items-center gap-2 px-5 py-3 rounded-[14px] backdrop-blur-[25px] border transition-all cursor-pointer shadow-elevation-1 hover:shadow-elevation-2 hover:scale-[1.02] focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 active:scale-100 ${
          isDark
            ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12] hover:border-[#e8c571]/30 text-[#e8dfd0]'
            : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:border-[#c9983a]/30 text-[#2d2820]'
        }`}
        onClick={onToggle}
      >
        <span className="text-[14px] font-semibold">
          {value}
        </span>
        <ChevronDown className={`w-4 h-4 transition-transform ${
          isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        } ${isOpen ? 'rotate-180' : ''}`} />
      </button>
      
      {/* Dropdown Menu */}
      {isOpen && (
        <>
          {/* Backdrop */}
          <div 
            className="fixed inset-0 z-40" 
            onClick={onClose}
          />
          
          {/* Menu */}
          <div className={`absolute top-full right-0 mt-2 w-48 rounded-[16px] border z-50 overflow-hidden backdrop-blur-[25px] shadow-elevation-3 transition-all ${
            isDark
              ? 'bg-[#1c1917]/90 border-white/15'
              : 'bg-[#fafaf9]/90 border-white/25'
          }`}>
            <div className="py-2">
              {options.map((option) => (
                <button
                  key={option}
                  className={`w-full px-5 py-2.5 text-left text-[13px] font-semibold transition-colors focus:outline-none focus:bg-[#c9983a]/10 ${
                    isDark
                      ? 'text-[#e8dfd0] hover:bg-white/[0.08] focus:text-[#e8c571]'
                      : 'text-[#2d2820] hover:bg-black/[0.05] focus:text-[#c9983a]'
                  }`}
                  onClick={() => handleSelect(option)}
                >
                  {option}
                </button>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
