import { ReactNode, useId } from 'react';
import { AlertCircle, CheckCircle } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

interface FormFieldProps {
  label: string;
  error?: string;
  success?: boolean;
  required?: boolean;
  disabled?: boolean;
  hint?: string;
  children: (props: {
    id: string;
    'aria-describedby': string | undefined;
    'aria-invalid': boolean;
    'aria-required': boolean;
    'aria-disabled': boolean;
    className: string;
  }) => ReactNode;
}

/**
 * FormField — wraps any input, textarea, or select with consistent
 * label, error, and success states meeting WCAG 2.1 AA.
 *
 * Usage:
 *   <FormField label="First Name" error={errors.firstName} required>
 *     {(fieldProps) => (
 *       <input type="text" value={value} onChange={...} {...fieldProps} />
 *     )}
 *   </FormField>
 */
export function FormField({
  label,
  error,
  success,
  required = false,
  disabled = false,
  hint,
  children,
}: FormFieldProps) {
  const { theme } = useTheme();
  const id = useId();
  const errorId = `${id}-error`;
  const hintId = `${id}-hint`;

  const hasError = Boolean(error);

  // Build the describedby list
  const describedBy = [
    hasError ? errorId : null,
    hint ? hintId : null,
  ]
    .filter(Boolean)
    .join(' ') || undefined;

  // Base input classes consistent with existing ProfileTab style
  const baseInputClass = [
    'w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border',
    'text-[14px] transition-all duration-150',
    // Focus ring — WCAG 2.1 AA 3:1 contrast
    'focus:outline-none focus-visible:ring-[3px]',
    hasError
      ? [
          'border-red-600 dark:border-red-500',
          'focus-visible:ring-red-600/20 dark:focus-visible:ring-red-400/20',
          theme === 'dark'
            ? 'bg-red-900/[0.08] text-[#f5efe5] placeholder-[#b8a898]'
            : 'bg-red-50/[0.15] text-[#2d2820] placeholder-[#7a6b5a]',
        ].join(' ')
      : success
        ? [
            'border-green-600 dark:border-green-500',
            'focus-visible:ring-[#c9983a]/35',
            theme === 'dark'
              ? 'bg-[#3d342c]/[0.4] text-[#f5efe5] placeholder-[#b8a898]'
              : 'bg-white/[0.15] text-[#2d2820] placeholder-[#7a6b5a]',
          ].join(' ')
        : [
            'focus-visible:ring-[#c9983a]/35 focus-visible:border-[#c9983a]/80',
            theme === 'dark'
              ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#b8a898]'
              : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]',
          ].join(' '),
    disabled ? 'opacity-40 cursor-not-allowed' : 'hover:border-[#c9983a]/40',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div className="flex flex-col gap-1.5">
      {/* Label */}
      <label
        htmlFor={id}
        className={`text-[14px] font-semibold transition-colors ${
          theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}
      >
        {label}
        {required && (
          <span className="text-red-500 ml-1" aria-hidden="true">*</span>
        )}
      </label>

      {/* Hint text */}
      {hint && (
        <p
          id={hintId}
          className={`text-[12px] transition-colors ${
            theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
          }`}
        >
          {hint}
        </p>
      )}

      {/* Input slot — rendered by parent */}
      <div className="relative">
        {children({
          id,
          'aria-describedby': describedBy,
          'aria-invalid': hasError,
          'aria-required': required,
          'aria-disabled': disabled,
          className: baseInputClass,
        })}

        {/* Success icon inside field */}
        {success && !hasError && (
          <CheckCircle
            className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-green-600 dark:text-green-400 pointer-events-none"
            aria-hidden="true"
          />
        )}
      </div>

      {/* Error message */}
      {hasError && (
        <p
          id={errorId}
          role="alert"
          aria-live="polite"
          className={`flex items-center gap-1.5 text-[12px] animate-in fade-in duration-150 ${
            theme === 'dark' ? 'text-red-400' : 'text-red-600'
          }`}
        >
          <AlertCircle className="w-4 h-4 flex-shrink-0" aria-hidden="true" />
          {error}
        </p>
      )}
    </div>
  );
}