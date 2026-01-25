import { ReactNode } from 'react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

interface NotificationSectionProps {
  title: string;
  children: ReactNode;
}

export function NotificationSection({ title, children }: NotificationSectionProps) {
  const { theme } = useTheme();

  return (
    <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
      theme === 'dark'
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/20'
    }`}>
      <h3 className={`text-[20px] font-bold mb-6 transition-colors ${
        theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
      }`}>{title}</h3>
      {children}
    </div>
  );
}