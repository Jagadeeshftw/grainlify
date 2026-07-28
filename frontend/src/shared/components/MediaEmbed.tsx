/**
 * MediaEmbed
 *
 * First-class embed surface for demo videos (MP4/WebM) and animated GIFs
 * referenced from project READMEs or a media gallery.
 *
 * Features:
 * - Fixed 16:9 aspect-ratio container (no layout shift at any breakpoint)
 * - Letterboxing via object-fit:contain + blurred background layer for
 *   mismatched source ratios
 * - IntersectionObserver lazy-load — media src is only set when the component
 *   is 200px from the viewport
 * - Autoplay policy: GIFs autoplay muted-looping with a visible pause control;
 *   videos NEVER autoplay with sound, require an explicit user tap
 * - prefers-reduced-motion: GIFs start paused, transitions run at 0ms
 * - Full state machine: poster-placeholder → loading → loaded-paused / playing
 *   (videos) or gif-autoplay-with-pause-control ↔ gif-paused (GIFs)
 * - error-unavailable state with retry
 * - WCAG 2.1 AA: keyboard-operable controls, aria-labels, aria-pressed,
 *   aria-live region, focus ring from design tokens
 *
 * Design tokens used:
 *   Container bg (dark)  : rgba(255,255,255,0.08)   [glassMedium]
 *   Container border     : rgba(255,255,255,0.08)   [darkMode.border.subtle]
 *   Border radius        : rounded-[24px]           [borderRadius.3xl]
 *   Poster bg (dark)     : #2d2820                  [surfaceSecondary]
 *   Control icon active  : #c9983a                  [darkMode.accent.primary]
 *   Control icon rest    : #f5f5f5                  [darkMode.text.primary]
 *   Focus ring           : #f1b400                  [darkMode.interactive.focusRing]
 *   Error icon           : #ef4444                  [color.semantic.error.500]
 *   Motion duration      : 300ms                    [motion.durations.normal]
 *   Motion easing        : cubic-bezier(0,0,0.2,1)  [motion.easing.easeOut]
 *
 * @see design/specs/video-gif-embed-spec.md
 */

