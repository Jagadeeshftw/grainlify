import React, { ReactNode, useState, useRef, useEffect } from 'react';
import { X, ChevronDown } from 'lucide-react';
import { useTheme } from '../../contexts/ThemeContext';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  icon?: ReactNode;
  width?: 'sm' | 'md' | 'lg' | 'xl';
  showCloseButton?: boolean;
  maxHeight?: boolean;
  footer?: ReactNode;
}

const widthClasses = {
  sm: 'w-[95vw] sm:w-[400px]',
  md: 'w-[95vw] sm:w-[500px]',
  lg: 'w-[95vw] sm:w-[550px]',
  xl: 'w-[95vw] sm:w-[650px]'
};

export function Modal({
  isOpen,
  onClose,
  title,
  children,
  icon,
  width = 'md',
  showCloseButton = true,
  maxHeight = false,
  footer
}: ModalProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  React.useEffect(() => {
    if (isOpen) {
      document.body.classList.add('modal-open');
    } else {
      document.body.classList.remove('modal-open');
    }
    return () => {
      document.body.classList.remove('modal-open');
    };
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-[10000] animate-in fade-in duration-200"
      onClick={onClose}
    >
      <div
        className={`rounded-[16px] md:rounded-[24px] border-2 shadow-[0_20px_60px_rgba(0,0,0,0.3)] ${widthClasses[width]} max-w-[95vw] sm:max-w-[90vw] max-h-[90vh] flex flex-col transition-all animate-in zoom-in-95 duration-200 ${isDark
            ? 'bg-[#3a3228] border-white/30'
            : 'bg-[#d4c5b0] border-white/40'
          }`}
        onClick={(e) => e.stopPropagation()}
      >
        {(title || icon || showCloseButton) && (
          <div className="flex items-start justify-between p-4 md:p-6 pb-3 md:pb-4 flex-shrink-0 border-b border-white/10">
            <div className="flex items-center gap-3 flex-1">
              {icon && (
                <div className={`w-8 h-8 md:w-10 md:h-10 rounded-[10px] md:rounded-[12px] flex items-center justify-center shadow-lg border-2 flex-shrink-0 ${isDark
                    ? 'bg-gradient-to-br from-[#e8c571]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-[#e8c571]/50'
                    : 'bg-gradient-to-br from-[#c9983a]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-[#c9983a]/50'
                  }`}>
                  {icon}
                </div>
              )}
              {title && (
                <h3 className={`text-[16px] md:text-[18px] font-bold transition-colors ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>
                  {title}
                </h3>
              )}
            </div>
            {showCloseButton && (
              <button
                onClick={onClose}
                className={`p-2 rounded-[10px] transition-all hover:scale-110 flex-shrink-0 ${isDark
                    ? 'hover:bg-white/[0.1] text-[#e8c571] hover:text-[#f5d98a]'
                    : 'hover:bg-black/[0.05] text-[#8b6f3a] hover:text-[#c9983a]'
                  }`}
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>
        )}

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto p-4 md:p-6 scrollbar-custom">
          {children}
        </div>
        {footer && (
          <div className="flex-shrink-0 border-t border-white/10 p-4 md:p-6 pt-3 md:pt-4">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}

interface ModalFooterProps {
  children: ReactNode;
  className?: string;
}

export function ModalFooter({ children, className = '' }: ModalFooterProps) {
  return (
    <div className={`flex flex-col sm:flex-row items-stretch sm:items-center justify-end gap-2 sm:gap-3 mt-4 md:mt-6 ${className}`}>
      {children}
    </div>
  );
}

interface ModalButtonProps {
  children: ReactNode;
  onClick?: () => void;
  type?: 'button' | 'submit' | 'reset';
  variant?: 'primary' | 'secondary';
  className?: string;
}

export function ModalButton({
  children,
  onClick,
  type = 'button',
  variant = 'secondary',
  className = ''
}: ModalButtonProps) {
  const { theme } = useTheme();

  if (variant === 'primary') {
    return (
      <button
        type={type}
        onClick={onClick}
        className={`px-4 md:px-5 py-2.5 rounded-[10px] md:rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-medium text-[13px] md:text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all border border-white/10 hover:scale-[1.02] active:scale-100 flex items-center justify-center gap-2 touch-manipulation min-h-[44px] w-full sm:w-auto ${className}`}
      >
        {children}
      </button>
    );
  }

  return (
    <button
      type={type}
      onClick={onClick}
      className={`px-4 md:px-5 py-2.5 rounded-[10px] md:rounded-[12px] backdrop-blur-[20px] border font-medium text-[13px] md:text-[14px] transition-all hover:scale-[1.02] active:scale-100 touch-manipulation min-h-[44px] w-full sm:w-auto ${theme === 'dark'
          ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4] hover:bg-white/[0.12] active:bg-white/[0.15]'
          : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2] active:bg-white/[0.25]'
        } ${className}`}
    >
      {children}
    </button>
  );
}

interface ModalInputProps {
  label?: string;
  type?: string;
  value: string;
  onChange: (value: string) => void;
  onBlur?: () => void;
  placeholder?: string;
  required?: boolean;
  rows?: number;
  className?: string;
  error?: string | null;
}

export function ModalInput({
  label,
  type = 'text',
  value,
  onChange,
  onBlur,
  placeholder,
  required = false,
  rows,
  className = '',
  error
}: ModalInputProps) {
  const { theme } = useTheme();

  const isError = !!error;

  const inputClasses = `w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${isError
      ? theme === 'dark'
        ? 'bg-red-500/10 border-red-500/40 text-[#f5f5f5] placeholder-red-300/50 focus:border-red-500/60'
        : 'bg-red-500/5 border-red-500/40 text-[#2d2820] placeholder-red-700/50 focus:border-red-500/60'
      : theme === 'dark'
        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
    } ${className}`;

  return (
    <div>
      {label && (
        <label className={`block text-[13px] font-medium mb-2 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>
          {label}
          {required && <span className="text-[#c9983a] ml-1">*</span>}
        </label>
      )}
      {rows ? (
        <textarea
          rows={rows}
          required={required}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={onBlur}
          className={`${inputClasses} resize-none`}
          placeholder={placeholder}
        />
      ) : (
        <input
          type={type}
          required={required}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={onBlur}
          className={inputClasses}
          placeholder={placeholder}
        />
      )}
      {isError && (
        <p className={`text-[12px] mt-1.5 transition-colors ${theme === 'dark' ? 'text-red-400' : 'text-red-600'
          }`}>
          {error}
        </p>
      )}
    </div>
  );
}

interface ModalSelectProps {
  label?: string;
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
  required?: boolean;
  className?: string;
}

export function ModalSelect({
  label,
  value,
  onChange,
  options,
  required = false,
  className = '',
}: ModalSelectProps) {
  const { theme } = useTheme();
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find((opt) => opt.value === value);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <div className={`flex flex-col gap-1 relative ${className}`} ref={containerRef}>
      {label && (
        <label
          className={`block text-[13px] font-medium mb-2 transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}
        >
          {label} {required && <span className="text-[#c9983a] ml-1">*</span>}
        </label>
      )}

      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className={`w-full flex items-center justify-between px-4 py-3 rounded-[14px] backdrop-blur-[30px] border transition-all text-[14px] outline-none ${
          theme === 'dark'
            ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] focus:border-[#c9983a]/30'
            : 'bg-white/[0.15] border-white/25 text-[#2d2820] focus:border-[#c9983a]/30'
        } ${isOpen ? 'border-[#c9983a]' : ''}`}

      >
        <span>{selectedOption ? selectedOption.label : 'Select...'}</span>
        <ChevronDown 
          className={`w-4 h-4 text-amber-500 transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`} 
        />
      </button>

{/* Custom Dropdown Menu - FINAL THEME FIX */}
      {isOpen && (
        <div
          className={`absolute z-[100] w-full mt-[80px] max-h-60 overflow-auto rounded-[14px] border shadow-2xl backdrop-blur-xl animate-in fade-in zoom-in-95 duration-200 ${
            theme === 'dark' 
              ? 'bg-[#2d241d] border-[#c9983a]/20 shadow-black/40' // Dark Brown background + Gold border
              : 'bg-[#ede3d0] border-[#c9983a]/60 shadow-amber-900/20'
          }`}
        >
          <ul className="py-2">
            {options.map((option) => (
              <li
                key={option.value}
                onClick={() => {
                  onChange(option.value);
                  setIsOpen(false);
                }}
                className={`px-4 py-2.5 cursor-pointer text-[14px] transition-colors flex items-center justify-between ${
                  theme === 'dark'
                    ? value === option.value
                      ? 'bg-[#c9983a]/20 text-[#c9983a] font-bold' // Dark mode active
                      : 'text-[#e8dfd0] hover:bg-[#c9983a]/10'     // Dark mode hover (Cream text, gold bg)
                    : value === option.value
                      ? 'bg-[#c9983a]/30 text-[#8b6b2d] font-bold'
                      : 'text-[#5c4d3c] hover:bg-[#c9983a]/20'
                }`}
              >
                {option.label}
                {value === option.value && (
                  <div className="w-2 h-2 rounded-full bg-[#c9983a] shadow-[0_0_8px_#c9983a]" />
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}