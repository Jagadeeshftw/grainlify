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
});
