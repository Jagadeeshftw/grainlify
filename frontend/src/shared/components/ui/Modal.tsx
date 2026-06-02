import React, { ReactNode, useState, useRef, useEffect } from 'react';
import { X, ChevronDown, Check } from 'lucide-react';
import * as Select from '@radix-ui/react-select';
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
  dimBackdrop?: boolean;
}

const widthClasses = {
  sm: 'w-[95vw] sm:w-[400px]',
  md: 'w-[95vw] sm:w-[500px]',
  lg: 'w-[95vw] sm:w-[550px]',
  xl: 'w-[95vw] sm:w-[650px]'
};

const MODAL_STACK: string[] = [];
const MODAL_BASE_Z_INDEX = 10000;

const getFocusableElements = (container: HTMLElement | null): HTMLElement[] => {
  if (!container) return [];
  const focusable = Array.from(
    container.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    ),
  ).filter(
    (element) =>
      !element.hasAttribute('disabled') &&
      element.getAttribute('aria-hidden') !== 'true' &&
      element.tabIndex >= 0 &&
      !element.hidden &&
      element.offsetParent !== null,
  );
  return focusable;
};

const removeModalFromStack = (id: string) => {
  const index = MODAL_STACK.indexOf(id);
  if (index !== -1) {
    MODAL_STACK.splice(index, 1);
  }
};

