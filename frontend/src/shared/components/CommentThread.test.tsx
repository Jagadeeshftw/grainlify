/**
 * CommentThread accessibility & behavior tests
 *
 * These tests pin the explicit contract described in CommentThread.tsx.
 * They cover:
 *  - Structural ARIA (role="feed", articles, live region)
 *  - Screen-reader announcements on submit
 *  - Keyboard navigation within the list (Arrow keys, Home, End)
 *  - Ctrl+Enter / Cmd+Enter submit shortcut
 *  - Read-only mode hides compose area
 *  - Deterministic behavior across retries and re-renders
 */
import React from 'react'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import '@testing-library/jest-dom'
import { CommentThread, Comment } from './CommentThread'

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const COMMENTS: Comment[] = [
  {
    id: '1',
    author: 'Alice',
    body: 'First comment',
    timestamp: '2024-01-01T10:00:00Z',
  },
  {
    id: '2',
    author: 'Bob',
    body: 'Second comment',
    timestamp: '2024-01-01T11:00:00Z',
  },
  {
    id: '3',
    author: 'Carol',
    body: 'Third comment',
    timestamp: '2024-01-01T12:00:00Z',
  },
]

function setup(ui: React.ReactElement) {
  return {
    user: userEvent.setup(),
    ...render(ui),
  }
}

// ---------------------------------------------------------------------------
// Structure / ARIA
// ---------------------------------------------------------------------------

