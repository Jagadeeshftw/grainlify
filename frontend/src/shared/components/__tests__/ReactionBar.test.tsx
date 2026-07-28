import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ReactionBar, CommentReaction } from '../ReactionBar';

vi.mock('../../contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark' }),
}));

const sampleReactions: CommentReaction[] = [
  { emoji: '+1', label: 'Thumbs up', count: 5, viewersReaction: false, reactors: ['alice', 'bob', 'charlie', 'dave', 'eve'] },
  { emoji: 'heart', label: 'Heart', count: 3, viewersReaction: true, reactors: ['currentUser', 'frank'] },
  { emoji: 'rocket', label: 'Rocket', count: 1, viewersReaction: false, reactors: ['grace'] },
];

describe('ReactionBar', () => {
  it('renders reaction buttons with emoji and count', () => {
    render(
      <ReactionBar
        reactions={sampleReactions}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByText('5')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('marks viewer-reacted button as pressed', () => {
    render(
      <ReactionBar
        reactions={sampleReactions}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    const buttons = screen.getAllByRole('button', { pressed: true });
    expect(buttons.length).toBe(1);
    expect(buttons[0]).toHaveAttribute('aria-pressed', 'true');
  });

  it('calls onReact when unreacted button is clicked', () => {
    const onReact = vi.fn();
    render(
      <ReactionBar
        reactions={sampleReactions}
        onReact={onReact}
        onRemoveReaction={vi.fn()}
      />
    );
    const unreactedButtons = screen.getAllByRole('button', { pressed: false });
    const thumbsUp = unreactedButtons.find((b) => b.textContent?.includes('5'));
    if (thumbsUp) fireEvent.click(thumbsUp);
    expect(onReact).toHaveBeenCalledWith('+1');
  });

  it('calls onRemoveReaction when viewer-reacted button is clicked', () => {
    const onRemoveReaction = vi.fn();
    render(
      <ReactionBar
        reactions={sampleReactions}
        onReact={vi.fn()}
        onRemoveReaction={onRemoveReaction}
      />
    );
    const reactedButtons = screen.getAllByRole('button', { pressed: true });
    fireEvent.click(reactedButtons[0]);
    expect(onRemoveReaction).toHaveBeenCalledWith('heart');
  });

  it('shows overflow button when more than 3 reaction types', () => {
    const manyReactions: CommentReaction[] = [
      { emoji: '+1', label: '+1', count: 1, viewersReaction: false, reactors: [] },
      { emoji: 'heart', label: 'Heart', count: 1, viewersReaction: false, reactors: [] },
      { emoji: 'rocket', label: 'Rocket', count: 1, viewersReaction: false, reactors: [] },
      { emoji: 'celebrate', label: 'Celebrate', count: 2, viewersReaction: false, reactors: ['test'] },
    ];
    render(
      <ReactionBar
        reactions={manyReactions}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByText('+1 more')).toBeInTheDocument();
  });

  it('includes aria-label on each reaction button', () => {
    render(
      <ReactionBar
        reactions={sampleReactions}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    const buttons = screen.getAllByRole('button', { pressed: false });
    expect(buttons.length).toBeGreaterThan(0);
    buttons.forEach((btn) => {
      expect(btn).toHaveAttribute('aria-label');
    });
  });

  it('has add reaction button', () => {
    render(
      <ReactionBar
        reactions={sampleReactions}
        onReact={vi.fn()}
        onRemoveReaction={vi.fn()}
      />
    );
    expect(screen.getByLabelText('Add reaction')).toBeInTheDocument();
  });
});
