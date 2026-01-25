import { useState } from 'react';
import { Filter, Search, X, Circle, ChevronDown, Check } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

interface FilterOption {
  label: string;
  value: string;
}

interface SearchWithFilterProps {
  searchPlaceholder?: string;
  searchValue?: string;
  onSearchChange?: (value: string) => void;
  filterSections?: {
    title: string;
    options?: FilterOption[];
    selectedValues?: string[];
    onToggle?: (value: string) => void;
    hasSearch?: boolean;
  }[];
  onReset?: () => void;
}

export function SearchWithFilter({
  searchPlaceholder = 'Search',
  searchValue = '',
  onSearchChange,
  filterSections = [],
  onReset,
}: SearchWithFilterProps) {
  const { theme } = useTheme();
  const [isFilterOpen, setIsFilterOpen] = useState(false);
  const [openSections, setOpenSections] = useState<Record<string, boolean>>({});

  const toggleSection = (sectionTitle: string) => {
    setOpenSections(prev => ({
      ...prev,
      [sectionTitle]: !prev[sectionTitle],
    }));
  };

  const handleReset = () => {
    onReset?.();
  };

  return (
    <>
      {/* Filter and Search Bar */}
      <div className="flex items-center gap-3">
        {/* Filter Button */}
        <button
          onClick={() => setIsFilterOpen(true)}
          className={`w-12 h-12 flex items-center justify-center rounded-[12px] backdrop-blur-[30px] border transition-all ${
            theme === 'dark'
              ? 'bg-white/[0.15] border-white/25 text-[#d4d4d4] hover:bg-white/[0.2] hover:border-[#c9983a]/40'
              : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2] hover:border-[#c9983a]/40'
          }`}
        >
          <Filter className="w-5 h-5" />
        </button>

        {/* Search Bar */}
        <div className="relative flex-1">
          <Search className={`absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 z-10 ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`} />
          <input
            type="text"
            placeholder={searchPlaceholder}
            value={searchValue}
            onChange={(e) => onSearchChange?.(e.target.value)}
            className={`w-full pl-12 pr-4 py-3 rounded-[12px] backdrop-blur-[30px] border focus:outline-none transition-all text-[13px] ${
              theme === 'dark'
                ? 'bg-white/[0.15] border-white/25 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.2] focus:border-[#c9983a]/40'
                : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/40'
            }`}
          />
        </div>
      </div>

      {/* Filter Modal */}
      {isFilterOpen && (
        <>
          {/* Backdrop */}
          <div
            className="fixed inset-0 bg-black/40 backdrop-blur-sm z-40 transition-opacity"
            onClick={() => setIsFilterOpen(false)}
          />

          {/* Filter Panel */}
          <div className={`fixed top-0 right-0 h-full w-[400px] backdrop-blur-[40px] border-l z-50 shadow-[0_0_40px_rgba(0,0,0,0.15)] p-6 flex flex-col animate-slide-in-right ${
            theme === 'dark'
              ? 'bg-[#2d2820]/95 border-white/30'
              : 'bg-[#e5ddd1]/95 border-white/30'
          }`}>
            {/* Header */}
            <div className="flex items-center justify-between mb-6">
              <h2 className={`text-[18px] font-bold ${
                theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>Filter</h2>
              <button
                onClick={() => setIsFilterOpen(false)}
                className={`w-8 h-8 flex items-center justify-center rounded-lg transition-all ${
                  theme === 'dark'
                    ? 'hover:bg-white/[0.1] text-[#f5f5f5]'
                    : 'hover:bg-white/[0.3] text-[#2d2820]'
                }`}
              >
                <X className="w-6 h-6 stroke-[2.5]" />
              </button>
            </div>

            {/* Filter Content */}
            <div className="flex-1 space-y-5 overflow-y-auto scrollbar-hide">
              {filterSections.map((section, idx) => (
                <div key={idx}>
                  <button
                    onClick={() => toggleSection(section.title)}
                    className="w-full flex items-center justify-between mb-3"
                  >
                    <div className="flex items-center gap-3">
                      <Circle className={`w-5 h-5 ${
                        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                      }`} />
                      <span className={`text-[14px] font-semibold ${
                        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                      }`}>{section.title}</span>
                      {section.selectedValues && section.selectedValues.length > 0 && (
                        <span className="px-2 py-0.5 bg-[#c9983a] text-white text-[11px] font-semibold rounded-full">
                          {section.selectedValues.length}
                        </span>
                      )}
                    </div>
                    <ChevronDown className={`w-5 h-5 transition-transform ${
                      openSections[section.title] ? 'rotate-180' : ''
                    } ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`} />
                  </button>

                  {openSections[section.title] && (
                    <div className="space-y-3">
                      {/* Search Field if enabled */}
                      {section.hasSearch && (
                        <div className="relative">
                          <Search className={`absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 ${
                            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                          }`} />
                          <input
                            type="text"
                            placeholder="Search"
                            className={`w-full pl-10 pr-4 py-3 rounded-[12px] backdrop-blur-[30px] border text-[13px] focus:outline-none transition-all ${
                              theme === 'dark'
                                ? 'bg-white/[0.15] border-white/25 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.2] focus:border-[#c9983a]/40'
                                : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/40'
                            }`}
                          />
                        </div>
                      )}

                      {/* Options List */}
                      {section.options && section.options.length > 0 ? (
                        <div className="space-y-1">
                          {section.options.map((option) => (
                            <button
                              key={option.value}
                              onClick={() => section.onToggle?.(option.value)}
                              className={`w-full px-4 py-3 rounded-[12px] text-left text-[13px] font-medium transition-all flex items-center justify-between ${
                                section.selectedValues?.includes(option.value)
                                  ? 'bg-[#c9983a] text-white shadow-[0_4px_12px_rgba(201,152,58,0.3)]'
                                  : theme === 'dark'
                                    ? 'backdrop-blur-[20px] bg-white/[0.1] border border-white/20 text-[#f5f5f5] hover:bg-white/[0.15]'
                                    : 'backdrop-blur-[20px] bg-white/[0.1] border border-white/20 text-[#2d2820] hover:bg-white/[0.15]'
                              }`}
                            >
                              <span>{option.label}</span>
                              {section.selectedValues?.includes(option.value) && (
                                <Check className="w-4 h-4" />
                              )}
                            </button>
                          ))}
                        </div>
                      ) : (
                        <div className={`backdrop-blur-[20px] rounded-[12px] border p-4 ${
                          theme === 'dark'
                            ? 'bg-white/[0.1] border-white/20'
                            : 'bg-white/[0.1] border-white/20'
                        }`}>
                          <p className={`text-[12px] text-center ${
                            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                          }`}>No items found</p>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              ))}
            </div>

            {/* Footer Buttons */}
            <div className="flex items-center gap-3 mt-6 pt-6 border-t border-white/20">
              <button
                onClick={handleReset}
                className={`flex-1 px-4 py-3 rounded-[12px] backdrop-blur-[30px] border text-[13px] font-semibold transition-all ${
                  theme === 'dark'
                    ? 'bg-white/[0.15] border-white/25 text-[#d4d4d4] hover:bg-white/[0.2]'
                    : 'bg-white/[0.15] border-white/25 text-[#6b5d4d] hover:bg-white/[0.2]'
                }`}
              >
                Reset
              </button>
              <button
                onClick={() => setIsFilterOpen(false)}
                className="flex-1 px-4 py-3 rounded-[12px] bg-[#c9983a] text-white text-[13px] font-semibold shadow-[0_4px_12px_rgba(201,152,58,0.3)] hover:shadow-[0_6px_16px_rgba(201,152,58,0.4)] transition-all"
              >
                Save
              </button>
            </div>
          </div>
        </>
      )}
    </>
  );
}
