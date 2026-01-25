import { ChevronDown, Check } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

interface IssueFilterDropdownProps {
  value: string;
  onChange: (value: string) => void;
  isOpen: boolean;
  onToggle: () => void;
  onClose: () => void;
}

const filterOptions = ['All', 'Waiting for review', 'In progress', 'Stale'];

export function IssueFilterDropdown({ value, onChange, isOpen, onToggle, onClose }: IssueFilterDropdownProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const handleSelect = (option: string) => {
    onChange(option);
    onClose();
  };

  return (
    <div className="relative flex-1 z-50">
      <button 
        className={`w-full flex items-center justify-between px-4 py-3 rounded-[14px] backdrop-blur-[25px] border transition-all ${
          isDark
            ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12] hover:border-[#e8c571]/30'
            : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:border-[#c9983a]/30'
        }`}
        onClick={onToggle}
      >
        <span className={`text-[14px] font-semibold ${
          isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
        }`}>
          {value}
        </span>
        <ChevronDown className={`w-4 h-4 transition-transform ${
          isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        } ${isOpen ? 'rotate-180' : ''}`} />
      </button>
      
      {/* Dropdown Menu */}
      {isOpen && (
        <>
          {/* Backdrop to close dropdown */}
          <div 
            className="fixed inset-0 z-40" 
            onClick={onClose}
          />
          
          {/* Dropdown content */}
          <div className={`absolute top-full left-0 right-0 mt-2 rounded-[20px] border z-50 overflow-hidden ${
            isDark
              ? 'bg-[#3a3228] border-white/30'
              : 'bg-[#d4c5b0] border-white/40'
          }`}>
            {/* Header */}
            <div className={`px-6 py-5 border-b-2 ${
              isDark
                ? 'border-white/20 bg-gradient-to-b from-white/10 to-transparent'
                : 'border-white/30 bg-gradient-to-b from-white/15 to-transparent'
            }`}>
              <h3 className={`text-[17px] font-bold ${
                isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}>
                DEFAULT
              </h3>
            </div>
            
            {/* Options */}
            <div className="py-3">
              {filterOptions.map((option) => (
                <button
                  key={option}
                  className={`w-full px-6 py-3.5 flex items-center justify-between transition-all group ${
                    isDark
                      ? 'hover:bg-[#4a3e30]'
                      : 'hover:bg-[#c9b8a0]'
                  }`}
                  onClick={() => handleSelect(option)}
                >
                  <span className={`text-[15px] font-bold transition-colors ${
                    isDark
                      ? 'text-[#e8dfd0] group-hover:text-[#e8c571]'
                      : 'text-[#2d2820] group-hover:text-[#c9983a]'
                  }`}>
                    {option}
                  </span>
                  {value === option && (
                    <Check className={`w-5 h-5 ${
                      isDark ? 'text-[#e8c571]' : 'text-[#c9983a]'
                    }`} strokeWidth={2.5} />
                  )}
                </button>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
