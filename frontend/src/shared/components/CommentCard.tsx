import React, { useState } from 'react';
import { Pencil, Trash2, MessageSquare } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';
import { TimestampDisplay } from './TimestampDisplay';
import { ReactionBar, CommentReaction } from './ReactionBar';
import { ReplyComposer } from './ReplyComposer';
import RenderMarkdownContent from '../../app/utils/renderMarkdown';

export interface CommentData {
  id: number;
  body: string;
  user: { login: string };
  created_at: string;
  updated_at: string;
  isAuthor?: boolean;
  isMaintainer?: boolean;
  reactions?: CommentReaction[];
  replyCount?: number;
  parentId?: number | null;
}

interface CommentCardProps {
  comment: CommentData;
  depth: 0 | 1;
  currentUserLogin?: string;
  failedAvatars: Set<string>;
  onAvatarError: (url: string) => void;
  onReact: (commentId: number, emoji: string) => void;
  onRemoveReaction: (commentId: number, emoji: string) => void;
  onReply: (parentId: number, body: string) => Promise<void>;
  onEdit?: (commentId: number, body: string) => Promise<void>;
  onDelete?: (commentId: number) => Promise<void>;
}

function getGitHubAvatar(login: string, size: number = 32): string {
  return `https://github.com/${login}.png?size=${size}`;
}

