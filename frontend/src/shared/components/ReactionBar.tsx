import React, { useState } from 'react';
import { SmilePlus } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
  TooltipProvider,
} from '../../app/components/ui/tooltip';

export interface CommentReaction {
  emoji: string;
  label: string;
  count: number;
  viewersReaction: boolean;
  reactors: string[];
}

interface ReactionBarProps {
  reactions: CommentReaction[];
  onReact: (emoji: string) => void;
  onRemoveReaction: (emoji: string) => void;
  disabled?: boolean;
}

const COMMON_REACTIONS = [
  { emoji: '+1', label: 'Thumbs up', char: '👍' },
  { emoji: 'heart', label: 'Heart', char: '❤️' },
  { emoji: 'rocket', label: 'Rocket', char: '🚀' },
  { emoji: 'celebrate', label: 'Celebrate', char: '🎉' },
  { emoji: 'laugh', label: 'Laugh', char: '😄' },
  { emoji: 'eyes', label: 'Eyes', char: '👀' },
];

function getEmojiChar(emoji: string): string {
  const found = COMMON_REACTIONS.find((r) => r.emoji === emoji);
  return found ? found.char : emoji;
}

function getEmojiLabel(emoji: string): string {
  const found = COMMON_REACTIONS.find((r) => r.emoji === emoji);
  return found ? found.label : emoji;
}

function ReactionButton({ reaction, onToggle, isDark }: { reaction: CommentReaction; onToggle: () => void; isDark: boolean }) {
  const isPressed = reaction.viewersReaction;
  const reactorsText =
    reaction.reactors.length > 0
      ? `${reaction.reactors.slice(0, 5).join(', ')}${reaction.reactors.length > 5 ? ` +${reaction.reactors.length - 5} more` : ''}`
      : '';

  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            role="button"
            aria-pressed={isPressed}
            aria-label={`${getEmojiLabel(reaction.emoji)}. ${reaction.count} reaction${reaction.count !== 1 ? 's' : ''}. Click to toggle.`}
            onClick={onToggle}
            className={`inline-flex items-center gap-1 px-2.5 py-1 rounded-[8px] text-[12px] font-semibold border transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a] ${
              isPressed
                ? 'bg-[#c9983a]/20 border-[#c9983a]/40 text-[#c9983a]'
                : isDark
                  ? 'bg-white/[0.06] border-white/10 text-[#d4d4d4] hover:bg-white/[0.1]'
                  : 'bg-white/[0.3] border-black/10 text-[#7a6b5a] hover:bg-white/[0.5]'
            }`}
          >
            <span aria-hidden="true">{getEmojiChar(reaction.emoji)}</span>
            <span>{reaction.count}</span>
          </button>
        </TooltipTrigger>
        {reactorsText && (
          <TooltipContent
            side="top"
            align="center"
            className="bg-[#1f1b15]/95 border border-[#c9983a]/30 text-[#e8dfd0] backdrop-blur-md px-3 py-2 shadow-lg z-50 rounded-lg text-xs"
          >
            {reactorsText}
          </TooltipContent>
        )}
      </Tooltip>
    </TooltipProvider>
  );
}

function AddReactionButton({ onSelect, disabled, isDark }: { onSelect: (emoji: string) => void; disabled?: boolean; isDark: boolean }) {
  const [open, setOpen] = useState(false);

  return (
    <div className="relative">
      <button
        type="button"
        aria-label="Add reaction"
        disabled={disabled}
        onClick={() => setOpen((v) => !v)}
        onBlur={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget as Node)) {
            setOpen(false);
          }
        }}
        className={`inline-flex items-center gap-1 px-2 py-1 rounded-[8px] text-[12px] font-semibold border transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a] ${
          disabled
            ? 'opacity-50 cursor-not-allowed'
            : isDark
              ? 'bg-white/[0.06] border-white/10 text-[#d4d4d4] hover:bg-white/[0.1]'
              : 'bg-white/[0.3] border-black/10 text-[#7a6b5a] hover:bg-white/[0.5]'
        }`}
      >
        <SmilePlus className="w-3.5 h-3.5" />
      </button>
      {open && (
        <div
          role="listbox"
          aria-label="Choose a reaction"
          className={`absolute bottom-full left-0 mb-2 flex gap-1 p-2 rounded-[12px] shadow-lg z-50 ${
            isDark
              ? 'bg-[#2d2820] border border-white/15'
              : 'bg-white border border-black/15'
          }`}
        >
          {COMMON_REACTIONS.map((r) => (
            <button
              key={r.emoji}
              type="button"
              role="option"
              aria-label={r.label}
              onClick={() => {
                onSelect(r.emoji);
                setOpen(false);
              }}
              className="p-1.5 rounded-[6px] text-lg hover:bg-white/[0.1] transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
            >
              {r.char}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

const MAX_VISIBLE_REACTIONS = 3;

export function ReactionBar({ reactions, onReact, onRemoveReaction, disabled = false }: ReactionBarProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const visible = reactions.slice(0, MAX_VISIBLE_REACTIONS);
  const overflow = reactions.slice(MAX_VISIBLE_REACTIONS);
  const hasOverflow = overflow.length > 0;

  return (
    <div className="flex items-center gap-1.5 flex-wrap" role="group" aria-label="Reactions">
      {visible.map((reaction) => (
        <ReactionButton
          key={reaction.emoji}
          reaction={reaction}
          isDark={isDark}
          onToggle={() => {
            if (reaction.viewersReaction) {
              onRemoveReaction(reaction.emoji);
            } else {
              onReact(reaction.emoji);
            }
          }}
        />
      ))}
      {hasOverflow && (
        <TooltipProvider delayDuration={300}>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                aria-label={`${overflow.length} more reaction type${overflow.length !== 1 ? 's' : ''}`}
                className={`inline-flex items-center gap-1 px-2.5 py-1 rounded-[8px] text-[12px] font-semibold border transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a] ${
                  isDark
                    ? 'bg-white/[0.06] border-white/10 text-[#d4d4d4] hover:bg-white/[0.1]'
                    : 'bg-white/[0.3] border-black/10 text-[#7a6b5a] hover:bg-white/[0.5]'
                }`}
              >
                +{overflow.length} more
              </button>
            </TooltipTrigger>
            <TooltipContent
              side="top"
              align="center"
              className="bg-[#1f1b15]/95 border border-[#c9983a]/30 text-[#e8dfd0] backdrop-blur-md px-3 py-2 shadow-lg z-50 rounded-lg text-xs space-y-1"
            >
              {overflow.map((r) => (
                <div key={r.emoji} className="flex items-center gap-2">
                  <span>{getEmojiChar(r.emoji)}</span>
                  <span>{r.count}</span>
                  <span className="text-[#b8a898]">{r.reactors.slice(0, 3).join(', ')}{r.reactors.length > 3 ? '...' : ''}</span>
                </div>
              ))}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      )}
      <AddReactionButton
        disabled={disabled}
        isDark={isDark}
        onSelect={(emoji) => {
          const existing = reactions.find((r) => r.emoji === emoji);
          if (existing?.viewersReaction) {
            onRemoveReaction(emoji);
          } else {
            onReact(emoji);
          }
        }}
      />
    </div>
  );
}
