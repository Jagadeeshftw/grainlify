import { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { ChevronDown, Search, X, Check } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';
import { ActiveFilterChips } from './ActiveFilterChips';

interface BaseFilterDropdownProps {
  label: string;
  options: string[];
  placeholder?: string;
}

interface SingleSelectFilterDropdownProps extends BaseFilterDropdownProps {
  multiple?: false;
  value: string;
  onChange: (value: string) => void;
}

interface MultiSelectFilterDropdownProps extends BaseFilterDropdownProps {
  multiple: true;
  /** Currently active tag/topic values, shown below the trigger as removable chips. */
  value: string[];
  onChange: (value: string[]) => void;
}

type FilterDropdownProps = SingleSelectFilterDropdownProps | MultiSelectFilterDropdownProps;

export function FilterDropdown(props: FilterDropdownProps) {
  const { label, options, placeholder, multiple } = props;
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const buttonRef = useRef<HTMLButtonElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const [dropdownPosition, setDropdownPosition] = useState({ top: 0, left: 0, width: 0 });

  // Update dropdown position when opened
  useEffect(() => {
    if (isOpen && buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect();
      const position = {
        top: rect.bottom + 8,
        left: rect.left,
        width: rect.width,
      };
      setDropdownPosition(position);
    }
  }, [isOpen]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
        setSearchQuery('');
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isOpen]);

  // Filter options based on search
  const filteredOptions = options.filter(option =>
    option.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const isOptionSelected = (optionValue: string) =>
    multiple ? props.value.includes(optionValue) : props.value === optionValue;

  const handleSelect = (option: string) => {
    if (multiple) {
      const next = props.value.includes(option)
        ? props.value.filter((v) => v !== option)
        : [...props.value, option];
      props.onChange(next);
      setSearchQuery('');
      // Dropdown stays open in multi-select mode so users can pick several tags in one pass.
      return;
    }
    props.onChange(option);
    setIsOpen(false);
    setSearchQuery('');
  };

  const handleRemoveChip = (option: string) => {
    if (!multiple) return;
    props.onChange(props.value.filter((v) => v !== option));
  };

  const handleButtonClick = () => {
    setIsOpen(!isOpen);
  };

  const displayValue = multiple
    ? props.value.length > 0
      ? `${label} (${props.value.length})`
      : label
    : props.value === 'all'
      ? label
      : props.value;

  return (
    <>
      <button
        ref={buttonRef}
        onClick={handleButtonClick}
        className={`w-full px-4 py-3 rounded-[12px] backdrop-blur-[30px] border text-[14px] font-medium transition-all flex items-center justify-between ${
          theme === 'dark'
            ? 'bg-[#1a1512]/[0.6] border-white/15 text-[#e8dfd0] hover:bg-[#1a1512]/[0.8] hover:border-[#c9983a]/30'
            : 'bg-white/[0.25] border-white/35 text-[#2d2820] hover:bg-white/[0.35] hover:border-[#c9983a]/40'
        }`}
      >
        <span className={!multiple && props.value === 'all' ? 'opacity-60' : ''}>{displayValue}</span>
        <ChevronDown className={`w-4 h-4 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
      </button>

      {multiple && props.value.length > 0 && (
        <div className="mt-2">
          <ActiveFilterChips
            filters={props.value}
            onRemove={handleRemoveChip}
            isDark={isDark}
            ariaLabel={`Active ${label.toLowerCase()} filters`}
            onAllRemoved={() => buttonRef.current?.focus()}
          />
        </div>
      )}

      {isOpen && createPortal(
        <div
          ref={dropdownRef}
          style={{
            position: 'fixed',
            top: `${dropdownPosition.top}px`,
            left: `${dropdownPosition.left}px`,
            width: `${dropdownPosition.width}px`,
            zIndex: 9999,
          }}
          className={`rounded-[16px] backdrop-blur-[40px] border shadow-xl overflow-hidden ${
            theme === 'dark'
              ? 'bg-[#1a1512]/[0.95] border-white/15'
              : 'bg-[#2d2820]/[0.95] border-white/30'
          }`}
        >
          {/* Search Input */}
          <div className="p-3 border-b border-white/10">
            <div className={`flex items-center gap-2 px-3 py-2 rounded-[10px] backdrop-blur-[20px] border ${
              theme === 'dark'
                ? 'bg-white/[0.06] border-white/10'
                : 'bg-white/[0.08] border-white/15'
            }`}>
              <Search className="w-4 h-4 text-[#b8a898]" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={placeholder || `Search ${label.toLowerCase()}...`}
                className="flex-1 bg-transparent border-none outline-none text-[13px] text-[#e8dfd0] placeholder:text-[#b8a898]/50"
              />
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery('')}
                  className="p-0.5 hover:bg-white/10 rounded transition-colors"
                >
                  <X className="w-3 h-3 text-[#b8a898]" />
                </button>
              )}
            </div>
          </div>

          {/* Options List */}
          <div className="max-h-[280px] overflow-y-auto custom-scrollbar">
            {filteredOptions.length === 0 ? (
              <div className="px-4 py-8 text-center">
                <p className="text-[13px] text-[#b8a898]">No options found.</p>
              </div>
            ) : (
              <div className="py-2">
                {filteredOptions.map((option) => {
                  // Convert "All Languages" -> "all", otherwise keep the option as-is
                  const optionValue = option.toLowerCase().startsWith('all ') ? 'all' : option;
                  
                  const selected = isOptionSelected(optionValue);

                  return (
                    <button
                      key={option}
                      onClick={() => handleSelect(optionValue)}
                      role="option"
                      aria-selected={selected}
                      className={`w-full px-4 py-2.5 text-left text-[13px] transition-colors flex items-center justify-between gap-2 ${
                        selected
                          ? theme === 'dark'
                            ? 'bg-[#c9983a]/20 text-[#c9983a] font-semibold'
                            : 'bg-[#c9983a]/30 text-[#c9983a] font-semibold'
                          : theme === 'dark'
                            ? 'text-[#d4c5b0] hover:bg-white/[0.08]'
                            : 'text-[#e8dfd0] hover:bg-white/[0.1]'
                      }`}
                    >
                      <span className="truncate">{option}</span>
                      {multiple && selected && <Check className="w-4 h-4 shrink-0" aria-hidden="true" />}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </div>,
        document.body
      )}
    </>
  );
}