export function CommentCard({
  comment,
  depth,
  currentUserLogin,
  failedAvatars,
  onAvatarError,
  onReact,
  onRemoveReaction,
  onReply,
  onEdit,
  onDelete,
}: CommentCardProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const isOwn = currentUserLogin != null && comment.user.login.toLowerCase() === currentUserLogin.toLowerCase();
  const [isEditing, setIsEditing] = useState(false);
  const [editBody, setEditBody] = useState(comment.body);
  const [isSubmittingEdit, setIsSubmittingEdit] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [showReply, setShowReply] = useState(false);

  const avatarUrl = getGitHubAvatar(comment.user.login, depth === 0 ? 32 : 24);
  const avatarSize = depth === 0 ? 'w-8 h-8' : 'w-6 h-6';
  const avatarBorder = depth === 0 ? 'border-2' : 'border';

  const handleEdit = async () => {
    if (!onEdit || !editBody.trim() || isSubmittingEdit) return;
    setIsSubmittingEdit(true);
    try {
      await onEdit(comment.id, editBody.trim());
      setIsEditing(false);
    } finally {
      setIsSubmittingEdit(false);
    }
  };

  const handleDelete = async () => {
    if (!onDelete || isDeleting) return;
    setIsDeleting(true);
    try {
      await onDelete(comment.id);
    } finally {
      setIsDeleting(false);
      setShowDeleteConfirm(false);
    }
  };

  return (
    <div
      className={`backdrop-blur-[25px] rounded-[16px] border p-5 transition-colors ${
        isDark ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
      }`}
    >
      <div className="flex items-start gap-3 mb-3">
        <button
          type="button"
          className="flex-shrink-0 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a] rounded-full"
          aria-label={`${comment.user.login}'s avatar`}
        >
          {failedAvatars.has(avatarUrl) ? (
            <div
              className={`${avatarSize} rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border border-[#c9983a]/40 flex items-center justify-center`}
            >
              <span className="text-[11px] font-bold text-[#c9983a]">
                {comment.user.login.substring(0, 2).toUpperCase()}
              </span>
            </div>
          ) : (
            <img
              src={avatarUrl}
              alt={comment.user.login}
              className={`${avatarSize} rounded-full border border-[#c9983a]/40`}
              onError={() => onAvatarError(avatarUrl)}
            />
          )}
        </button>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span
              className={`text-[14px] font-bold transition-colors ${
                isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              {comment.user.login}
            </span>
            {comment.isMaintainer && (
              <span className="px-2 py-0.5 rounded-[4px] bg-[#c9983a]/20 border border-[#c9983a]/30 text-[10px] font-bold text-[#c9983a]">
                MAINTAINER
              </span>
            )}
            {comment.isAuthor && (
              <span className="px-2 py-0.5 rounded-[4px] bg-[#c9983a]/20 border border-[#c9983a]/30 text-[10px] font-bold text-[#c9983a]">
                AUTHOR
              </span>
            )}
            <span className="text-[12px] text-[#b8a898]" aria-hidden="true">·</span>
            <TimestampDisplay
              timestamp={comment.created_at}
              className={`text-[12px] transition-colors ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}
            />
          </div>
        </div>
      </div>

      {isEditing ? (
        <div className="space-y-2">
          <textarea
            value={editBody}
            onChange={(e) => setEditBody(e.target.value)}
            className="w-full min-h-[100px] rounded-[12px] border px-4 py-3 text-[14px] outline-none transition-colors resize-y bg-white/[0.06] border-white/15 text-[#e8dfd0] placeholder:text-[#b8a898]/60 focus-visible:ring-1 focus-visible:ring-[#c9983a]"
            aria-label="Edit comment"
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                setIsEditing(false);
                setEditBody(comment.body);
              }
            }}
          />
          <div className="flex items-center gap-2 justify-end">
            <button
              type="button"
              onClick={() => {
                setIsEditing(false);
                setEditBody(comment.body);
              }}
              className="px-3 py-1.5 rounded-[8px] text-[12px] font-semibold bg-white/[0.06] border border-white/10 text-[#d4d4d4] hover:bg-white/[0.1] transition-all"
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={!editBody.trim() || isSubmittingEdit}
              onClick={handleEdit}
              className="px-3 py-1.5 rounded-[8px] text-[12px] font-semibold bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white border border-white/10 hover:opacity-90 transition-all disabled:opacity-50"
            >
              {isSubmittingEdit ? 'Saving...' : 'Save'}
            </button>
          </div>
        </div>
      ) : (
        <div
          className={`text-[14px] leading-relaxed whitespace-pre-wrap break-words transition-colors ${
            isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
          }`}
        >
          <RenderMarkdownContent content={comment.body} />
        </div>
      )}

      <div className="mt-3 flex items-center justify-between gap-2 flex-wrap">
        <ReactionBar
          reactions={comment.reactions || []}
          onReact={(emoji) => onReact(comment.id, emoji)}
          onRemoveReaction={(emoji) => onRemoveReaction(comment.id, emoji)}
          disabled={isEditing}
        />

        <div className="flex items-center gap-1">
          <button
            type="button"
            aria-label={`Reply to ${comment.user.login}`}
            onClick={() => setShowReply((v) => !v)}
            className="inline-flex items-center gap-1 px-2.5 py-1 rounded-[8px] text-[12px] font-semibold border bg-white/[0.06] border-white/10 text-[#d4d4d4] hover:bg-white/[0.1] transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
          >
            <MessageSquare className="w-3.5 h-3.5" />
            <span>Reply</span>
          </button>

          {isOwn && (
            <>
              <button
                type="button"
                aria-label="Edit comment"
                onClick={() => {
                  setIsEditing(true);
                  setEditBody(comment.body);
                }}
                className="inline-flex items-center gap-1 px-2.5 py-1 rounded-[8px] text-[12px] font-semibold border bg-white/[0.06] border-white/10 text-[#d4d4d4] hover:bg-white/[0.1] transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
              >
                <Pencil className="w-3.5 h-3.5" />
              </button>
              {showDeleteConfirm ? (
                <div className="flex items-center gap-1">
                  <span className="text-[11px] text-[#ef4444]">Confirm?</span>
                  <button
                    type="button"
                    disabled={isDeleting}
                    onClick={handleDelete}
                    className="px-2 py-1 rounded-[6px] text-[11px] font-semibold bg-red-500/20 border border-red-500/30 text-red-400 hover:bg-red-500/30 transition-all"
                  >
                    {isDeleting ? '...' : 'Delete'}
                  </button>
                  <button
                    type="button"
                    onClick={() => setShowDeleteConfirm(false)}
                    className="px-2 py-1 rounded-[6px] text-[11px] font-semibold bg-white/[0.06] border border-white/10 text-[#d4d4d4] hover:bg-white/[0.1] transition-all"
                  >
                    Cancel
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  aria-label="Delete comment"
                  onClick={() => setShowDeleteConfirm(true)}
                  className="inline-flex items-center gap-1 px-2.5 py-1 rounded-[8px] text-[12px] font-semibold border bg-white/[0.06] border-white/10 text-[#d4d4d4] hover:bg-red-500/20 hover:border-red-500/30 hover:text-red-400 transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </button>
              )}
            </>
          )}
        </div>
      </div>

      {showReply && (
        <ReplyComposer
          authorName={comment.user.login}
          onCancel={() => setShowReply(false)}
          onSubmit={async (body) => {
            await onReply(comment.id, body);
            setShowReply(false);
          }}
        />
      )}
    </div>
  );
}
