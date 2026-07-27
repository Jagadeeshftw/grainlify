import React, {
  useRef,
  useCallback,
  useId,
  KeyboardEvent,
} from 'react'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Comment {
  id: string
  author: string
  avatarUrl?: string
  body: string
  /** ISO-8601 timestamp */
  timestamp: string
}

export interface CommentThreadProps {
  /** Thread-level label announced to screen readers, e.g. "Issue #42 comments" */
  label: string
  comments: Comment[]
  /** Called when the user submits a new comment. Return value is ignored. */
  onSubmit?: (body: string) => void
  /** Disables the reply textarea + submit button */
  isReadOnly?: boolean
  /** Custom class applied to the outermost element */
  className?: string
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Returns the array of focusable comment items and the compose textarea
 * inside the given container, in DOM order.
 */
function getFocusableItems(container: HTMLElement): HTMLElement[] {
  return Array.from(
    container.querySelectorAll<HTMLElement>(
      '[data-comment-item], [data-compose-textarea], [data-compose-submit]',
    ),
  )
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface CommentItemProps {
  comment: Comment
  index: number
  total: number
  listId: string
}

const CommentItem = React.memo(function CommentItem({
  comment,
  index,
  total,
  listId,
}: CommentItemProps) {
  const itemId = `${listId}-comment-${comment.id}`

  return (
    <article
      id={itemId}
      data-comment-item
      tabIndex={0}
      aria-label={`Comment ${index + 1} of ${total} by ${comment.author}`}
      aria-posinset={index + 1}
      aria-setsize={total}
      className="flex gap-3 p-3 rounded-lg focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
    >
      {/* Avatar */}
      {comment.avatarUrl ? (
        <img
          src={comment.avatarUrl}
          alt={`${comment.author}'s avatar`}
          aria-hidden="false"
          className="w-8 h-8 rounded-full flex-shrink-0 object-cover"
        />
      ) : (
        <span
          aria-hidden="true"
          className="w-8 h-8 rounded-full flex-shrink-0 bg-gray-300 flex items-center justify-center text-sm font-semibold select-none"
        >
          {comment.author.charAt(0).toUpperCase()}
        </span>
      )}

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-baseline gap-2 flex-wrap">
          <span className="font-semibold text-sm">{comment.author}</span>
          <time
            dateTime={comment.timestamp}
            className="text-xs text-gray-500"
          >
            {new Date(comment.timestamp).toLocaleString()}
          </time>
        </div>
        <p className="mt-1 text-sm whitespace-pre-wrap break-words">
          {comment.body}
        </p>
      </div>
    </article>
  )
})

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

/**
 * CommentThread
 *
 * Accessibility contract (pinned for regression safety):
 *
 * Structure
 * - The comment list is a `<section>` with `role="feed"` so screen readers
 *   announce position-in-set / set-size for each article.
 * - Each comment is an `<article>` with tabIndex=0, aria-posinset, and
 *   aria-setsize.
 * - New comments are announced via an aria-live="polite" region.
 * - The compose area is labelled with a visible `<label>` and connected via
 *   htmlFor/id.
 *
 * Keyboard navigation (roving-tabindex pattern on the list)
 * - Arrow Down / Arrow Right  → next comment
 * - Arrow Up   / Arrow Left   → previous comment
 * - Home                      → first comment
 * - End                       → last comment
 * - Tab                       → leaves the list and reaches the compose area
 *
 * The compose textarea supports standard text-editing keys. Enter alone adds
 * a newline; Ctrl+Enter / Cmd+Enter submits (matches GitHub / Linear UX).
 *
 * Read-only mode
 * - `isReadOnly` hides and disables the compose area entirely.
 * - Comment items remain keyboard-focusable so readers can still navigate.
 */
export function CommentThread({
  label,
  comments,
  onSubmit,
  isReadOnly = false,
  className = '',
}: CommentThreadProps) {
  const baseId = useId()
  const listId = `${baseId}-list`
  const liveRegionId = `${baseId}-live`
  const textareaId = `${baseId}-textarea`

  const containerRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const [draft, setDraft] = React.useState('')
  const [liveMessage, setLiveMessage] = React.useState('')

  // ------------------------------------------------------------------
  // Keyboard navigation within the comment list (roving focus)
  // ------------------------------------------------------------------
  const handleListKeyDown = useCallback(
    (e: KeyboardEvent<HTMLElement>) => {
      const container = containerRef.current
      if (!container) return

      const items = getFocusableItems(container)
      const commentItems = items.filter((el) =>
        el.hasAttribute('data-comment-item'),
      )
      if (commentItems.length === 0) return

      const focused = document.activeElement as HTMLElement
      const currentIndex = commentItems.indexOf(focused)

      let nextIndex: number | null = null

      switch (e.key) {
        case 'ArrowDown':
        case 'ArrowRight':
          e.preventDefault()
          nextIndex =
            currentIndex < commentItems.length - 1 ? currentIndex + 1 : currentIndex
          break
        case 'ArrowUp':
        case 'ArrowLeft':
          e.preventDefault()
          nextIndex = currentIndex > 0 ? currentIndex - 1 : 0
          break
        case 'Home':
          e.preventDefault()
          nextIndex = 0
          break
        case 'End':
          e.preventDefault()
          nextIndex = commentItems.length - 1
          break
        default:
          return
      }

      if (nextIndex !== null) {
        ;(commentItems[nextIndex] as HTMLElement).focus()
      }
    },
    [],
  )

  // ------------------------------------------------------------------
  // Compose area – Ctrl/Cmd+Enter to submit
  // ------------------------------------------------------------------
  const handleTextareaKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        submitComment()
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [draft],
  )

  const submitComment = useCallback(() => {
    const trimmed = draft.trim()
    if (!trimmed || isReadOnly) return
    onSubmit?.(trimmed)
    setDraft('')
    setLiveMessage(`Comment posted by you.`)
    // Reset live region after announcement (prevents stale repeat)
    setTimeout(() => setLiveMessage(''), 1000)
    textareaRef.current?.focus()
  }, [draft, isReadOnly, onSubmit])

  const handleSubmitClick = useCallback(() => {
    submitComment()
  }, [submitComment])

  // ------------------------------------------------------------------
  // Render
  // ------------------------------------------------------------------
  return (
    <div
      ref={containerRef}
      className={`flex flex-col gap-4 ${className}`}
      data-testid="comment-thread"
    >
      {/* Visually hidden live region for screen-reader announcements */}
      <div
        id={liveRegionId}
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {liveMessage}
      </div>

      {/* Comment list */}
      <section
        aria-label={label}
        aria-describedby={comments.length === 0 ? undefined : undefined}
        // role="feed" is appropriate for a dynamic stream of articles
        role="feed"
        onKeyDown={handleListKeyDown}
      >
        {comments.length === 0 ? (
          <p className="text-sm text-gray-500 py-4 text-center" aria-live="polite">
            No comments yet.
          </p>
        ) : (
          comments.map((comment, index) => (
            <CommentItem
              key={comment.id}
              comment={comment}
              index={index}
              total={comments.length}
              listId={listId}
            />
          ))
        )}
      </section>

      {/* Compose area */}
      {!isReadOnly && (
        <div className="flex flex-col gap-2" role="form" aria-label="Add a comment">
          <label htmlFor={textareaId} className="text-sm font-medium">
            Write a comment
          </label>
          <textarea
            id={textareaId}
            ref={textareaRef}
            data-compose-textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={handleTextareaKeyDown}
            aria-label="Comment body"
            aria-multiline="true"
            placeholder="Add a comment… (Ctrl+Enter to submit)"
            rows={3}
            className="w-full rounded-md border border-gray-300 p-2 text-sm resize-y focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
          />
          <button
            data-compose-submit
            type="button"
            onClick={handleSubmitClick}
            disabled={draft.trim().length === 0}
            aria-disabled={draft.trim().length === 0}
            className="self-end px-4 py-1.5 rounded-md text-sm font-semibold disabled:opacity-50 disabled:cursor-not-allowed focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
          >
            Comment
          </button>
        </div>
      )}
    </div>
  )
}

export default CommentThread
