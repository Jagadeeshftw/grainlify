import { useEffect, useMemo, useState } from 'react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { ArrowLeft, Calendar, MapPin } from 'lucide-react';
import { getOpenSourceWeekEvent } from '../../../shared/api/client';

interface OpenSourceWeekDetailPageProps {
  eventId: string;
  eventName: string;
  onBack: () => void;
}

export function OpenSourceWeekDetailPage({ eventId, eventName, onBack }: OpenSourceWeekDetailPageProps) {
  const { theme } = useTheme();
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [event, setEvent] = useState<{
    id: string;
    title: string;
    description: string | null;
    location: string | null;
    status: string;
    start_at: string;
    end_at: string;
  } | null>(null);

  useEffect(() => {
    let mounted = true;
    const fetchEvent = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const res = await getOpenSourceWeekEvent(eventId);
        if (!mounted) return;
        setEvent(res.event);
      } catch (e) {
        if (!mounted) return;
        setEvent(null);
        setError(e instanceof Error ? e.message : 'Failed to load event');
      } finally {
        if (!mounted) return;
        setIsLoading(false);
      }
    };
    fetchEvent();
    return () => {
      mounted = false;
    };
  }, [eventId]);

  const view = useMemo(() => {
    const title = event?.title || eventName;
    const start = event ? new Date(event.start_at) : null;
    const end = event ? new Date(event.end_at) : null;
    return {
      title,
      location: event?.location || 'TBA',
      startDate: start ? start.toLocaleDateString(undefined, { day: '2-digit', month: 'short', year: 'numeric' }) : '—',
      startTime: start ? start.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }) : '—',
      endDate: end ? end.toLocaleDateString(undefined, { day: '2-digit', month: 'short', year: 'numeric' }) : '—',
      endTime: end ? end.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }) : '—',
      description: event?.description || 'Details will appear here once the admin configures this event.',
    };
  }, [event, eventName]);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <button
          onClick={onBack}
          className={`flex items-center gap-2 px-4 py-2 rounded-[12px] backdrop-blur-[30px] border transition-all ${
            theme === 'dark'
              ? 'bg-white/[0.08] border-white/10 text-[#f5f5f5] hover:bg-white/[0.12]'
              : 'bg-white/[0.15] border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
          }`}
        >
          <ArrowLeft className="w-4 h-4" />
          <span className="text-[14px] font-medium">Back to Open-Source Week</span>
        </button>
        <div />
      </div>

      {isLoading ? (
        <div
          className={`backdrop-blur-[40px] rounded-[20px] border p-8 transition-colors animate-pulse ${
            theme === 'dark' ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
          }`}
        >
          <div className={`h-6 w-60 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
          <div className={`h-4 w-96 rounded mt-4 ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
        </div>
      ) : error || !event ? (
        <div
          className={`backdrop-blur-[40px] rounded-[20px] border p-8 transition-colors ${
            theme === 'dark'
              ? 'bg-white/[0.08] border-white/10 text-[#d4d4d4]'
              : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
          }`}
        >
          <p className="text-[14px]">{error || 'Event not found.'}</p>
                    </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-6 gap-6">
          <div className="lg:col-span-2 space-y-6">
            <div
              className={`backdrop-blur-[40px] rounded-[20px] border p-6 transition-colors ${
                theme === 'dark' ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
              }`}
            >
              <div className="flex items-center gap-3">
                <div className="w-12 h-12 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-lg border border-white/20">
                  <Calendar className="w-6 h-6 text-white" />
                  </div>
                <h1 className={`text-[18px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                  {view.title}
                </h1>
              </div>
            </div>

            <div
                          className={`backdrop-blur-[40px] rounded-[20px] border p-6 transition-colors ${
                theme === 'dark' ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
              }`}
            >
              <h3 className={`text-[11px] font-semibold mb-4 uppercase transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                Date
                              </h3>
              <div className="space-y-4">
                <div>
                  <div className={`text-[16px] font-bold mb-1 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                    {view.startDate}
                  </div>
                  <div className={`text-[12px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                    {view.startTime}
                  </div>
                </div>
                <div className={`h-px ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                <div>
                  <div className={`text-[16px] font-bold mb-1 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                    {view.endDate}
                            </div>
                  <div className={`text-[12px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                    {view.endTime}
                        </div>
                      </div>
                    </div>
                </div>

            <div
              className={`backdrop-blur-[40px] rounded-[20px] border p-6 transition-colors ${
                theme === 'dark' ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
              }`}
            >
              <h3 className={`text-[11px] font-semibold mb-3 uppercase transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                Location
                              </h3>
              <div className={`text-[16px] font-medium transition-colors flex items-center gap-2 ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                <MapPin className="w-4 h-4 text-[#c9983a]" />
                <span>{view.location}</span>
                          </div>
                        </div>
              </div>

          <div className="lg:col-span-4 space-y-6">
            <div
              className={`backdrop-blur-[40px] rounded-[20px] border-2 p-8 transition-colors ${
                theme === 'dark' ? 'bg-white/[0.05] border-[#c9983a]/40' : 'bg-white/[0.1] border-[#c9983a]/40'
              }`}
            >
              <h2 className={`text-[18px] font-bold mb-4 transition-colors ${theme === 'dark' ? 'text-[#e8c77f]' : 'text-[#6d5530]'}`}>
                Overview
              </h2>
              <div className={`p-4 rounded-[14px] backdrop-blur-[30px] border ${
                theme === 'dark' ? 'bg-white/[0.05] border-white/10' : 'bg-white/[0.1] border-white/25'
              }`}>
                <p className={`text-[13px] leading-relaxed transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                  {view.description}
                </p>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}