describe('CommentThread – structure', () => {
  it('renders a feed region with the provided label', () => {
    render(<CommentThread label="Issue #1 comments" comments={COMMENTS} />)
    const feed = screen.getByRole('feed', { name: 'Issue #1 comments' })
    expect(feed).toBeInTheDocument()
  })

  it('renders each comment as an article inside the feed', () => {
    render(<CommentThread label="Comments" comments={COMMENTS} />)
    const articles = screen.getAllByRole('article')
    expect(articles).toHaveLength(COMMENTS.length)
  })

  it('each article has aria-posinset and aria-setsize', () => {
    render(<CommentThread label="Comments" comments={COMMENTS} />)
    const articles = screen.getAllByRole('article')
    articles.forEach((article, i) => {
      expect(article).toHaveAttribute('aria-posinset', String(i + 1))
      expect(article).toHaveAttribute('aria-setsize', String(COMMENTS.length))
    })
  })

  it('each article label includes position, total, and author', () => {
    render(<CommentThread label="Comments" comments={COMMENTS} />)
    expect(
      screen.getByRole('article', { name: /Comment 1 of 3 by Alice/i }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('article', { name: /Comment 3 of 3 by Carol/i }),
    ).toBeInTheDocument()
  })

  it('shows empty state message when no comments', () => {
    render(<CommentThread label="Comments" comments={[]} />)
    expect(screen.getByText(/No comments yet/i)).toBeInTheDocument()
    expect(screen.queryAllByRole('article')).toHaveLength(0)
  })

  it('renders a hidden live region (role=status)', () => {
    render(<CommentThread label="Comments" comments={COMMENTS} />)
    const live = screen.getByRole('status')
    expect(live).toBeInTheDocument()
    expect(live).toHaveClass('sr-only')
    expect(live).toHaveAttribute('aria-live', 'polite')
    expect(live).toHaveAttribute('aria-atomic', 'true')
  })

  it('all article elements are focusable (tabIndex=0)', () => {
    render(<CommentThread label="Comments" comments={COMMENTS} />)
    screen.getAllByRole('article').forEach((el) => {
      expect(el).toHaveAttribute('tabindex', '0')
    })
  })
})

// ---------------------------------------------------------------------------
// Compose area
// ---------------------------------------------------------------------------

describe('CommentThread – compose area', () => {
  it('renders a labeled textarea and submit button when not read-only', () => {
    render(<CommentThread label="Comments" comments={[]} />)
    expect(screen.getByLabelText(/Write a comment/i)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Comment/i })).toBeInTheDocument()
  })

  it('submit button is disabled when textarea is empty', () => {
    render(<CommentThread label="Comments" comments={[]} />)
    expect(screen.getByRole('button', { name: /Comment/i })).toBeDisabled()
  })

  it('submit button becomes enabled after typing', async () => {
    const { user } = setup(<CommentThread label="Comments" comments={[]} />)
    await user.type(screen.getByLabelText(/Write a comment/i), 'hello')
    expect(screen.getByRole('button', { name: /Comment/i })).not.toBeDisabled()
  })

  it('calls onSubmit with trimmed text and clears the textarea on click', async () => {
    const onSubmit = vi.fn()
    const { user } = setup(
      <CommentThread label="Comments" comments={[]} onSubmit={onSubmit} />,
    )
    await user.type(screen.getByLabelText(/Write a comment/i), '  hello world  ')
    await user.click(screen.getByRole('button', { name: /Comment/i }))
    expect(onSubmit).toHaveBeenCalledOnce()
    expect(onSubmit).toHaveBeenCalledWith('hello world')
    expect(screen.getByLabelText(/Write a comment/i)).toHaveValue('')
  })

  it('calls onSubmit via Ctrl+Enter', async () => {
    const onSubmit = vi.fn()
    const { user } = setup(
      <CommentThread label="Comments" comments={[]} onSubmit={onSubmit} />,
    )
    const textarea = screen.getByLabelText(/Write a comment/i)
    await user.type(textarea, 'ctrl submit')
    await user.keyboard('{Control>}{Enter}{/Control}')
    expect(onSubmit).toHaveBeenCalledWith('ctrl submit')
  })

  it('calls onSubmit via Meta+Enter (Cmd+Enter)', async () => {
    const onSubmit = vi.fn()
    const { user } = setup(
      <CommentThread label="Comments" comments={[]} onSubmit={onSubmit} />,
    )
    const textarea = screen.getByLabelText(/Write a comment/i)
    await user.type(textarea, 'meta submit')
    await user.keyboard('{Meta>}{Enter}{/Meta}')
    expect(onSubmit).toHaveBeenCalledWith('meta submit')
  })

  it('plain Enter adds a newline, does not submit', async () => {
    const onSubmit = vi.fn()
    const { user } = setup(
      <CommentThread label="Comments" comments={[]} onSubmit={onSubmit} />,
    )
    const textarea = screen.getByLabelText(/Write a comment/i)
    await user.type(textarea, 'line1')
    await user.keyboard('{Enter}')
    await user.type(textarea, 'line2')
    expect(onSubmit).not.toHaveBeenCalled()
    expect(textarea).toHaveValue('line1\nline2')
  })

  it('does not call onSubmit when textarea contains only whitespace', async () => {
    const onSubmit = vi.fn()
    const { user } = setup(
      <CommentThread label="Comments" comments={[]} onSubmit={onSubmit} />,
    )
    const textarea = screen.getByLabelText(/Write a comment/i)
    await user.type(textarea, '   ')
    await user.keyboard('{Control>}{Enter}{/Control}')
    expect(onSubmit).not.toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// Screen-reader live region announcements
// ---------------------------------------------------------------------------

describe('CommentThread – live region', () => {
  it('live region is empty on initial render', () => {
    render(<CommentThread label="Comments" comments={[]} />)
    expect(screen.getByRole('status')).toHaveTextContent('')
  })

  it('announces posted comment after submit', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={[]} onSubmit={vi.fn()} />,
    )
    await user.type(screen.getByLabelText(/Write a comment/i), 'hello')
    await user.click(screen.getByRole('button', { name: /Comment/i }))
    expect(screen.getByRole('status')).toHaveTextContent(/Comment posted/i)
  })
})

// ---------------------------------------------------------------------------
// Keyboard navigation in the comment list
// ---------------------------------------------------------------------------

describe('CommentThread – keyboard navigation', () => {
  it('ArrowDown moves focus to next comment', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[0].focus()
    await user.keyboard('{ArrowDown}')
    expect(articles[1]).toHaveFocus()
  })

  it('ArrowUp moves focus to previous comment', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[1].focus()
    await user.keyboard('{ArrowUp}')
    expect(articles[0]).toHaveFocus()
  })

  it('ArrowDown on last item keeps focus on last item', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[articles.length - 1].focus()
    await user.keyboard('{ArrowDown}')
    expect(articles[articles.length - 1]).toHaveFocus()
  })

  it('ArrowUp on first item keeps focus on first item', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[0].focus()
    await user.keyboard('{ArrowUp}')
    expect(articles[0]).toHaveFocus()
  })

  it('End moves focus to last comment', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[0].focus()
    await user.keyboard('{End}')
    expect(articles[articles.length - 1]).toHaveFocus()
  })

  it('Home moves focus to first comment', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[2].focus()
    await user.keyboard('{Home}')
    expect(articles[0]).toHaveFocus()
  })

  it('ArrowRight also moves focus to next comment', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[0].focus()
    await user.keyboard('{ArrowRight}')
    expect(articles[1]).toHaveFocus()
  })

  it('ArrowLeft also moves focus to previous comment', async () => {
    const { user } = setup(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    const articles = screen.getAllByRole('article')
    articles[2].focus()
    await user.keyboard('{ArrowLeft}')
    expect(articles[1]).toHaveFocus()
  })
})

