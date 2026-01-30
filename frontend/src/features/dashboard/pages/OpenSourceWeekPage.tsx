import React, { useEffect, useMemo, useState } from 'react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { Calendar } from 'lucide-react';
import { getOpenSourceWeekEvents } from '../../../shared/api/client';

interface OpenSourceWeekPageProps {
  onEventClick: (id: string, name: string) => void;
}

export function OpenSourceWeekPage({ onEventClick }: OpenSourceWeekPageProps) {
  const { theme } = useTheme();

  const [isLoading, setIsLoading] = useState(true);
  const [events, setEvents] = useState<
    Array<{
      id: string;
      title: string;
      description: string | null;
      location: string | null;
      status: string;
      start_at: string;
      end_at: string;
    }>
  >([]);

  useEffect(() => {
    let mounted = true;
    const fetchEvents = async () => {
      try {
        setIsLoading(true);
        const res = await getOpenSourceWeekEvents();
        if (!mounted) return;
        setEvents(res.events || []);
      } catch (e) {
        if (!mounted) return;
        setEvents([]);
      } finally {
        if (!mounted) return;
        setIsLoading(false);
      }
    };
    fetchEvents();
    return () => {
      mounted = false;
    };
  }, []);

  const formattedEvents = useMemo(() => {
    const fmtDate = (iso: string) =>
      new Date(iso).toLocaleDateString(undefined, { day: '2-digit', month: 'short', year: 'numeric' });
    const fmtTime = (iso: string) =>
      new Date(iso).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    const fmtStatus = (s: string) => {
      if (s === 'upcoming') return 'Upcoming';
      if (s === 'running') return 'Running';
      if (s === 'completed') return 'Completed';
      return 'Draft';
    };
    return events.map((e) => ({
      ...e,
      startDate: fmtDate(e.start_at),
      endDate: fmtDate(e.end_at),
      startTime: fmtTime(e.start_at),
      endTime: fmtTime(e.end_at),
      statusLabel: fmtStatus(e.status),
    }));
  }, [events]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-center sm:items-start justify-between gap-4 sm:gap-0 text-center sm:text-left">
        <div>
          <h1 className={`text-[24px] sm:text-[32px] font-bold mb-2 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Open-Source Week</h1>
          <p className={`text-[14px] sm:text-[16px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>
            Gear-round Hack is a week for developers with focus on rewarding.
          </p>
        </div>
        <div className="w-16 h-16 sm:w-20 sm:h-20 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15 shrink-0">
          <Calendar className="w-8 h-8 sm:w-10 sm:h-10 text-white" />
        </div>
      </div>

      {/* Main Events */}
      <div className="space-y-5">
        {isLoading ? (
          <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 sm:p-8 shadow-[0_8px_32px_rgba(0,0,0,0.08)] ${
            theme === 'dark' ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
          }`}>
            <div className="animate-pulse space-y-6">
              <div className="flex flex-col sm:flex-row items-start justify-between gap-4">
                <div className="flex items-start space-x-4 w-full">
                  <div className={`w-12 h-12 sm:w-14 sm:h-14 rounded-[16px] shrink-0 ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                  <div className="space-y-3 w-full">
                    <div className={`h-6 w-3/4 sm:w-64 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                    <div className={`h-8 w-24 sm:w-28 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                  </div>
                </div>
                <div className={`h-10 w-full sm:w-48 rounded-[14px] ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-4 gap-4 sm:gap-8">
                {Array.from({ length: 4 }).map((_, i) => (
                  <div key={i} className="space-y-2">
                    <div className={`h-3 w-20 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                    <div className={`h-7 w-24 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : formattedEvents.length === 0 ? (
          <div className={`backdrop-blur-[40px] rounded-[24px] border p-8 sm:p-10 shadow-[0_8px_32px_rgba(0,0,0,0.08)] text-center ${
            theme === 'dark' ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
          }`}>
            <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15 mx-auto mb-4">
              <Calendar className="w-8 h-8 text-white" />
            </div>
            <h3 className={`text-[20px] font-bold mb-2 ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
              No Open-Source Week events yet
            </h3>
            <p className={`${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
              Once an admin creates an event, it will show up here.
            </p>
          </div>
        ) : (
          formattedEvents.map((event) => (
          <div
            key={event.id}
            onClick={() => onEventClick(event.id, event.title)}
            className={`backdrop-blur-[40px] rounded-[24px] border p-6 sm:p-8 shadow-[0_8px_32px_rgba(0,0,0,0.08)] transition-all cursor-pointer ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/10 hover:bg-white/[0.12] hover:shadow-[0_8px_24px_rgba(201,152,58,0.15)]'
                : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:shadow-[0_8px_24px_rgba(0,0,0,0.12)]'
            }`}
          >
            <div className="flex flex-col sm:flex-row items-start justify-between mb-6 gap-4 sm:gap-0">
              <div className="flex items-start space-x-4">
                <div className="w-12 h-12 sm:w-14 sm:h-14 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-md border border-white/10 shrink-0">
                  <Calendar className="w-6 h-6 sm:w-7 sm:h-7 text-white" />
                </div>
                <div>
                  <h3 className={`text-[18px] sm:text-[22px] font-bold mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>{event.title}</h3>
                  <span className={`px-3 py-1.5 rounded-[10px] text-[11px] sm:text-[12px] font-semibold ${
                    theme === 'dark'
                      ? 'bg-[#c9983a]/20 border border-[#c9983a]/40 text-[#e8c77f]'
                      : 'bg-[#c9983a]/15 border border-[#c9983a]/30 text-[#6d5530]'
                  }`}>
                    {event.statusLabel}
                  </span>
                </div>
              </div>
              <button className="w-full sm:w-auto px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[14px] font-semibold text-[13px] sm:text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.4)] transition-all border border-white/10">
                Join Event
              </button>
            </div>

            <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between pt-6 border-t border-white/10 gap-6 sm:gap-0">
              <div className="grid grid-cols-2 gap-8 w-full sm:w-auto">
                <div>
                  <div className={`text-[11px] sm:text-[12px] mb-1 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Start date</div>
                  <div className={`text-[14px] sm:text-[15px] font-semibold transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>{event.startDate}</div>
                  <div className={`text-[11px] sm:text-[12px] transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>{event.startTime}</div>
                </div>
                <div>
                  <div className={`text-[11px] sm:text-[12px] mb-1 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>End date</div>
                  <div className={`text-[14px] sm:text-[15px] font-semibold transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>{event.endDate}</div>
                  <div className={`text-[11px] sm:text-[12px] transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>{event.endTime}</div>
                </div>
              </div>
              <div className="w-full sm:w-auto">
                <div className={`text-[11px] sm:text-[12px] mb-1 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Location</div>
                <div className={`text-[14px] sm:text-[15px] font-semibold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.location || 'TBA'}</div>
              </div>
            </div>
          </div>
          ))
        )}
      </div>
    </div>
  );
}