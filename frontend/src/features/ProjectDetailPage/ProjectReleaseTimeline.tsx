import React, { useRef, useState } from 'react';
import { motion } from 'motion';
import { 
  CheckCircle2, 
  XCircle, 
  PlayCircle, 
  Calendar, 
  Coins, 
  ExternalLink
} from 'lucide-react';
import { useTheme } from '../../shared/contexts/ThemeContext';
import { useReducedMotion } from '../../shared/hooks/useReducedMotion';
import { Popover, PopoverContent, PopoverTrigger } from '../../app/components/ui/popover';

export interface ProgramReleaseSchedule {
  schedule_id: number;
  recipient: string;
  amount: string | number;
  release_timestamp: number; // Unix timestamp in seconds
  released: boolean;
  released_at: number | null;
  released_by: string | null;
  tx_hash?: string | null;
}

export type MilestoneState = 'released' | 'unlocked' | 'upcoming' | 'skipped';

interface ProjectReleaseTimelineProps {
  milestones: ProgramReleaseSchedule[];
  currentTime?: number;
}

export function ProjectReleaseTimeline({ 
  milestones, 
  currentTime = Math.floor(Date.now() / 1000) 
}: ProjectReleaseTimelineProps) {
  const { theme } = useTheme();
  const prefersReduced = useReducedMotion();
  const [activePopoverIndex, setActivePopoverIndex] = useState<number | null>(null);
  
  // Track button refs for focus navigation
  const nodeRefs = useRef<(HTMLButtonElement | null)[]>([]);

  if (!milestones || milestones.length === 0) {
    return (
      <div className={`p-6 rounded-[24px] border text-center ${
        theme === 'dark' 
          ? 'bg-white/[0.05] border-white/15 text-[#d4d4d4]' 
          : 'bg-white/[0.08] border-white/20 text-[#7a6b5a]'
      }`}>
        <p className="text-[14px] font-semibold">No release schedules configured for this program.</p>
      </div>
    );
  }

  // Derive state for each milestone
  const getMilestoneState = (milestone: ProgramReleaseSchedule, idx: number): MilestoneState => {
    if (milestone.released) {
      return 'released';
    }
    if (currentTime >= milestone.release_timestamp) {
      // Bypassed/Skipped logic: if a later milestone is already released
      const hasLaterReleased = milestones.slice(idx + 1).some(m => m.released);
      if (hasLaterReleased) {
        return 'skipped';
      }
      return 'unlocked';
    }
    return 'upcoming';
  };

  const getStatusColorClass = (state: MilestoneState) => {
    switch (state) {
      case 'released':
        return theme === 'dark' 
          ? { text: 'text-[#c9983a]', border: 'border-[#c9983a]', bg: 'bg-[#c9983a]/10', fill: 'bg-[#c9983a]' }
          : { text: 'text-[#b8872f]', border: 'border-[#b8872f]', bg: 'bg-[#b8872f]/10', fill: 'bg-[#b8872f]' };
      case 'unlocked':
        return theme === 'dark'
          ? { text: 'text-[#22c55e]', border: 'border-[#22c55e]', bg: 'bg-[#22c55e]/10', fill: 'bg-[#22c55e]' }
          : { text: 'text-[#16a34a]', border: 'border-[#16a34a]', bg: 'bg-[#16a34a]/10', fill: 'bg-[#16a34a]' };
      case 'skipped':
        return theme === 'dark'
          ? { text: 'text-[#ef4444]', border: 'border-[#ef4444]', bg: 'bg-[#ef4444]/10', fill: 'bg-[#ef4444]' }
          : { text: 'text-[#dc2626]', border: 'border-[#dc2626]', bg: 'bg-[#dc2626]/10', fill: 'bg-[#dc2626]' };
      case 'upcoming':
      default:
        return theme === 'dark'
          ? { text: 'text-[#b8a898]', border: 'border-white/20', bg: 'bg-white/[0.05]', fill: 'bg-[#b8a898]' }
          : { text: 'text-[#7a6b5a]', border: 'border-black/15', bg: 'bg-black/[0.05]', fill: 'bg-[#7a6b5a]' };
    }
  };

  const getStatusIcon = (state: MilestoneState, className = "w-5 h-5") => {
    switch (state) {
      case 'released':
        return <CheckCircle2 className={className} />;
      case 'unlocked':
        return <PlayCircle className={className} />;
      case 'skipped':
        return <XCircle className={className} />;
      case 'upcoming':
      default:
        return <div className={`${className} rounded-full border-2 border-current`} />;
    }
  };

  // Convert raw amount to human readable (Stellar 7 decimal places)
  const formatAmount = (amount: string | number) => {
    const rawVal = typeof amount === 'string' ? parseFloat(amount) : amount;
    return (rawVal / 1e7).toLocaleString(undefined, { 
      minimumFractionDigits: 2, 
      maximumFractionDigits: 7 
    });
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      timeZoneName: 'short'
    });
  };

  // Custom keyboard focus management
  const handleKeyDown = (e: React.KeyboardEvent, index: number) => {
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
      e.preventDefault();
      const nextIndex = (index + 1) % milestones.length;
      nodeRefs.current[nextIndex]?.focus();
    } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
      e.preventDefault();
      const prevIndex = (index - 1 + milestones.length) % milestones.length;
      nodeRefs.current[prevIndex]?.focus();
    }
  };

  // Animation variants configuration
  const trackVariants = {
    hidden: { scaleX: 0 },
    visible: { 
      scaleX: 1, 
      transition: { duration: prefersReduced ? 0 : 0.8, ease: 'easeOut' } 
    }
  };

  const verticalTrackVariants = {
    hidden: { scaleY: 0 },
    visible: { 
      scaleY: 1, 
      transition: { duration: prefersReduced ? 0 : 0.8, ease: 'easeOut' } 
    }
  };

  const nodeVariants = {
    hidden: { opacity: 0, scale: prefersReduced ? 1 : 0.5, y: prefersReduced ? 0 : 20 },
    visible: (idx: number) => ({
      opacity: 1,
      scale: 1,
      y: 0,
      transition: {
        delay: prefersReduced ? 0 : idx * 0.1,
        duration: prefersReduced ? 0 : 0.4,
        type: 'spring',
        stiffness: 120,
        damping: 12
      }
    })
  };

  // Highlight filled track up to the last released node
  const lastReleasedIdx = milestones.map(m => m.released).lastIndexOf(true);
  const percentFilled = milestones.length > 1 
    ? (lastReleasedIdx >= 0 ? (lastReleasedIdx / (milestones.length - 1)) * 100 : 0)
    : (lastReleasedIdx >= 0 ? 100 : 0);

  return (
    <div 
      className={`backdrop-blur-[40px] rounded-[24px] border p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}
    >
      <div className="flex items-center justify-between mb-8">
        <h2 className={`text-[18px] font-bold flex items-center gap-2 transition-colors ${
          theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
        }`}>
          <span className="text-[#c9983a]">✦</span>
          Milestone Release Schedule
        </h2>
        <div className="flex items-center gap-4 text-[12px] font-semibold">
          <div className="flex items-center gap-1.5">
            <span className="w-2.5 h-2.5 rounded-full bg-[#c9983a]" />
            <span className={theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}>Released</span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="w-2.5 h-2.5 rounded-full bg-[#22c55e]" />
            <span className={theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}>Unlocked</span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="w-2.5 h-2.5 rounded-full bg-[#ef4444]" />
            <span className={theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}>Skipped</span>
          </div>
        </div>
      </div>

      {/* Desktop Layout - Horizontal Scrolling */}
      <div 
        className="hidden md:block relative px-8 py-12"
        role="list"
        aria-label="Milestone Release Schedule Timeline"
      >
        {/* Connection Track Line */}
        <div className="absolute top-[84px] left-16 right-16 h-1 rounded bg-white/[0.1] overflow-hidden">
          <motion.div 
            className="h-full bg-gradient-to-r from-[#c9983a] to-[#d4af37]"
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-100px" }}
            variants={trackVariants}
            style={{ width: `${percentFilled}%`, transformOrigin: 'left' }}
          />
        </div>

        {/* Nodes list */}
        <div className="flex justify-between items-start relative z-10">
          {milestones.map((milestone, idx) => {
            const state = getMilestoneState(milestone, idx);
            const style = getStatusColorClass(state);
            const isLast = idx === milestones.length - 1;

            return (
              <motion.div
                key={milestone.schedule_id}
                role="listitem"
                aria-label={`Milestone ${milestone.schedule_id} - ${state} - ${formatAmount(milestone.amount)} USDC`}
                custom={idx}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true }}
                variants={nodeVariants}
                className="flex flex-col items-center text-center w-36 group"
              >
                <Popover 
                  open={activePopoverIndex === idx} 
                  onOpenChange={(open) => setActivePopoverIndex(open ? idx : null)}
                >
                  <PopoverTrigger asChild>
                    <button
                      ref={(el) => (nodeRefs.current[idx] = el)}
                      onKeyDown={(e) => handleKeyDown(e, idx)}
                      tabIndex={0}
                      aria-expanded={activePopoverIndex === idx}
                      aria-haspopup="dialog"
                      aria-controls={`popover-desktop-${milestone.schedule_id}`}
                      className={`relative flex items-center justify-center w-12 h-12 rounded-full border-2 ${style.border} ${style.bg} ${style.text} cursor-pointer hover:scale-105 active:scale-95 transition-all outline-none focus:ring-2 focus:ring-[#c9983a] focus:ring-offset-2 focus:ring-offset-transparent`}
                    >
                      {getStatusIcon(state, "w-6 h-6")}
                    </button>
                  </PopoverTrigger>

                  <PopoverContent
                    id={`popover-desktop-${milestone.schedule_id}`}
                    align="center"
                    side="bottom"
                    sideOffset={12}
                    className={`w-72 p-5 rounded-[18px] border backdrop-blur-[40px] z-[999] shadow-2xl transition-all ${
                      theme === 'dark' 
                        ? 'bg-[#1e1914] border-white/15 text-[#f5f5f5]' 
                        : 'bg-[#fcfbf9] border-[#e2d6c1] text-[#2d2820]'
                    }`}
                  >
                    <div className="space-y-4">
                      {/* Popover Header */}
                      <div className="flex items-center justify-between pb-2 border-b border-white/10">
                        <span className="text-[12px] font-bold tracking-wider uppercase opacity-70">
                          Milestone #{milestone.schedule_id}
                        </span>
                        <span className={`px-2 py-0.5 rounded-[6px] text-[11px] font-extrabold uppercase tracking-wide border ${style.border} ${style.bg} ${style.text}`}>
                          {state}
                        </span>
                      </div>

                      {/* Popover Body */}
                      <div className="space-y-3">
                        <div className="flex items-center gap-2">
                          <div className={`p-2 rounded-lg ${style.bg} ${style.text}`}>
                            <Coins className="w-4 h-4" />
                          </div>
                          <div>
                            <span className="block text-[11px] opacity-60 font-semibold leading-none">Amount</span>
                            <span className="text-[16px] font-bold tracking-tight">
                              {formatAmount(milestone.amount)} USDC
                            </span>
                          </div>
                        </div>

                        <div className="flex items-center gap-2">
                          <div className={`p-2 rounded-lg ${style.bg} ${style.text}`}>
                            <Calendar className="w-4 h-4" />
                          </div>
                          <div>
                            <span className="block text-[11px] opacity-60 font-semibold leading-none">Release Date</span>
                            <span className="text-[12px] font-semibold">
                              {formatDate(milestone.release_timestamp)}
                            </span>
                          </div>
                        </div>
                      </div>

                      {/* Explorer Link */}
                      {milestone.tx_hash ? (
                        <a
                          href={`https://stellar.expert/explorer/testnet/tx/${milestone.tx_hash}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className={`flex items-center justify-center gap-1.5 w-full py-2 px-3 rounded-xl border text-[12px] font-bold text-center transition-all ${
                            theme === 'dark'
                              ? 'bg-white/[0.05] hover:bg-white/[0.1] border-white/15 text-[#f5f5c5]'
                              : 'bg-[#f5ebd6] hover:bg-[#ebdcb9] border-[#e2d6c1] text-[#8b6f3a]'
                          }`}
                        >
                          <ExternalLink className="w-3.5 h-3.5" />
                          View in Explorer
                        </a>
                      ) : (
                        <a
                          href={`https://stellar.expert/explorer/testnet/account/${milestone.recipient}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className={`flex items-center justify-center gap-1.5 w-full py-2 px-3 rounded-xl border text-[12px] font-bold text-center transition-all ${
                            theme === 'dark'
                              ? 'bg-white/[0.05] hover:bg-white/[0.1] border-white/15 text-[#f5c563]'
                              : 'bg-[#f5ebd6] hover:bg-[#ebdcb9] border-[#e2d6c1] text-[#b8872f]'
                          }`}
                        >
                          <ExternalLink className="w-3.5 h-3.5" />
                          View Recipient Address
                        </a>
                      )}
                    </div>
                  </PopoverContent>
                </Popover>

                {/* Sub-node Labels */}
                <div className="mt-3">
                  <span className={`block text-[13px] font-bold transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>
                    Milestone #{milestone.schedule_id}
                  </span>
                  <span className="block text-[12px] font-bold text-[#c9983a] mt-0.5">
                    {formatAmount(milestone.amount)} USDC
                  </span>
                  <span className={`block text-[11px] font-semibold mt-1 transition-colors ${
                    theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                    {new Date(milestone.release_timestamp * 1000).toLocaleDateString(undefined, { 
                      month: 'short', 
                      day: 'numeric' 
                    })}
                  </span>
                </div>
              </motion.div>
            );
          })}
        </div>
      </div>

      {/* Mobile Layout - Vertical Timeline */}
      <div 
        className="block md:hidden relative pl-8 py-4"
        role="list"
        aria-label="Milestone Release Schedule Timeline"
      >
        {/* Mobile Connector Line */}
        <div className="absolute top-2 bottom-2 left-3 w-1 rounded bg-white/[0.1] overflow-hidden">
          <motion.div 
            className="w-full bg-gradient-to-b from-[#c9983a] to-[#d4af37]"
            initial="hidden"
            whileInView="visible"
            viewport={{ once: true, margin: "-50px" }}
            variants={verticalTrackVariants}
            style={{ height: `${percentFilled}%`, transformOrigin: 'top' }}
          />
        </div>

        {/* Vertical items */}
        <div className="space-y-8">
          {milestones.map((milestone, idx) => {
            const state = getMilestoneState(milestone, idx);
            const style = getStatusColorClass(state);

            return (
              <motion.div
                key={milestone.schedule_id}
                role="listitem"
                aria-label={`Milestone ${milestone.schedule_id} - ${state} - ${formatAmount(milestone.amount)} USDC`}
                custom={idx}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true }}
                variants={nodeVariants}
                className="flex items-start gap-4 relative"
              >
                <Popover 
                  open={activePopoverIndex === idx} 
                  onOpenChange={(open) => setActivePopoverIndex(open ? idx : null)}
                >
                  {/* Trigger Pin */}
                  <PopoverTrigger asChild>
                    <button
                      ref={(el) => (nodeRefs.current[idx] = el)}
                      onKeyDown={(e) => handleKeyDown(e, idx)}
                      tabIndex={0}
                      aria-expanded={activePopoverIndex === idx}
                      aria-haspopup="dialog"
                      aria-controls={`popover-mobile-${milestone.schedule_id}`}
                      className={`z-10 flex-shrink-0 relative flex items-center justify-center w-8 h-8 rounded-full border-2 ${style.border} ${style.bg} ${style.text} cursor-pointer hover:scale-105 active:scale-95 transition-all outline-none focus:ring-2 focus:ring-[#c9983a]`}
                    >
                      {getStatusIcon(state, "w-4.5 h-4.5")}
                    </button>
                  </PopoverTrigger>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between">
                      <span className={`text-[14px] font-bold block transition-colors ${
                        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                      }`}>
                        Milestone #{milestone.schedule_id}
                      </span>
                      <span className={`px-2 py-0.5 rounded-[4px] text-[9px] font-black uppercase tracking-wider border ${style.border} ${style.bg} ${style.text}`}>
                        {state}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 mt-1">
                      <span className="text-[13px] font-bold text-[#c9983a]">
                        {formatAmount(milestone.amount)} USDC
                      </span>
                      <span className={`text-[12px] transition-colors ${
                        theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                      }`}>
                        • {new Date(milestone.release_timestamp * 1000).toLocaleDateString(undefined, { 
                          month: 'short', 
                          day: 'numeric', 
                          year: 'numeric' 
                        })}
                      </span>
                    </div>
                  </div>

                  <PopoverContent
                    id={`popover-mobile-${milestone.schedule_id}`}
                    align="start"
                    side="bottom"
                    sideOffset={8}
                    className={`w-[290px] p-4 rounded-[16px] border backdrop-blur-[40px] z-[999] shadow-xl transition-all ${
                      theme === 'dark' 
                        ? 'bg-[#1e1914] border-white/15 text-[#f5f5f5]' 
                        : 'bg-[#fcfbf9] border-[#e2d6c1] text-[#2d2820]'
                    }`}
                  >
                    <div className="space-y-3">
                      <div className="flex items-center justify-between pb-1.5 border-b border-white/10">
                        <span className="text-[11px] font-bold tracking-wider uppercase opacity-70">
                          Details
                        </span>
                        <span className={`px-2 py-0.5 rounded-[4px] text-[10px] font-extrabold uppercase tracking-wide border ${style.border} ${style.bg} ${style.text}`}>
                          {state}
                        </span>
                      </div>

                      <div className="space-y-2">
                        <div className="flex items-center gap-2">
                          <Coins className={`w-4 h-4 ${style.text}`} />
                          <span className="text-[13px] font-bold">
                            {formatAmount(milestone.amount)} USDC
                          </span>
                        </div>
                        <div className="flex items-center gap-2">
                          <Calendar className={`w-4 h-4 ${style.text}`} />
                          <span className="text-[11px] font-medium leading-none">
                            {formatDate(milestone.release_timestamp)}
                          </span>
                        </div>
                      </div>

                      {milestone.tx_hash ? (
                        <a
                          href={`https://stellar.expert/explorer/testnet/tx/${milestone.tx_hash}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className={`flex items-center justify-center gap-1.5 w-full py-1.5 px-3 rounded-lg border text-[11px] font-bold text-center transition-all ${
                            theme === 'dark'
                              ? 'bg-white/[0.05] hover:bg-white/[0.1] border-white/15 text-[#f5f5c5]'
                              : 'bg-[#f5ebd6] hover:bg-[#ebdcb9] border-[#e2d6c1] text-[#8b6f3a]'
                          }`}
                        >
                          <ExternalLink className="w-3 h-3" />
                          View in Explorer
                        </a>
                      ) : (
                        <a
                          href={`https://stellar.expert/explorer/testnet/account/${milestone.recipient}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className={`flex items-center justify-center gap-1.5 w-full py-1.5 px-3 rounded-lg border text-[11px] font-bold text-center transition-all ${
                            theme === 'dark'
                              ? 'bg-white/[0.05] hover:bg-white/[0.1] border-white/15 text-[#f5c563]'
                              : 'bg-[#f5ebd6] hover:bg-[#ebdcb9] border-[#e2d6c1] text-[#b8872f]'
                          }`}
                        >
                          <ExternalLink className="w-3 h-3" />
                          View Recipient
                        </a>
                      )}
                    </div>
                  </PopoverContent>
                </Popover>
              </motion.div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
