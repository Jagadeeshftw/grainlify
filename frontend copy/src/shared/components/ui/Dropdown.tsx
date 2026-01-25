import { ChevronDown, Search, X } from 'lucide-react';
import { useTheme } from '../../contexts/ThemeContext';
import { useState, useRef, useEffect, ReactNode } from 'react';

interface DropdownOption {
  name: string;
  [key: string]: any;
}

interface DropdownProps {
  filterType: string;
  options: DropdownOption[];
  selectedValues: string[];
  onToggle: (value: string) => void;
  searchValue: string;
  onSearchChange: (value: string) => void;
  isOpen: boolean;
  onToggleOpen: () => void;
  onClose: () => void;
  renderOption?: (option: DropdownOption, isSelected: boolean) => ReactNode;
}

export function Dropdown({
  filterType,
  options,
  selectedValues,
  onToggle,
  searchValue,
  onSearchChange,
  isOpen,
  onToggleOpen,
  onClose,
  renderOption
}: DropdownProps) {
  const { theme } = useTheme();
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isOpen, onClose]);

  const filteredOptions = options.filter((option) =>
    option.name.toLowerCase().includes(searchValue.toLowerCase())
  );

  return (
    <div ref={dropdownRef} className="relative">
      <button
        onClick={onToggleOpen}
        className={`px-4 py-2.5 border-[1.5px] rounded-[12px] text-[14px] transition-all flex items-center space-x-2 font-semibold hover:scale-105 shadow-md ${
          selectedValues.length > 0
            ? theme === 'dark'
              ? 'bg-[#a17932] border-[#c9983a] text-white'
              : 'bg-[#b8872f] border-[#a17932] text-white'
            : theme === 'dark'
            ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4] hover:bg-white/[0.12] hover:text-[#f5f5f5] backdrop-blur-[30px]'
            : 'bg-white/[0.15] border-white/25 text-[#6b5d4d] hover:bg-white/[0.2] hover:text-[#2d2820] backdrop-blur-[30px]'
        }`}
      >
        <span>
          Select {filterType}
          {selectedValues.length > 0 && ` (${selectedValues.length})`}
        </span>
        <ChevronDown className={`w-4 h-4 transition-transform duration-300 ${isOpen ? 'rotate-180' : ''}`} />
      </button>

      {/* Dropdown */}
      {isOpen && (
        <div className={`absolute top-full left-0 mt-2 w-[340px] rounded-[16px] border-[1.5px] shadow-[0_20px_60px_rgba(0,0,0,0.4)] z-50 overflow-hidden animate-in fade-in slide-in-from-top-2 duration-200 backdrop-blur-[40px] ${
          theme === 'dark'
            ? 'bg-[#2d2820]/[0.95] border-[#c9983a]/30'
            : 'bg-[#d4c5b0]/[0.95] border-[#c9983a]/30'
        }`}>
          {/* Header with Title and Close Button */}
          <div className={`px-5 py-3.5 flex items-center justify-between border-b-[1.5px] ${
            theme === 'dark'
              ? 'border-white/[0.15] bg-gradient-to-b from-white/[0.03] to-transparent'
              : 'border-white/[0.2] bg-gradient-to-b from-white/[0.05] to-transparent'
          }`}>
            <h3 className={`text-[15px] font-bold capitalize ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Select {filterType}
            </h3>
            <button
              onClick={onClose}
              className={`p-1.5 rounded-lg transition-all hover:scale-110 ${
                theme === 'dark'
                  ? 'hover:bg-white/[0.1] text-[#c9983a] hover:text-[#f5c563]'
                  : 'hover:bg-black/[0.05] text-[#8b6f3a] hover:text-[#7a5a3a]'
              }`}
            >
              <X className="w-4 h-4" />
            </button>
          </div>

          {/* Search Input */}
          <div className="p-4">
            <div className="relative group">
              <Search 
                className={`absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 transition-colors ${
                  theme === 'dark' 
                    ? 'text-[#b8a898] group-focus-within:text-[#c9983a]' 
                    : 'text-[#7a6b5a] group-focus-within:text-[#c9983a]'
                }`} 
              />
              <input
                type="text"
                placeholder={`Search ${filterType}...`}
                value={searchValue}
                onChange={(e) => onSearchChange(e.target.value)}
                autoFocus
                className={`w-full pl-10 pr-3 py-2.5 rounded-[11px] border-[1.5px] focus:outline-none transition-all text-[14px] placeholder:text-[14px] font-normal ${
                  theme === 'dark'
                    ? 'bg-[#1a1512] border-white/[0.2] text-[#f5f5f5] placeholder-[#9a8a7a] focus:bg-[#1a1512] focus:border-[#c9983a] focus:shadow-[0_0_0_3px_rgba(201,152,58,0.15)]'
                    : 'bg-white/[0.3] backdrop-blur-[20px] border-white/[0.4] text-[#2d2820] placeholder-[#8a7a6a] focus:bg-white/[0.4] focus:border-[#c9983a] focus:shadow-[0_0_0_3px_rgba(201,152,58,0.1)]'
                }`}
              />
            </div>
          </div>

          {/* Options List */}
          <div className="max-h-[320px] overflow-y-auto">
            {filteredOptions.length > 0 ? (
              <div className="pb-2">
                {filteredOptions.map((option, idx) => {
                  const isSelected = selectedValues.includes(option.name);
                  
                  return (
                    <button
                      key={idx}
                      onClick={() => onToggle(option.name)}
                      className={`w-full px-4 py-3 transition-all flex items-start justify-between group ${
                        theme === 'dark'
                          ? 'hover:bg-white/[0.05]'
                          : 'hover:bg-black/[0.03]'
                      }`}
                    >
                      <div className="flex items-start gap-3 flex-1 min-w-0">
                        {/* Checkbox */}
                        <div className={`mt-0.5 flex-shrink-0 w-[19px] h-[19px] rounded-[5px] border-[2.5px] flex items-center justify-center transition-all ${
                          isSelected
                            ? 'bg-gradient-to-br from-[#c9983a] to-[#b8872f] border-[#c9983a] shadow-[0_3px_10px_rgba(201,152,58,0.4)]'
                            : theme === 'dark'
                            ? 'border-white/[0.3] group-hover:border-[#c9983a]/60'
                            : 'border-[#8a7a6a]/[0.35] group-hover:border-[#c9983a]/60'
                        }`}>
                          {isSelected && (
                            <svg className="w-3.5 h-3.5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="4">
                              <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                            </svg>
                          )}
                        </div>

                        {/* Option Content */}
                        {renderOption ? (
                          renderOption(option, isSelected)
                        ) : (
                          <div className="flex-1 min-w-0 text-left">
                            <div className={`text-[14px] font-semibold transition-colors ${
                              isSelected
                                ? theme === 'dark' ? 'text-[#f5c563]' : 'text-[#8b6f3a]'
                                : theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                            }`}>
                              {option.name}
                            </div>
                          </div>
                        )}
                      </div>
                    </button>
                  );
                })}
              </div>
            ) : (
              <div className={`px-4 py-12 text-center ${
                theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>
                <div className="text-[14px] font-semibold">No options found</div>
                <div className="text-[12px] mt-1 opacity-70">Try a different search term</div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