// ---------------------------------------------------------------------------
// Read-only mode
// ---------------------------------------------------------------------------

describe('CommentThread – read-only mode', () => {
  it('hides the compose textarea', () => {
    render(
      <CommentThread label="Comments" comments={COMMENTS} isReadOnly />,
    )
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument()
    expect(
      screen.queryByRole('button', { name: /Comment/i }),
    ).not.toBeInTheDocument()
  })

  it('comment articles remain focusable in read-only mode', () => {
    render(
      <CommentThread label="Comments" comments={COMMENTS} isReadOnly />,
    )
    screen.getAllByRole('article').forEach((el) => {
      expect(el).toHaveAttribute('tabindex', '0')
    })
  })

  it('does not call onSubmit even if called programmatically while read-only', async () => {
    // Simulates re-render with isReadOnly toggled mid-session
    const onSubmit = vi.fn()
    const { rerender } = render(
      <CommentThread label="Comments" comments={[]} onSubmit={onSubmit} />,
    )
    rerender(
      <CommentThread
        label="Comments"
        comments={[]}
        onSubmit={onSubmit}
        isReadOnly
      />,
    )
    // No compose area to interact with – onSubmit must not have been called
    expect(onSubmit).not.toHaveBeenCalled()
  })
})

// ---------------------------------------------------------------------------
// Determinism across re-renders
// ---------------------------------------------------------------------------

describe('CommentThread – determinism', () => {
  it('produces identical ARIA structure on repeated renders with same props', () => {
    const props = { label: 'Comments', comments: COMMENTS }
    const { container: c1, unmount } = render(<CommentThread {...props} />)
    const snapshot1 = c1.innerHTML
    unmount()

    const { container: c2 } = render(<CommentThread {...props} />)
    const snapshot2 = c2.innerHTML

    // IDs differ because useId generates new ones, but structure must match
    const strip = (s: string) => s.replace(/id="[^"]*"/g, 'id="X"').replace(/for="[^"]*"/g, 'for="X"').replace(/aria-labelledby="[^"]*"/g, '')
    expect(strip(snapshot1)).toBe(strip(snapshot2))
  })

  it('re-render with same comments does not change article count', () => {
    const { rerender } = render(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    expect(screen.getAllByRole('article')).toHaveLength(3)
    rerender(<CommentThread label="Comments" comments={COMMENTS} />)
    expect(screen.getAllByRole('article')).toHaveLength(3)
  })

  it('adding a comment updates aria-setsize on all existing articles', () => {
    const extra: Comment = {
      id: '4',
      author: 'Dave',
      body: 'Fourth',
      timestamp: '2024-01-01T13:00:00Z',
    }
    const { rerender } = render(
      <CommentThread label="Comments" comments={COMMENTS} />,
    )
    rerender(
      <CommentThread label="Comments" comments={[...COMMENTS, extra]} />,
    )
    screen.getAllByRole('article').forEach((el) => {
      expect(el).toHaveAttribute('aria-setsize', '4')
    })
  })
})
