import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { CommentThread } from '../CommentThread';
import { CommentData } from '../CommentCard';

vi.mock('../../contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark' }),
}));

vi.mock('../../../app/utils/renderMarkdown', () => ({
  default: ({ content }: { content: string }) => <div data-testid="markdown">{content}</div>,
}));

const topLevel: CommentData = {
  id: 1,
  body: 'First comment body',
  user: { login: 'alice' },
  created_at: '2026-07-26T18:00:00.000Z',
  updated_at: '2026-07-26T18:00:00.000Z',
  isAuthor: true,
  isMaintainer: false,
  reactions: [],
};

const reply1: CommentData = {
  id: 2,
  body: 'A reply',
  user: { login: 'bob' },
  created_at: '2026-07-26T19:00:00.000Z',
  updated_at: '2026-07-26T19:00:00.000Z',
  isAuthor: false,
  parentId: 1,
};

const reply2: CommentData = {
  id: 3,
  body: 'Another reply',
  user: { login: 'charlie' },
  created_at: '2026-07-26T19:30:00.000Z',
  updated_at: '2026-07-26T19:30:00.000Z',
  isAuthor: false,
  parentId: 1,
};

describe('CommentThread', () => {
  it('renders top-level comments', () => {
    render(
      <CommentThread
        comments={[topLevel]}
        onReply={vi.fn()}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByText('alice')).toBeInTheDocument();
    expect(screen.getByTestId('markdown')).toHaveTextContent('First comment body');
  });

  it('renders nested replies with indentation', () => {
    render(
      <CommentThread
        comments={[topLevel, reply1]}
        onReply={vi.fn()}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByText('bob')).toBeInTheDocument();
    expect(screen.getByText('A reply')).toBeInTheDocument();
  });

  it('shows empty state when no comments', () => {
    render(
      <CommentThread
        comments={[]}
        onReply={vi.fn()}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByText('No comments yet')).toBeInTheDocument();
  });

  it('shows comment count when totalCommentCount provided', () => {
    render(
      <CommentThread
        comments={[topLevel, reply1]}
        totalCommentCount={2}
        onReply={vi.fn()}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByText('2 comments')).toBeInTheDocument();
  });

  it('shows collapsed thread button when 5+ replies', () => {
    const manyReplies: CommentData[] = Array.from({ length: 6 }, (_, i) => ({
      id: 10 + i,
      body: `Reply ${i + 1}`,
      user: { login: `user${i}` },
      created_at: '2026-07-26T19:00:00.000Z',
      updated_at: '2026-07-26T19:00:00.000Z',
      parentId: 1,
    }));
    render(
      <CommentThread
        comments={[topLevel, ...manyReplies]}
        onReply={vi.fn()}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByText('View 4 more replies')).toBeInTheDocument();
  });

  it('shows aria-label on the section and list', () => {
    render(
      <CommentThread
        comments={[topLevel]}
        onReply={vi.fn()}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByLabelText('Comments')).toBeInTheDocument();
    expect(screen.getByLabelText('Comment thread')).toBeInTheDocument();
  });

  it('has data-testid attribute', () => {
    render(
      <CommentThread
        comments={[topLevel]}
        onReply={vi.fn()}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByTestId('comment-thread')).toBeInTheDocument();
  });

  describe('Accessibility - Keyboard Navigation', () => {
    it('supports Escape key to close edit mode', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          currentUserLogin="alice"
          onEdit={vi.fn()}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const editButton = screen.getByLabelText('Edit comment');
      fireEvent.click(editButton);
      
      const textarea = screen.getByLabelText(/Edit comment by/);
      expect(textarea).toBeInTheDocument();
      
      fireEvent.keyDown(textarea, { key: 'Escape' });
      expect(textarea).not.toBeInTheDocument();
    });

    it('supports Escape key to cancel reply composer', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const replyButton = screen.getByLabelText(/Reply to/);
      fireEvent.click(replyButton);
      
      const textarea = screen.getByLabelText(/Write a reply to/);
      expect(textarea).toBeInTheDocument();
      
      fireEvent.keyDown(textarea, { key: 'Escape' });
      expect(textarea).not.toBeInTheDocument();
    });

    it('supports Ctrl/Cmd+Enter to submit reply', () => {
      const mockReply = vi.fn().mockResolvedValue(undefined);
      render(
        <CommentThread
          comments={[topLevel]}
          onReply={mockReply}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const replyButton = screen.getByLabelText(/Reply to/);
      fireEvent.click(replyButton);
      
      const textarea = screen.getByLabelText(/Write a reply to/);
      fireEvent.change(textarea, { target: { value: 'Test reply' } });
      
      fireEvent.keyDown(textarea, { key: 'Enter', ctrlKey: true });
      expect(mockReply).toHaveBeenCalledWith(1, 'Test reply');
    });
  });

  describe('Accessibility - ARIA Attributes', () => {
    it('has aria-expanded on collapsed replies button', () => {
      const manyReplies: CommentData[] = Array.from({ length: 6 }, (_, i) => ({
        id: 10 + i,
        body: `Reply ${i + 1}`,
        user: { login: `user${i}` },
        created_at: '2026-07-26T19:00:00.000Z',
        updated_at: '2026-07-26T19:00:00.000Z',
        parentId: 1,
      }));
      
      render(
        <CommentThread
          comments={[topLevel, ...manyReplies]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const collapseButton = screen.getByLabelText(/View 4 more replies/);
      expect(collapseButton).toHaveAttribute('aria-expanded', 'false');
    });

    it('has aria-expanded on replies list', () => {
      const manyReplies: CommentData[] = Array.from({ length: 6 }, (_, i) => ({
        id: 10 + i,
        body: `Reply ${i + 1}`,
        user: { login: `user${i}` },
        created_at: '2026-07-26T19:00:00.000Z',
        updated_at: '2026-07-26T19:00:00.000Z',
        parentId: 1,
      }));
      
      render(
        <CommentThread
          comments={[topLevel, ...manyReplies]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const repliesList = screen.getByLabelText(/Replies to alice/);
      expect(repliesList).toHaveAttribute('aria-expanded', 'false');
    });

    it('has aria-label on all action buttons', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          currentUserLogin="alice"
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
          onEdit={vi.fn()}
          onDelete={vi.fn()}
        />
      );
      
      expect(screen.getByLabelText(/Reply to alice/)).toBeInTheDocument();
      expect(screen.getByLabelText('Edit comment')).toBeInTheDocument();
      expect(screen.getByLabelText('Delete comment')).toBeInTheDocument();
    });

    it('has aria-label on cancel buttons in edit mode', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          currentUserLogin="alice"
          onEdit={vi.fn()}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const editButton = screen.getByLabelText('Edit comment');
      fireEvent.click(editButton);
      
      expect(screen.getByLabelText('Cancel editing comment')).toBeInTheDocument();
      expect(screen.getByLabelText(/Save comment edit/)).toBeInTheDocument();
    });

    it('has aria-label on cancel buttons in delete confirmation', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          currentUserLogin="alice"
          onDelete={vi.fn()}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const deleteButton = screen.getByLabelText('Delete comment');
      fireEvent.click(deleteButton);
      
      expect(screen.getByLabelText('Confirm delete comment')).toBeInTheDocument();
      expect(screen.getByLabelText('Cancel delete comment')).toBeInTheDocument();
    });

    it('has aria-label on reply composer buttons', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const replyButton = screen.getByLabelText(/Reply to/);
      fireEvent.click(replyButton);
      
      expect(screen.getByLabelText('Cancel reply')).toBeInTheDocument();
      expect(screen.getByLabelText('Post reply')).toBeInTheDocument();
    });

    it('has aria-haspopup and aria-expanded on reaction picker button', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const addReactionButton = screen.getByLabelText('Add reaction');
      expect(addReactionButton).toHaveAttribute('aria-haspopup', 'listbox');
      expect(addReactionButton).toHaveAttribute('aria-expanded', 'false');
    });
  });

  describe('Accessibility - Screen Reader Behavior', () => {
    it('announces reply count in collapsed state', () => {
      const manyReplies: CommentData[] = Array.from({ length: 6 }, (_, i) => ({
        id: 10 + i,
        body: `Reply ${i + 1}`,
        user: { login: `user${i}` },
        created_at: '2026-07-26T19:00:00.000Z',
        updated_at: '2026-07-26T19:00:00.000Z',
        parentId: 1,
      }));
      
      render(
        <CommentThread
          comments={[topLevel, ...manyReplies]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const collapseButton = screen.getByLabelText(/View 4 more replies/);
      expect(collapseButton).toHaveAttribute('aria-label', 'View 4 more replies');
    });

    it('announces edit textarea with context', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          currentUserLogin="alice"
          onEdit={vi.fn()}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const editButton = screen.getByLabelText('Edit comment');
      fireEvent.click(editButton);
      
      const textarea = screen.getByLabelText('Edit comment by alice');
      expect(textarea).toBeInTheDocument();
    });

    it('announces reply composer with author context', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const replyButton = screen.getByLabelText(/Reply to/);
      fireEvent.click(replyButton);
      
      const textarea = screen.getByLabelText('Write a reply to alice');
      expect(textarea).toBeInTheDocument();
    });
  });

  describe('Accessibility - Focus Management', () => {
    it('auto-focuses reply composer textarea when opened', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const replyButton = screen.getByLabelText(/Reply to/);
      fireEvent.click(replyButton);
      
      const textarea = screen.getByLabelText(/Write a reply to/);
      expect(textarea).toHaveFocus();
    });

    it('all interactive elements have focus-visible styles', () => {
      render(
        <CommentThread
          comments={[topLevel]}
          onReply={vi.fn()}
          onReact={vi.fn()}
          onRemoveReaction={vi.fn()}
        />
      );
      
      const replyButton = screen.getByLabelText(/Reply to/);
      expect(replyButton).toHaveClass('focus-visible:outline-none');
      expect(replyButton).toHaveClass('focus-visible:ring-1');
    });
  });
});
