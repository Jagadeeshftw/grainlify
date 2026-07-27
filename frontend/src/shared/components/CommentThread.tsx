import React, { useState } from 'react';
import { MessageSquare, Loader2 } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';
import { CommentCard, CommentData } from './CommentCard';
import { SkeletonLoader } from './SkeletonLoader';

interface CommentThreadProps {
  comments: CommentData[];
  currentUserLogin?: string;
  onReply: (parentId: number, body: string) => Promise<void>;
  onReact: (commentId: number, emoji: string) => void;
  onRemoveReaction: (commentId: number, emoji: string) => void;
  onEdit?: (commentId: number, body: string) => Promise<void>;
  onDelete?: (commentId: number) => Promise<void>;
  onLoadMore?: () => Promise<void>;
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCommentCount?: number;
  ['data-testid']?: string;
}

const REPLIES_COLLAPSE_THRESHOLD = 5;

function CollapsedRepliesButton({
  count,
  onExpand,
}: {
  count: number;
  onExpand: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onExpand}
      className="w-full text-left px-2 py-2 rounded-[8px] text-[12px] font-semibold text-[#c9983a] hover:bg-white/[0.05] transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
    >
      View {count} more repl{count !== 1 ? 'ies' : 'y'}
    </button>
  );
}

function TopLevelComment({
  comment,
  allReplies,
  currentUserLogin,
  failedAvatars,
  onAvatarError,
  onReact,
  onRemoveReaction,
  onReply,
  onEdit,
  onDelete,
}: {
  comment: CommentData;
  allReplies: CommentData[];
  currentUserLogin?: string;
  failedAvatars: Set<string>;
  onAvatarError: (url: string) => void;
  onReact: (commentId: number, emoji: string) => void;
  onRemoveReaction: (commentId: number, emoji: string) => void;
  onReply: (parentId: number, body: string) => Promise<void>;
  onEdit?: (commentId: number, body: string) => Promise<void>;
  onDelete?: (commentId: number) => Promise<void>;
}) {
  const [expanded, setExpanded] = useState(false);
  const replies = allReplies;
  const hasManyReplies = replies.length >= REPLIES_COLLAPSE_THRESHOLD;
  const visibleReplies = hasManyReplies && !expanded ? replies.slice(0, 2) : replies;
  const hiddenCount = replies.length - visibleReplies.length;

  return (
    <li className="space-y-3">
      <CommentCard
        comment={comment}
        depth={0}
        currentUserLogin={currentUserLogin}
        failedAvatars={failedAvatars}
        onAvatarError={onAvatarError}
        onReact={onReact}
        onRemoveReaction={onRemoveReaction}
        onReply={onReply}
        onEdit={onEdit}
        onDelete={onDelete}
      />

      {replies.length > 0 && (
        <ol
          className="ml-8 space-y-3 border-l-2 border-white/10 pl-4"
          aria-label={`Replies to ${comment.user.login}`}
        >
          {visibleReplies.map((reply) => (
            <li key={reply.id}>
              <CommentCard
                comment={reply}
                depth={1}
                currentUserLogin={currentUserLogin}
                failedAvatars={failedAvatars}
                onAvatarError={onAvatarError}
                onReact={onReact}
                onRemoveReaction={onRemoveReaction}
                onReply={onReply}
                onEdit={onEdit}
                onDelete={onDelete}
              />
            </li>
          ))}
          {hasManyReplies && !expanded && (
            <li>
              <CollapsedRepliesButton
                count={hiddenCount}
                onExpand={() => setExpanded(true)}
              />
            </li>
          )}
        </ol>
      )}
    </li>
  );
}

export function CommentThread({
  comments,
  currentUserLogin,
  onReply,
  onReact,
  onRemoveReaction,
  onEdit,
  onDelete,
  onLoadMore,
  hasMore = false,
  isLoadingMore = false,
  totalCommentCount,
  'data-testid': testId = 'comment-thread',
}: CommentThreadProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const [failedAvatars, setFailedAvatars] = useState<Set<string>>(new Set());

  const handleAvatarError = (url: string) => {
    setFailedAvatars((prev) => new Set(prev).add(url));
  };

  const topLevel = comments.filter((c) => !c.parentId);
  const repliesByParent = comments.reduce<Record<number, CommentData[]>>((acc, c) => {
    if (c.parentId) {
      if (!acc[c.parentId]) acc[c.parentId] = [];
      acc[c.parentId].push(c);
    }
    return acc;
  }, {});

  if (comments.length === 0) {
    return (
      <div
        data-testid={testId}
        className={`p-8 rounded-[16px] backdrop-blur-[25px] border text-center min-h-[300px] flex flex-col items-center justify-center ${
          isDark ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
        }`}
      >
        <MessageSquare
          className={`w-12 h-12 mx-auto mb-4 transition-colors ${
            isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}
        />
        <p
          className={`text-[14px] transition-colors ${
            isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}
        >
          No comments yet
        </p>
        <p
          className={`text-[12px] mt-1 transition-colors ${
            isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}
        >
          Be the first to comment on this issue.
        </p>
      </div>
    );
  }

  return (
    <section data-testid={testId} aria-label="Comments" className="space-y-4">
      {totalCommentCount != null && (
        <div
          className={`text-[13px] font-semibold transition-colors ${
            isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
          }`}
        >
          {totalCommentCount} comment{totalCommentCount !== 1 ? 's' : ''}
        </div>
      )}

      <ol role="list" aria-label="Comment thread" className="space-y-4">
        {topLevel.map((comment) => (
          <TopLevelComment
            key={comment.id}
            comment={comment}
            allReplies={repliesByParent[comment.id] || []}
            currentUserLogin={currentUserLogin}
            failedAvatars={failedAvatars}
            onAvatarError={handleAvatarError}
            onReact={onReact}
            onRemoveReaction={onRemoveReaction}
            onReply={onReply}
            onEdit={onEdit}
            onDelete={onDelete}
          />
        ))}
      </ol>

      {hasMore && (
        <div className="flex justify-center pt-2">
          {isLoadingMore ? (
            <div className="space-y-3 w-full">
              <div
                className={`backdrop-blur-[25px] rounded-[16px] border p-5 transition-colors ${
                  isDark ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
                }`}
              >
                <SkeletonLoader className="h-4 w-3/4 rounded-[8px]" />
                <SkeletonLoader className="h-3 w-full mt-3 rounded-[8px]" />
                <SkeletonLoader className="h-3 w-1/2 mt-2 rounded-[8px]" />
              </div>
              <div
                className={`backdrop-blur-[25px] rounded-[16px] border p-5 transition-colors ${
                  isDark ? 'bg-white/[0.08] border-white/10' : 'bg-white/[0.15] border-white/25'
                }`}
              >
                <SkeletonLoader className="h-4 w-2/3 rounded-[8px]" />
                <SkeletonLoader className="h-3 w-full mt-3 rounded-[8px]" />
              </div>
            </div>
          ) : (
            <button
              type="button"
              onClick={onLoadMore}
              className="px-4 py-2 rounded-[10px] text-[13px] font-semibold bg-white/[0.06] border border-white/10 text-[#d4d4d4] hover:bg-white/[0.1] transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
            >
              Load more comments
            </button>
          )}
        </div>
      )}
    </section>
  );
}
