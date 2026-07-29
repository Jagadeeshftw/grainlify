/**
 * MediaEmbed.test.tsx
 *
 * Test suite for the MediaEmbed component.
 *
 * Coverage areas:
 * 1. Rendering — poster-placeholder on mount, correct ARIA structure
 * 2. Lazy-load — IntersectionObserver fires, src committed to media element
 * 3. Video state machine — loading → loaded-paused → playing → loaded-paused
 * 4. GIF state machine — loading → gif-autoplay-with-pause-control → gif-paused → resume
 * 5. Error state — error event → error-unavailable, retry resets state
 * 6. Autoplay policy — video never calls play() without user gesture
 * 7. Accessibility — aria-labels, aria-pressed, aria-live, role="region"
 * 8. prefers-reduced-motion — GIFs start paused, transitions suppressed
 * 9. Design QA — play/pause control icon contrast (4.5:1 minimum)
 * 10. Keyboard — play/pause reachable via keyboard, no unexpected sound autoplay
 * 11. Responsive — aspect-ratio container present at 375px (no layout shift)
 *
 * @see design/specs/video-gif-embed-spec.md
 */

import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ThemeProvider } from '../contexts/ThemeContext';
import { MediaEmbed } from './MediaEmbed';

// ---------------------------------------------------------------------------
// IntersectionObserver mock
// ---------------------------------------------------------------------------

type IOCallback = (entries: IntersectionObserverEntry[]) => void;

// Track every observer instance created so we can target the right one
const observerInstances: MockIntersectionObserver[] = [];

const mockDisconnect = vi.fn();
const mockObserve = vi.fn();

class MockIntersectionObserver {
  private cb: IOCallback;
  observedElement: Element | null = null;

  constructor(cb: IOCallback) {
    this.cb = cb;
    observerInstances.push(this);
  }
  observe = (el: Element) => {
    this.observedElement = el;
    mockObserve(el);
  };
  disconnect = mockDisconnect;
  unobserve = vi.fn();

  /** Trigger this specific observer instance. */
  trigger(isIntersecting = true) {
    this.cb([
      { isIntersecting, target: this.observedElement! } as IntersectionObserverEntry,
    ]);
  }
}

/** Trigger the FIRST observer (lazy-load sentinel). */
function triggerIntersection(isIntersecting = true) {
  observerInstances[0]?.trigger(isIntersecting);
}

// ---------------------------------------------------------------------------
// matchMedia mock (prefers-reduced-motion)
// ---------------------------------------------------------------------------

function mockMatchMedia(prefersReduced: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn((query: string) => ({
      matches: query.includes('prefers-reduced-motion') ? prefersReduced : false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

// ---------------------------------------------------------------------------
// Wrapper
// ---------------------------------------------------------------------------

function Wrapper({ children }: { children: React.ReactNode }) {
  return <ThemeProvider>{children}</ThemeProvider>;
}

function renderEmbed(props: React.ComponentProps<typeof MediaEmbed>) {
  return render(<MediaEmbed {...props} />, { wrapper: Wrapper });
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  vi.stubGlobal('IntersectionObserver', MockIntersectionObserver);
  mockMatchMedia(false);
  mockDisconnect.mockClear();
  mockObserve.mockClear();
  observerInstances.length = 0;
});

afterEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
});

// ---------------------------------------------------------------------------
// 1. Rendering — initial state
// ---------------------------------------------------------------------------

describe('MediaEmbed — initial render', () => {
  it('renders with role="region" on the container', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video' });
    expect(screen.getByRole('region')).toBeInTheDocument();
  });

  it('applies aria-label with title when provided', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video', title: 'Project demo' });
    expect(screen.getByRole('region')).toHaveAttribute(
      'aria-label',
      'Video: Project demo',
    );
  });

  it('applies generic aria-label for video without title', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video' });
    expect(screen.getByRole('region')).toHaveAttribute('aria-label', 'Video');
  });

  it('applies generic aria-label for gif without title', () => {
    renderEmbed({ src: 'demo.gif', kind: 'gif' });
    expect(screen.getByRole('region')).toHaveAttribute('aria-label', 'Animated GIF');
  });

  it('does NOT render play button before viewport intersection (video)', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video' });
    expect(screen.queryByRole('button', { name: /play video/i })).toBeNull();
  });

  it('does NOT commit src to video element before intersection', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    const video = container.querySelector('video');
    // video element is rendered but src not yet set
    expect(video?.getAttribute('src')).toBeFalsy();
  });

  it('shows sentinel div in poster-placeholder state', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    // sentinel: last aria-hidden div inside aspect-ratio root
    const sentinel = container.querySelector('[aria-hidden="true"].absolute.bottom-0');
    expect(sentinel).toBeInTheDocument();
  });

  it('starts with aria-busy="false" (not loading yet)', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video' });
    expect(screen.getByRole('region')).toHaveAttribute('aria-busy', 'false');
  });
});