const getBackdropOpacity = (stackIndex: number) => {
  return Math.min(0.62, 0.5 + Math.max(0, stackIndex - 1) * 0.04);
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
  footer,
  dimBackdrop = true
}: ModalProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const [stackIndex, setStackIndex] = React.useState(1);
  const modalRef = React.useRef<HTMLDivElement | null>(null);
  const idRef = React.useRef(`modal-${Math.random().toString(36).slice(2)}`);
  const titleIdRef = React.useRef(`modal-title-${Math.random().toString(36).slice(2)}`);
  const lastFocusedElementRef = React.useRef<HTMLElement | null>(null);

  React.useEffect(() => {
    if (!isOpen) return;

    const previouslyFocused = document.activeElement;
    lastFocusedElementRef.current = previouslyFocused instanceof HTMLElement ? previouslyFocused : null;

    if (!MODAL_STACK.includes(idRef.current)) {
      MODAL_STACK.push(idRef.current);
    }

    setStackIndex(MODAL_STACK.length);
    document.body.classList.add('modal-open');

    const focusableElements = getFocusableElements(modalRef.current);
    if (focusableElements.length > 0) {
      focusableElements[0].focus();
    } else {
      modalRef.current?.focus();
    }

    return () => {
      removeModalFromStack(idRef.current);
      if (MODAL_STACK.length === 0) {
        document.body.classList.remove('modal-open');
      }
      lastFocusedElementRef.current?.focus();
    };
  }, [isOpen]);

  const handleKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
      return;
    }

    if (event.key !== 'Tab') {
      return;
    }

    const focusableElements = getFocusableElements(modalRef.current);
    if (focusableElements.length === 0) {
      event.preventDefault();
      return;
    }

    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];
    const activeElement = document.activeElement as HTMLElement | null;

    if (event.shiftKey) {
      if (activeElement === firstElement || activeElement === modalRef.current) {
        event.preventDefault();
        lastElement.focus();
      }
    } else {
      if (activeElement === lastElement) {
        event.preventDefault();
        firstElement.focus();
      }
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className={`fixed inset-0 ${dimBackdrop ? 'flex items-center justify-center' : 'flex items-center justify-center'} `}
      style={{
        zIndex: MODAL_BASE_Z_INDEX + stackIndex * 20,
        backgroundColor: dimBackdrop ? `rgba(0, 0, 0, ${getBackdropOpacity(stackIndex)})` : 'transparent'
      }}
      onClick={onClose}
      role="presentation"
    >
      <div
        ref={modalRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? titleIdRef.current : undefined}
        className={`rounded-[16px] md:rounded-[24px] border-2 shadow-[0_20px_60px_rgba(0,0,0,0.3)] ${widthClasses[width]} max-w-[95vw] sm:max-w-[90vw] ${maxHeight ? 'max-h-[90vh]' : ''} flex flex-col transition-all duration-200 animate-in zoom-in-95 ${isDark
          ? 'bg-[#3a3228] border-white/30'
          : 'bg-[#d4c5b0] border-white/40'
          }`}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
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
                <h3 id={titleIdRef.current} className={`text-[16px] md:text-[18px] font-bold transition-colors ${isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>
                  {title}
                </h3>
              )}
            </div>
            {showCloseButton && (
              <button
                onClick={onClose}
                className={`p-2 rounded-[10px] transition-all hover:scale-110 flex-shrink-0 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-600 ${isDark
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
        <div className={`flex-1 overflow-y-auto p-4 md:p-6 scrollbar-custom ${maxHeight ? '' : 'max-h-[calc(100vh-200px)]'}`}>
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
  disabled?: boolean; // ADDED
}

export function ModalButton({
  children,
  onClick,
  type = 'button',
  variant = 'secondary',
  className = '',
  disabled = false // ADDED
}: ModalButtonProps) {
  const { theme } = useTheme();

  if (variant === 'primary') {
    return (
      <button
        type={type}
        onClick={onClick}
        disabled={disabled}
        className={`px-4 md:px-5 py-2.5 rounded-[10px] md:rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-medium text-[13px] md:text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all border border-white/10 hover:scale-[1.02] active:scale-100 flex items-center justify-center gap-2 touch-manipulation min-h-[44px] w-full sm:w-auto ${disabled ? 'opacity-50 cursor-not-allowed' : ''} ${className}`}
      >
        {children}
      </button>
    );
  }

  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className={`px-4 md:px-5 py-2.5 rounded-[10px] md:rounded-[12px] backdrop-blur-[20px] border font-medium text-[13px] md:text-[14px] transition-all hover:scale-[1.02] active:scale-100 touch-manipulation min-h-[44px] w-full sm:w-auto ${disabled ? 'opacity-50 cursor-not-allowed' : ''} ${theme === 'dark'
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
  const isDark = theme === 'dark';

  return (
    <div className={`flex flex-col gap-1 relative ${className}`}>
      {label && (
        <label className={`block text-[13px] font-medium mb-2 transition-colors ${
          isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}>
          {label}
          {required && <span className="text-[#c9983a] ml-1">*</span>}
        </label>
      )}
      
      <Select.Root value={value} onValueChange={onChange} required={required}>
        <Select.Trigger 
          className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] flex items-center justify-between group ${
            isDark
              ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] hover:bg-white/[0.12] data-[state=open]:border-[#c9983a]/50'
              : 'bg-white/[0.15] border-white/25 text-[#2d2820] hover:bg-white/[0.2] data-[state=open]:border-[#c9983a]/50'
          }`}
        >
          <Select.Value placeholder="Select an option" />
          <Select.Icon>
            <ChevronDown className={`w-4 h-4 text-amber-500 transition-transform duration-200 group-data-[state=open]:rotate-180`} />
          </Select.Icon>
        </Select.Trigger>

        <Select.Portal>
          <Select.Content 
            className={`z-[10001] min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-[14px] border shadow-[0_10px_40px_rgba(0,0,0,0.2)] backdrop-blur-[30px] animate-in fade-in zoom-in-95 duration-200 ${
              isDark
                ? 'bg-[#2d241d] border-[#c9983a]/20 shadow-black/40'
                : 'bg-[#ede3d0] border-[#c9983a]/20 shadow-amber-900/10'
            }`}
            position="popper"
            sideOffset={8}
          >
            <Select.Viewport className="p-1">
              {options.map((option) => (
                <Select.Item
                  key={option.value}
                  value={option.value}
                  className={`relative flex w-full cursor-default select-none items-center rounded-[10px] py-2.5 pl-3 pr-8 text-[14px] outline-none transition-colors data-[disabled]:pointer-events-none data-[disabled]:opacity-50 ${
                    isDark
                      ? 'text-[#d4d4d4] focus:bg-white/[0.08] focus:text-[#f5f5f5] data-[state=checked]:bg-white/[0.08] data-[state=checked]:text-[#f5f5f5]'
                      : 'text-[#7a6b5a] focus:bg-black/[0.05] focus:text-[#2d2820] data-[state=checked]:bg-black/[0.05] data-[state=checked]:text-[#2d2820]'
                  }`}
                >
                  <Select.ItemText>{option.label}</Select.ItemText>
                  <Select.ItemIndicator className="absolute right-2.5 flex items-center justify-center text-[#c9983a]">
                    <Check className="h-4 w-4" />
                  </Select.ItemIndicator>
                </Select.Item>
              ))}
            </Select.Viewport>
          </Select.Content>
        </Select.Portal>
      </Select.Root>
    </div>
  );
}
