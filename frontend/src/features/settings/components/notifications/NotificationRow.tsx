import { ToggleSwitch } from '../shared/ToggleSwitch';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { TimestampDisplay } from '../../../../shared/components/TimestampDisplay';

interface NotificationRowProps {
  title: string;
  description: string;
  emailEnabled: boolean;
  weeklyEnabled: boolean;
  onEmailChange: (value: boolean) => void;
  onWeeklyChange: (value: boolean) => void;
  showBorder?: boolean;
  timestamp?: string | Date | number;
}

export function NotificationRow({
  title,
  description,
  emailEnabled,
  weeklyEnabled,
  onEmailChange,
  onWeeklyChange,
  showBorder = true,
  timestamp,
}: NotificationRowProps) {
  const { theme } = useTheme();

  return (
    <div className={`grid grid-cols-[1fr_200px_220px] gap-4 items-center py-5 ${showBorder ? 'border-b border-white/10' : ''}`}>
      <div>
        <div className="flex items-center justify-between gap-2 mb-1">
          <div className={`text-[15px] font-semibold transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>{title}</div>
          {timestamp && (
            <TimestampDisplay
              timestamp={timestamp}
              className={`text-[11px] ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}
            />
          )}
        </div>
        <div className={`text-[13px] transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>{description}</div>
      </div>
      <div className="flex justify-center">
        <ToggleSwitch enabled={emailEnabled} onChange={onEmailChange} />
      </div>
      <div className="flex justify-center">
        <ToggleSwitch enabled={weeklyEnabled} onChange={onWeeklyChange} />
      </div>
    </div>
  );
}