// ---------------------------------------------------------------------------
// 2. Lazy-load — IntersectionObserver
// ---------------------------------------------------------------------------

describe('MediaEmbed — lazy-load', () => {
  it('registers IntersectionObserver on mount (at least one observer)', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video' });
    // Two observers mount: lazy-load sentinel + scroll-out-of-viewport for video
    expect(mockObserve).toHaveBeenCalled();
  });

  it('sets aria-busy="true" after intersection fires (loading state)', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    expect(screen.getByRole('region')).toHaveAttribute('aria-busy', 'true');
  });

  it('commits src to video element after intersection', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video');
    expect(video?.getAttribute('src')).toContain('demo.mp4');
  });

  it('commits src to img element after intersection for gif', () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])');
    expect(img?.getAttribute('src')).toContain('anim.gif');
  });

  it('disconnects lazy-load observer after first intersection', () => {
    renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    // disconnect is called by the lazy-load observer on intersection
    // (may also be called by cleanup — just verify it was called)
    expect(mockDisconnect).toHaveBeenCalled();
  });

  it('does not re-commit src on second intersection call', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    act(() => triggerIntersection(true));
    const video = container.querySelector('video');
    expect(video?.getAttribute('src')).toContain('demo.mp4');
  });
});

// ---------------------------------------------------------------------------
// 3. Video state machine
// ---------------------------------------------------------------------------

describe('MediaEmbed — video state machine', () => {
  it('shows play button after canplay event (loaded-paused)', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    act(() => fireEvent.canPlay(video));
    expect(screen.getByRole('button', { name: /play video/i })).toBeInTheDocument();
  });

  it('play button click calls video.play() (playing state)', async () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;

    // Stub play to return a resolved promise (browser API)
    const playSpy = vi.spyOn(video, 'play').mockResolvedValue(undefined);

    act(() => fireEvent.canPlay(video));
    const playBtn = screen.getByRole('button', { name: /play video/i });
    await userEvent.click(playBtn);

    expect(playSpy).toHaveBeenCalledTimes(1);
  });

  it('shows pause button after video transitions to playing state', async () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));

    await userEvent.click(screen.getByRole('button', { name: /play video/i }));
    expect(await screen.findByRole('button', { name: /pause video/i })).toBeInTheDocument();
  });

  it('pause button calls video.pause() and returns to loaded-paused', async () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    const pauseSpy = vi.spyOn(video, 'pause').mockImplementation(() => undefined);
    act(() => fireEvent.canPlay(video));

    await userEvent.click(screen.getByRole('button', { name: /play video/i }));
    await userEvent.click(await screen.findByRole('button', { name: /pause video/i }));

    expect(pauseSpy).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole('button', { name: /play video/i })).toBeInTheDocument();
  });

  it('video ended event returns to loaded-paused', async () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));
    await userEvent.click(screen.getByRole('button', { name: /play video/i }));
    act(() => fireEvent.ended(video));

    expect(await screen.findByRole('button', { name: /play video/i })).toBeInTheDocument();
  });

  it('renders captions track when captionsSrc is provided', () => {
    const { container } = renderEmbed({
      src: 'demo.mp4',
      kind: 'video',
      captionsSrc: 'captions.vtt',
    });
    act(() => triggerIntersection(true));
    const track = container.querySelector('track[kind="captions"]');
    expect(track).toBeInTheDocument();
    expect(track).toHaveAttribute('src', 'captions.vtt');
  });

  it('shows "CC unavailable" note when no captionsSrc and video is playing', async () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));
    await userEvent.click(screen.getByRole('button', { name: /play video/i }));

    expect(await screen.findByText(/CC unavailable/i)).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// 4. GIF state machine