import React, {
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import * as AspectRatio from '@radix-ui/react-aspect-ratio';
import { SkeletonLoader } from './SkeletonLoader';
import { useTheme } from '../contexts/ThemeContext';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type MediaEmbedKind = 'video' | 'gif';

export type MediaEmbedState =
  | 'poster-placeholder'
  | 'loading'
  | 'loaded-paused'
  | 'playing'
  | 'gif-autoplay-with-pause-control'
  | 'gif-paused'
  | 'error-unavailable';

export interface MediaEmbedProps {
  /** URL of the video (MP4/WebM) or animated GIF */
  src: string;
  /** Detected or supplied media kind */
  kind: MediaEmbedKind;
  /** Optional poster image URL for videos */
  poster?: string;
  /** Optional static preview image URL for GIFs */
  gifPoster?: string;
  /** Human-readable title for aria-label and screen-reader announcements */
  title?: string;
  /** URL to a WebVTT captions file */
  captionsSrc?: string;
  /**
   * Aspect ratio as width / height.
   * @default 16/9
   */
  aspectRatio?: number;
  /** Optional className for the outer container */
  className?: string;
  /** Called when the component enters the error state */
  onError?: (error: Error) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Returns true when the OS prefers reduced motion. */
function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined') return false;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/** Transition duration respecting prefers-reduced-motion. */
function transitionDuration(): string {
  return prefersReducedMotion() ? '0ms' : '300ms';
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function MediaEmbed({
  src,
  kind,
  poster,
  gifPoster,
  title,
  captionsSrc,
  aspectRatio = 16 / 9,
  className = '',
  onError,
}: MediaEmbedProps) {
  const { theme } = useTheme();
  const dark = theme === 'dark' || theme === 'high-contrast';

  const [mediaState, setMediaState] = useState<MediaEmbedState>('poster-placeholder');
  const [isLoaded, setIsLoaded] = useState(false);

  // Live-region announcement text (screen readers)
  const [announcement, setAnnouncement] = useState('');

  // Sentinel ref for IntersectionObserver
  const sentinelRef = useRef<HTMLDivElement>(null);
  // Video element ref
  const videoRef = useRef<HTMLVideoElement>(null);
  // GIF img element ref
  const gifRef = useRef<HTMLImageElement>(null);
  // Whether src has been committed to the element (lazy-load guard)
  const srcCommittedRef = useRef(false);
  // Canvas ref for GIF frame capture (pause simulation)
  const canvasRef = useRef<HTMLCanvasElement>(null);
  // Frozen frame data-URI (used when pausing a GIF)
  const frozenFrameRef = useRef<string | null>(null);
  // Original GIF src preserved for resume
  const originalGifSrcRef = useRef<string>(src);

  // ---------------------------------------------------------------------------
  // Intersection observer — lazy-load trigger
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && !srcCommittedRef.current) {
          srcCommittedRef.current = true;
          observer.disconnect();
          setMediaState('loading');

          if (kind === 'video' && videoRef.current) {
            videoRef.current.setAttribute('src', src);
            videoRef.current.load();
          } else if (kind === 'gif' && gifRef.current) {
            gifRef.current.setAttribute('src', src);
          }
        }
      },
      { rootMargin: '200px', threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [src, kind]);

  // ---------------------------------------------------------------------------
  // Pause video when it scrolls out of viewport (playing state only)
  // ---------------------------------------------------------------------------
  const containerRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (kind !== 'video') return;
    const container = containerRef.current;
    if (!container) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries[0].isIntersecting && mediaState === 'playing') {
          videoRef.current?.pause();
          setMediaState('loaded-paused');
          setAnnouncement('Video paused');
        }
      },
      { threshold: 0 },
    );

    observer.observe(container);
    return () => observer.disconnect();
  }, [kind, mediaState]);

  // ---------------------------------------------------------------------------
  // GIF: apply reduced-motion on mount
  // ---------------------------------------------------------------------------
  useEffect(() => {
    if (kind === 'gif' && mediaState === 'gif-autoplay-with-pause-control') {
      if (prefersReducedMotion()) {
        pauseGif();
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind, mediaState]);

  // ---------------------------------------------------------------------------
  // Video event handlers
  // ---------------------------------------------------------------------------
  const handleVideoCanPlay = useCallback(() => {
    setIsLoaded(true);
    setMediaState('loaded-paused');
  }, []);

  const handleVideoError = useCallback(() => {
    const err = new Error(`Failed to load video: ${src}`);
    setMediaState('error-unavailable');
    onError?.(err);
  }, [src, onError]);

  const handleVideoEnded = useCallback(() => {
    setMediaState('loaded-paused');
    setAnnouncement('Video ended');
  }, []);

  // ---------------------------------------------------------------------------
  // GIF event handlers
  // ---------------------------------------------------------------------------
  const handleGifLoad = useCallback(() => {
    setIsLoaded(true);
    const initialState = prefersReducedMotion()
      ? 'gif-paused'
      : 'gif-autoplay-with-pause-control';
    setMediaState(initialState);
  }, []);

  const handleGifError = useCallback(() => {
    const err = new Error(`Failed to load GIF: ${src}`);
    setMediaState('error-unavailable');
    onError?.(err);
  }, [src, onError]);

  // ---------------------------------------------------------------------------
  // Video controls
  // ---------------------------------------------------------------------------
  const handlePlayVideo = useCallback(() => {
    if (!videoRef.current) return;
    videoRef.current.play().then(() => {
      setMediaState('playing');
      setAnnouncement('Video playing');
    }).catch(() => {
      // play() rejected (e.g. browser policy) — stay paused
    });
  }, []);

  const handlePauseVideo = useCallback(() => {
    videoRef.current?.pause();
    setMediaState('loaded-paused');
    setAnnouncement('Video paused');
  }, []);

  // ---------------------------------------------------------------------------
  // GIF pause/resume via canvas frame capture
  // ---------------------------------------------------------------------------
  const pauseGif = useCallback(() => {
    const img = gifRef.current;
    const canvas = canvasRef.current;
    if (!img || !canvas) {
      setMediaState('gif-paused');
      return;
    }
    // Capture current frame
    try {
      canvas.width = img.naturalWidth || img.width;
      canvas.height = img.naturalHeight || img.height;
      const ctx = canvas.getContext('2d');
      if (ctx) {
        ctx.drawImage(img, 0, 0);
        frozenFrameRef.current = canvas.toDataURL('image/png');
        img.src = frozenFrameRef.current;
      }
    } catch {
      // Canvas taint (cross-origin) — just blank the src to stop animation
      img.src = gifPoster || '';
    }
    setMediaState('gif-paused');
  }, [gifPoster]);

  const resumeGif = useCallback(() => {
    const img = gifRef.current;
    if (!img) return;
    img.src = originalGifSrcRef.current;
    setMediaState('gif-autoplay-with-pause-control');
  }, []);

  // ---------------------------------------------------------------------------
  // Retry
  // ---------------------------------------------------------------------------
  const handleRetry = useCallback(() => {
    srcCommittedRef.current = false;
    setIsLoaded(false);
    setMediaState('poster-placeholder');
    if (videoRef.current) videoRef.current.removeAttribute('src');
    if (gifRef.current) gifRef.current.removeAttribute('src');
    // Re-trigger observer by re-mounting sentinel (done via state reset + effect)
  }, []);

  // ---------------------------------------------------------------------------
  // Derived display flags
  // ---------------------------------------------------------------------------
  const showSkeleton = mediaState === 'loading';
  const showPoster =
    kind === 'video' &&
    (mediaState === 'poster-placeholder' || mediaState === 'loading' || mediaState === 'loaded-paused');
  const showPlayButton = kind === 'video' && mediaState === 'loaded-paused';
  const showPauseButton = kind === 'video' && mediaState === 'playing';
  const showGifPauseControl = mediaState === 'gif-autoplay-with-pause-control';
  const showGifPlayControl = mediaState === 'gif-paused';
  const showError = mediaState === 'error-unavailable';

  // ---------------------------------------------------------------------------
  // Styles
  // ---------------------------------------------------------------------------
  const containerBg = dark
    ? 'bg-white/[0.08] border border-white/[0.08]'
    : 'bg-white/[0.15] border border-white/[0.25]';

  const placeholderBg = dark ? 'bg-[#2d2820]' : 'bg-[#e7e5e4]';

  // Focus ring: token darkMode.interactive.focusRing = #f1b400
  const focusRingClass =
    'focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]';

  const transitionStyle: React.CSSProperties = {
    transition: `opacity ${transitionDuration()} cubic-bezier(0,0,0.2,1)`,
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------
  const ariaLabel = title
    ? `${kind === 'gif' ? 'Animated GIF' : 'Video'}: ${title}`
    : kind === 'gif'
    ? 'Animated GIF'
    : 'Video';

  return (
    <div
      ref={containerRef}
      className={`relative rounded-[24px] overflow-hidden ${containerBg} ${className}`}
      role="region"
      aria-label={ariaLabel}
      aria-busy={mediaState === 'loading'}
    >
      {/* Hidden live region for screen-reader announcements */}
      <div
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {announcement}
      </div>

      <AspectRatio.Root ratio={aspectRatio}>
        {/* ------------------------------------------------------------------ */}
        {/* Poster-placeholder / loading background                             */}
        {/* ------------------------------------------------------------------ */}
        {(mediaState === 'poster-placeholder' || showSkeleton) && (
          <div
            className={`absolute inset-0 ${placeholderBg}`}
            aria-label="Loading media"
          >
            {showSkeleton && (
              <SkeletonLoader
                className="absolute inset-0 w-full h-full rounded-none"
                style={{ transitionStyle } as React.CSSProperties}
              />
            )}
          </div>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* Blurred letterbox background layer (mismatched aspect ratios)       */}
        {/* ------------------------------------------------------------------ */}
        {isLoaded && (kind === 'video' ? (
          poster ? (
            <img
              src={poster}
              alt=""
              aria-hidden="true"
              className="absolute inset-0 w-full h-full object-cover"
              style={{ filter: 'blur(20px)', opacity: 0.25 }}
            />
          ) : null
        ) : (
          <img
            ref={undefined}
            src={src}
            alt=""
            aria-hidden="true"
            className="absolute inset-0 w-full h-full object-cover"
            style={{ filter: 'blur(20px)', opacity: 0.25 }}
          />
        ))}

        {/* ------------------------------------------------------------------ */}
        {/* Poster overlay for video (before/while paused)                      */}
        {/* ------------------------------------------------------------------ */}
        {showPoster && poster && (
          <img
            src={poster}
            alt=""
            aria-hidden="true"
            className="absolute inset-0 w-full h-full object-contain"
            style={transitionStyle}
          />
        )}

        {/* ------------------------------------------------------------------ */}
        {/* Video element                                                        */}
        {/* ------------------------------------------------------------------ */}
        {kind === 'video' && (
          <video
            ref={videoRef}
            aria-hidden="true"
            className="absolute inset-0 w-full h-full object-contain"
            style={{
              opacity: mediaState === 'playing' ? 1 : 0,
              ...transitionStyle,
            }}
            playsInline
            preload="none"
            onCanPlay={handleVideoCanPlay}
            onError={handleVideoError}
            onEnded={handleVideoEnded}
            onPause={() => {
              if (mediaState === 'playing') {
                setMediaState('loaded-paused');
                setAnnouncement('Video paused');
              }
            }}
          >
            {captionsSrc && (
              <track kind="captions" src={captionsSrc} default />
            )}
          </video>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* GIF element                                                          */}
        {/* ------------------------------------------------------------------ */}
        {kind === 'gif' && (
          <>
            {/* Hidden canvas for frame capture */}
            <canvas ref={canvasRef} className="hidden" aria-hidden="true" />
            <img
              ref={gifRef}
              alt={title || ''}
              className="absolute inset-0 w-full h-full object-contain"
              style={{
                opacity: isLoaded ? 1 : 0,
                ...transitionStyle,
              }}
              onLoad={handleGifLoad}
              onError={handleGifError}
            />
          </>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* Video: centred play button (loaded-paused)                           */}
        {/* ------------------------------------------------------------------ */}
        {showPlayButton && (
          <div className="absolute inset-0 flex items-center justify-center">
            <button
              type="button"
              onClick={handlePlayVideo}
              aria-label="Play video"
              className={`
                w-12 h-12 rounded-full bg-black/50 flex items-center justify-center
                hover:bg-black/70 active:scale-95
                ${focusRingClass}
              `}
              style={{ transition: `transform ${transitionDuration()} cubic-bezier(0,0,0.2,1)` }}
            >
              {/* Play icon */}
              <svg
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                aria-hidden="true"
              >
                <polygon points="5,3 19,12 5,21" fill="#f5f5f5" />
              </svg>
            </button>
          </div>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* Video: bottom control bar (playing)                                  */}
        {/* ------------------------------------------------------------------ */}
        {showPauseButton && (
          <div
            className="absolute bottom-0 left-0 right-0 h-10 bg-black/40 flex items-center px-3 gap-2"
            style={transitionStyle}
          >
            <button
              type="button"
              onClick={handlePauseVideo}
              aria-label="Pause video"
              className={`
                w-8 h-8 rounded flex items-center justify-center
                hover:bg-white/10 active:scale-95
                ${focusRingClass}
              `}
            >
              {/* Pause icon */}
              <svg
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                aria-hidden="true"
              >
                <rect x="6" y="4" width="4" height="16" fill="#f5f5f5" />
                <rect x="14" y="4" width="4" height="16" fill="#f5f5f5" />
              </svg>
            </button>

            {/* Captions unavailable note — placeholder until VTT supplied */}
            {!captionsSrc && (
              <span className="ml-auto text-[11px] text-white/50 select-none">
                CC unavailable
              </span>
            )}
          </div>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* GIF: pause button (gif-autoplay-with-pause-control)                  */}
        {/* ------------------------------------------------------------------ */}
        {showGifPauseControl && (
          <button
            type="button"
            onClick={pauseGif}
            aria-label="Pause animation"
            aria-pressed={false}
            className={`
              absolute top-2 right-2 w-8 h-8 rounded-[8px] bg-black/60
              flex items-center justify-center z-10
              hover:bg-black/80 active:scale-95
              ${focusRingClass}
            `}
            style={{ transition: `transform ${transitionDuration()} cubic-bezier(0,0,0.2,1)` }}
          >
            {/* Pause icon */}
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              aria-hidden="true"
            >
              <rect x="6" y="4" width="4" height="16" fill="#f5f5f5" />
              <rect x="14" y="4" width="4" height="16" fill="#f5f5f5" />
            </svg>
          </button>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* GIF: play button (gif-paused)                                        */}
        {/* ------------------------------------------------------------------ */}
        {showGifPlayControl && (
          <button
            type="button"
            onClick={resumeGif}
            aria-label="Play animation"
            aria-pressed={true}
            className={`
              absolute top-2 right-2 w-8 h-8 rounded-[8px] bg-black/60
              flex items-center justify-center z-10
              hover:bg-black/80 active:scale-95
              ${focusRingClass}
            `}
            style={{ transition: `transform ${transitionDuration()} cubic-bezier(0,0,0.2,1)` }}
          >
            {/* Play icon */}
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              aria-hidden="true"
            >
              <polygon points="5,3 19,12 5,21" fill="#f5f5f5" />
            </svg>
          </button>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* Error / unavailable state                                            */}
        {/* ------------------------------------------------------------------ */}
        {showError && (
          <div
            role="alert"
            aria-label="Media unavailable"
            className={`
              absolute inset-0 flex flex-col items-center justify-center gap-3
              ${placeholderBg}
            `}
          >
            {/* Error icon — color.semantic.error.500 = #ef4444 */}
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M12 2L2 20h20L12 2z"
                stroke="#ef4444"
                strokeWidth="2"
                strokeLinejoin="round"
              />
              <line
                x1="12"
                y1="10"
                x2="12"
                y2="14"
                stroke="#ef4444"
                strokeWidth="2"
                strokeLinecap="round"
              />
              <circle cx="12" cy="17" r="1" fill="#ef4444" />
            </svg>
            <span
              className={`text-[14px] ${dark ? 'text-[#a8a29e]' : 'text-[#78716c]'}`}
            >
              Media unavailable
            </span>
            <button
              type="button"
              onClick={handleRetry}
              className={`
                mt-1 px-4 py-1.5 rounded-[8px] text-[13px] font-medium
                border ${dark
                  ? 'border-white/20 text-[#f5f5f5] hover:bg-white/10'
                  : 'border-black/20 text-[#2d2820] hover:bg-black/5'
                }
                ${focusRingClass}
              `}
            >
              Retry
            </button>
          </div>
        )}

        {/* ------------------------------------------------------------------ */}
        {/* Sentinel for IntersectionObserver (1px, invisible)                  */}
        {/* ------------------------------------------------------------------ */}
        {mediaState === 'poster-placeholder' && (
          <div
            ref={sentinelRef}
            className="absolute bottom-0 left-0 w-px h-px pointer-events-none"
            aria-hidden="true"
          />
        )}
      </AspectRatio.Root>
    </div>
  );
}
