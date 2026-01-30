"use client";

import * as React from "react";
import { format } from "date-fns";
import { Calendar as CalendarIcon } from "lucide-react";
import { Calendar } from "../../../app/components/ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "../../../app/components/ui/popover";
import { useTheme } from "../../contexts/ThemeContext";
import { cn } from "../../../app/components/ui/utils";

interface DatePickerProps {
  label?: string;
  value: string; // YYYY-MM-DD format
  onChange: (value: string) => void;
  placeholder?: string;
  required?: boolean;
  className?: string;
  error?: string | null; // ADDED: Error message support
}

export function DatePicker({
  label,
  value,
  onChange,
  placeholder = "Pick a date",
  required = false,
  className = "",
  error // ADDED
}: DatePickerProps) {
  const { theme } = useTheme();
  const [open, setOpen] = React.useState(false);

  // ADDED: Check if there's an error
  const isError = !!error;

  // Parse the date value (YYYY-MM-DD format)
  // Parse as UTC to avoid timezone issues
  const date = React.useMemo(() => {
    if (!value) return undefined;
    try {
      const [year, month, day] = value.split('-').map(Number);
      if (isNaN(year) || isNaN(month) || isNaN(day)) return undefined;
      return new Date(Date.UTC(year, month - 1, day));
    } catch {
      return undefined;
    }
  }, [value]);

  // Handle date selection
  const handleSelect = (selectedDate: Date | undefined) => {
    if (selectedDate) {
      // Format as YYYY-MM-DD
      const formattedDate = format(selectedDate, "yyyy-MM-dd");
      onChange(formattedDate);
      setOpen(false);
    }
  };

  // Format date for display
  const displayValue = date ? format(date, "MMM dd, yyyy") : "";

  // UPDATED: Input styling matching ModalInput with error support
  const inputClasses = `w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] flex items-center justify-between ${
    isError
      ? theme === 'dark'
        ? 'bg-red-500/10 border-red-500/40 text-[#f5f5f5] placeholder-red-300/50 focus:border-red-500/60'
        : 'bg-red-500/5 border-red-500/40 text-[#2d2820] placeholder-red-700/50 focus:border-red-500/60'
      : theme === 'dark'
        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
  } ${className}`;

  // Popover content styling for theme - using theme colors
  const popoverContentClasses = theme === 'dark'
    ? 'bg-[#1a1512] border-[#b8a898]/30 backdrop-blur-[30px] text-[#f5f5f5]'
    : 'bg-white/[0.4] border-[#c9983a]/20 backdrop-blur-[30px] text-[#2d2820]';

  // Calendar styling for theme - using theme colors consistently
  const calendarClassNames = {
    months: "flex flex-col sm:flex-row gap-2",
    month: "flex flex-col gap-4",
    caption: "flex justify-center pt-1 relative items-center w-full",
    caption_label: `text-sm font-medium ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`,
    nav: "flex items-center gap-1",
    nav_button: cn(
      "h-7 w-7 bg-transparent p-0 opacity-50 hover:opacity-100 rounded-md border transition-colors",
      theme === 'dark'
        ? 'border-[#b8a898]/40 text-[#f5f5f5] hover:bg-[#b8a898]/20 hover:border-[#c9983a]/50'
        : 'border-[#c9983a]/20 text-[#2d2820] hover:bg-[#c9983a]/10 hover:border-[#c9983a]/30'
    ),
    nav_button_previous: "absolute left-1",
    nav_button_next: "absolute right-1",
    table: "w-full border-collapse space-x-1",
    head_row: "flex",
    head_cell: cn(
      "rounded-md w-8 font-normal text-[0.8rem]",
      theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
    ),
    row: "flex w-full mt-2",
    cell: cn(
      "relative p-0 text-center text-sm focus-within:relative focus-within:z-20",
      "[&:has([aria-selected])]:rounded-md"
    ),
    day: cn(
      "h-8 w-8 p-0 font-normal rounded-md transition-colors",
      theme === 'dark'
        ? 'text-[#f5f5f5] hover:bg-[#b8a898]/15 hover:text-[#f5f5f5]'
        : 'text-[#2d2820] hover:bg-[#c9983a]/15 hover:text-[#2d2820]',
      "aria-selected:opacity-100"
    ),
    day_selected: cn(
      "bg-[#c9983a] text-white hover:bg-[#c9983a]/90 focus:bg-[#c9983a] focus:text-white",
      "hover:text-white focus:text-white"
    ),
    day_today: cn(
      theme === 'dark' 
        ? 'bg-[#b8a898]/15 text-[#f5f5f5] border border-[#c9983a]/40' 
        : 'bg-[#c9983a]/15 text-[#2d2820] border border-[#c9983a]/30'
    ),
    day_outside: cn(
      theme === 'dark' ? 'text-[#7a7a7a]' : 'text-[#b8a898]'
    ),
    day_disabled: cn(
      theme === 'dark' ? 'text-[#7a7a7a] opacity-50' : 'text-[#b8a898] opacity-50'
    ),
    day_hidden: "invisible",
  };

  return (
    <div>
      {label && (
        <label className={`block text-[13px] font-medium mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}>
          {label}
          {required && <span className="text-[#c9983a] ml-1">*</span>}
        </label>
      )}
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button
            type="button"
            className={inputClasses}
            onClick={() => setOpen(!open)}
          >
            <span className={displayValue ? "" : `text-[14px] ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
              {displayValue || placeholder}
            </span>
            <CalendarIcon className={`h-4 w-4 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`} />
          </button>
        </PopoverTrigger>
        <PopoverContent className={cn("w-auto p-0 z-[10001]", popoverContentClasses)} align="start">
          <Calendar
            mode="single"
            selected={date}
            onSelect={handleSelect}
            initialFocus
            classNames={calendarClassNames}
          />
        </PopoverContent>
      </Popover>
      
      {/* ADDED: Error message display */}
      {isError && (
        <p className={`text-[12px] mt-1.5 transition-colors ${
          theme === 'dark' ? 'text-red-400' : 'text-red-600'
        }`}>
          {error}
        </p>
      )}
    </div>
  );
}