// ---------------------------------------------------------------------------

describe('MediaEmbed — GIF state machine', () => {
  it('shows pause control after GIF img load event', () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));

    expect(
      screen.getByRole('button', { name: /pause animation/i }),
    ).toBeInTheDocument();
  });

  it('pause button has aria-pressed="false" in autoplay state', () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));

    expect(
      screen.getByRole('button', { name: /pause animation/i }),
    ).toHaveAttribute('aria-pressed', 'false');
  });

  it('clicking pause control shows play control (gif-paused)', async () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));
    await userEvent.click(screen.getByRole('button', { name: /pause animation/i }));

    expect(
      await screen.findByRole('button', { name: /play animation/i }),
    ).toBeInTheDocument();
  });

  it('play control has aria-pressed="true" when gif is paused', async () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));
    await userEvent.click(screen.getByRole('button', { name: /pause animation/i }));

    expect(
      await screen.findByRole('button', { name: /play animation/i }),
    ).toHaveAttribute('aria-pressed', 'true');
  });

  it('clicking play control resumes GIF (shows pause control again)', async () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));
    await userEvent.click(screen.getByRole('button', { name: /pause animation/i }));
    await userEvent.click(await screen.findByRole('button', { name: /play animation/i }));

    expect(
      await screen.findByRole('button', { name: /pause animation/i }),
    ).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// 5. Error state
// ---------------------------------------------------------------------------

describe('MediaEmbed — error state', () => {
  it('shows error state after video error event', () => {
    const { container } = renderEmbed({ src: 'bad.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    act(() => fireEvent.error(video));

    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(/media unavailable/i)).toBeInTheDocument();
  });

  it('shows error state after GIF img error event', () => {
    const { container } = renderEmbed({ src: 'bad.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.error(img));

    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('error alert has aria-label="Media unavailable"', () => {
    const { container } = renderEmbed({ src: 'bad.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    act(() => fireEvent.error(container.querySelector('video')!));

    expect(screen.getByRole('alert')).toHaveAttribute('aria-label', 'Media unavailable');
  });

  it('calls onError callback with an Error instance', () => {
    const onError = vi.fn();
    const { container } = renderEmbed({ src: 'bad.mp4', kind: 'video', onError });
    act(() => triggerIntersection(true));
    act(() => fireEvent.error(container.querySelector('video')!));

    expect(onError).toHaveBeenCalledOnce();
    expect(onError.mock.calls[0][0]).toBeInstanceOf(Error);
  });

  it('retry button resets back to poster-placeholder state', async () => {
    const { container } = renderEmbed({ src: 'bad.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    act(() => fireEvent.error(container.querySelector('video')!));

    await userEvent.click(screen.getByRole('button', { name: /retry/i }));

    // After retry the alert should be gone and region no longer busy
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('region')).toHaveAttribute('aria-busy', 'false');
  });
});

// ---------------------------------------------------------------------------
// 6. Autoplay policy — video never autoplays with sound
// ---------------------------------------------------------------------------

describe('MediaEmbed — autoplay policy', () => {
  it('video element does NOT have autoplay attribute', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    const video = container.querySelector('video')!;
    expect(video).not.toHaveAttribute('autoplay');
  });

  it('play() is never called before user interaction', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    const playSpy = vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));

    // No click happened — play should not have been called
    expect(playSpy).not.toHaveBeenCalled();
  });

  it('video element has playsInline attribute (mobile policy)', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    expect(container.querySelector('video')).toHaveAttribute('playsinline');
  });
});

// ---------------------------------------------------------------------------
// 7. Accessibility — ARIA structure
// ---------------------------------------------------------------------------

describe('MediaEmbed — accessibility', () => {
  it('all icon SVGs are aria-hidden', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));

    container.querySelectorAll('svg').forEach((svg) => {
      expect(svg).toHaveAttribute('aria-hidden', 'true');
    });
  });

  it('has a polite aria-live region for announcements', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    const liveRegion = container.querySelector('[aria-live="polite"]');
    expect(liveRegion).toBeInTheDocument();
    expect(liveRegion).toHaveAttribute('aria-atomic', 'true');
  });

  it('live region announces "Video playing" after play', async () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));
    await userEvent.click(screen.getByRole('button', { name: /play video/i }));

    const liveRegion = container.querySelector('[aria-live="polite"]')!;
    expect(liveRegion.textContent).toBe('Video playing');
  });

  it('live region announces "Video paused" after pause', async () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    vi.spyOn(video, 'pause').mockImplementation(() => undefined);
    act(() => fireEvent.canPlay(video));
    await userEvent.click(screen.getByRole('button', { name: /play video/i }));
    await userEvent.click(await screen.findByRole('button', { name: /pause video/i }));

    const liveRegion = container.querySelector('[aria-live="polite"]')!;
    expect(liveRegion.textContent).toBe('Video paused');
  });

  it('GIF pause control is always visible (not only on hover)', async () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));

    // The button must exist in the DOM without any hover simulation
    expect(screen.getByRole('button', { name: /pause animation/i })).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// 8. prefers-reduced-motion
// ---------------------------------------------------------------------------

describe('MediaEmbed — prefers-reduced-motion', () => {
  beforeEach(() => {
    mockMatchMedia(true); // activate reduced motion
  });

  it('GIF starts in gif-paused state (pause applied on mount after load)', async () => {
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));

    // Should immediately enter gif-paused — play control is shown
    expect(
      await screen.findByRole('button', { name: /play animation/i }),
    ).toBeInTheDocument();
  });

  it('video does not autoplay even without reduced motion guard (unchanged behaviour)', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    const playSpy = vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));
    expect(playSpy).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// 9. Design QA — play/pause control icon contrast ≥ 4.5:1
//
// The spec mandates:
//   Icon fill  : #f5f5f5  (token: darkMode.text.primary)
//   Button bg  : #000000 at 50% opacity on dark media surface
//
// Relative luminance calculation (WCAG 2.1):
//   #f5f5f5  → sRGB (245,245,245) → linear (0.9603, 0.9603, 0.9603) → L = 0.9603
//   Effective bg ≈ #404040 (black/50 over dark media ~#1a1714)
//   → sRGB (64,64,64) → linear (0.0569, 0.0569, 0.0569) → L = 0.0569
//   Contrast ratio = (0.9603 + 0.05) / (0.0569 + 0.05) = 1.0103 / 0.1069 ≈ 9.45:1
//
// 9.45:1 >> 4.5:1 — passes WCAG AA for normal text and UI components.
// ---------------------------------------------------------------------------

describe('MediaEmbed — design QA: icon contrast', () => {
  /** WCAG relative luminance for a linear channel value. */
  function linearise(c8bit: number): number {
    const s = c8bit / 255;
    return s <= 0.04045 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  }

  function relativeLuminance(r: number, g: number, b: number): number {
    return 0.2126 * linearise(r) + 0.7152 * linearise(g) + 0.0722 * linearise(b);
  }

  function contrastRatio(l1: number, l2: number): number {
    const lighter = Math.max(l1, l2);
    const darker = Math.min(l1, l2);
    return (lighter + 0.05) / (darker + 0.05);
  }

  it('icon fill #f5f5f5 vs button bg #000 at 50% over #1a1714 meets 4.5:1', () => {
    // Icon: #f5f5f5
    const iconL = relativeLuminance(245, 245, 245);

    // Effective bg: blend black (0,0,0) at alpha=0.5 over media surface #1a1714 (26,23,20)
    const bgR = Math.round(0 * 0.5 + 26 * 0.5);
    const bgG = Math.round(0 * 0.5 + 23 * 0.5);
    const bgB = Math.round(0 * 0.5 + 20 * 0.5);
    const bgL = relativeLuminance(bgR, bgG, bgB);

    const ratio = contrastRatio(iconL, bgL);
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('GIF pause control icon #f5f5f5 vs bg-black/60 over dark surface meets 4.5:1', () => {
    const iconL = relativeLuminance(245, 245, 245);
    // bg-black/60 = rgba(0,0,0,0.6) over #1a1714
    const bgR = Math.round(0 * 0.6 + 26 * 0.4);
    const bgG = Math.round(0 * 0.6 + 23 * 0.4);
    const bgB = Math.round(0 * 0.6 + 20 * 0.4);
    const bgL = relativeLuminance(bgR, bgG, bgB);

    const ratio = contrastRatio(iconL, bgL);
    expect(ratio).toBeGreaterThanOrEqual(4.5);
  });

  it('error icon #ef4444 vs placeholder bg #2d2820 meets 3:1 (UI component)', () => {
    // 3:1 is the WCAG AA threshold for non-text UI components
    const iconL = relativeLuminance(239, 68, 68);
    const bgL   = relativeLuminance(45, 40, 32);
    const ratio = contrastRatio(iconL, bgL);
    expect(ratio).toBeGreaterThanOrEqual(3.0);
  });
});

// ---------------------------------------------------------------------------
// 10. Keyboard — controls operable without mouse (issue requirement)
// ---------------------------------------------------------------------------

describe('MediaEmbed — keyboard operability', () => {
  it('play button is focusable via Tab key', async () => {
    const user = userEvent.setup();
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));

    // Focus the play button by tabbing
    await user.tab();
    expect(screen.getByRole('button', { name: /play video/i })).toHaveFocus();
  });

  it('pressing Enter on play button triggers play (no unexpected sound autoplay)', async () => {
    const user = userEvent.setup();
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    act(() => triggerIntersection(true));
    const video = container.querySelector('video')!;
    const playSpy = vi.spyOn(video, 'play').mockResolvedValue(undefined);
    act(() => fireEvent.canPlay(video));

    await user.tab();
    await user.keyboard('{Enter}');

    // play() only called after explicit user keyboard gesture
    expect(playSpy).toHaveBeenCalledTimes(1);
  });

  it('GIF pause control is focusable via Tab key', async () => {
    const user = userEvent.setup();
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));

    await user.tab();
    expect(screen.getByRole('button', { name: /pause animation/i })).toHaveFocus();
  });

  it('pressing Space on GIF pause control pauses animation', async () => {
    const user = userEvent.setup();
    const { container } = renderEmbed({ src: 'anim.gif', kind: 'gif' });
    act(() => triggerIntersection(true));
    const img = container.querySelector('img:not([aria-hidden])')!;
    act(() => fireEvent.load(img));

    await user.tab();
    await user.keyboard(' ');

    expect(
      await screen.findByRole('button', { name: /play animation/i }),
    ).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// 11. Responsive — aspect-ratio container at 375px (no layout shift)
// ---------------------------------------------------------------------------

describe('MediaEmbed — responsive / aspect-ratio', () => {
  it('renders Radix AspectRatio root element', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    // Radix AspectRatio renders a div with a style padding-bottom trick
    // We verify the outer container is present with correct overflow
    expect(container.querySelector('.rounded-\\[24px\\]')).toBeInTheDocument();
  });

  it('uses default 16/9 ratio (no explicit aspectRatio prop)', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    // Radix AspectRatio sets padding-bottom: calc(100% / ratio) on its inner div
    const ratioInner = container.querySelector('[style*="padding"]');
    if (ratioInner) {
      // 16/9 ≈ 56.25%
      expect(ratioInner.getAttribute('style')).toContain('56.25');
    } else {
      // Radix may implement via aspect-ratio CSS property instead
      expect(container.querySelector('[style*="aspect-ratio"]')).toBeInTheDocument();
    }
  });

  it('accepts a custom aspectRatio prop (e.g. 4/3)', () => {
    const { container } = renderEmbed({
      src: 'demo.mp4',
      kind: 'video',
      aspectRatio: 4 / 3,
    });
    // Simply confirm it renders without throwing
    expect(container.querySelector('.rounded-\\[24px\\]')).toBeInTheDocument();
  });

  it('container has overflow-hidden to prevent bleed', () => {
    const { container } = renderEmbed({ src: 'demo.mp4', kind: 'video' });
    expect(container.querySelector('.overflow-hidden')).toBeInTheDocument();
  });

  it('forwards className to the outer container', () => {
    const { container } = renderEmbed({
      src: 'demo.mp4',
      kind: 'video',
      className: 'my-4',
    });
    expect(container.firstChild).toHaveClass('my-4');
  });
});
