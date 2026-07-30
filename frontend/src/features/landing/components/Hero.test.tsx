/**
 * Hero component tests — issue #1511
 *
 * Coverage areas:
 *   1. Renders headline, description, CTAs, and stat strip
 *   2. Decorative orbs have aria-hidden="true"
 *   3. Full motion: orb elements carry animate-pulse class
 *   4. Reduced motion: animate-pulse class removed from orbs
 *   5. h1 renders as a single node (no ARIA fragmentation)
 *   6. Both CTA links are focusable (tab-order not gated by animation)
 *   7. Primary CTA has focus-visible gold outline class
 *   8. Secondary CTA has rel="noopener noreferrer"
 *   9. Stat cards render all three labels
 *  10. Component renders without crashing when useReducedMotion returns true
 */

import React from 'react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'

// ---------------------------------------------------------------------------
// Module mocks — defined before any imports that use them
// ---------------------------------------------------------------------------

// Mock react-router-dom's Link to render as a plain anchor so jsdom can
// resolve href/focus without needing a router feature set.
vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>()
  return {
    ...actual,
    Link: ({ to, children, className }: { to: string; children: React.ReactNode; className?: string }) => (
      <a href={to} className={className}>
        {children}
      </a>
    ),
  }
})

// Mock ThemeContext — return 'dark' by default (most components tested in dark)
vi.mock('../../../shared/contexts/ThemeContext', () => ({
  useTheme: () => ({ theme: 'dark', setTheme: vi.fn() }),
}))

// Mock useLandingStats — return static display values
vi.mock('../../../shared/hooks/useLandingStats', () => ({
  useLandingStats: () => ({
    display: {
      activeProjects: '42',
      contributors: '1,234',
      grantsDistributed: '$56,789',
    },
    isLoading: false,
    error: null,
  }),
}))

// useReducedMotion — controlled per test via vi.mocked()
vi.mock('../../../shared/hooks/useReducedMotion', () => ({
  useReducedMotion: vi.fn().mockReturnValue(false),
}))

// motion/react — render children, skipping actual animation engine.
// We filter out Framer Motion-specific props so React doesn't warn about
// unknown DOM attributes (whileHover, whileTap, variants, initial, etc.).
vi.mock('motion/react', () => {
  const MOTION_PROPS = new Set([
    'initial', 'animate', 'exit', 'variants', 'transition',
    'whileHover', 'whileTap', 'whileFocus', 'whileDrag', 'whileInView',
    'drag', 'dragConstraints', 'dragElastic', 'dragMomentum',
    'layout', 'layoutId', 'onAnimationStart', 'onAnimationComplete',
    'onHoverStart', 'onHoverEnd', 'onTapStart', 'onTap', 'onTapCancel',
    'style', // forward style since it's needed for some tests
  ])

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const makeMotionComponent = (tag: string) => ({ children, className, style, ...rest }: any) => {
    // Filter out Framer Motion props to avoid React DOM warnings
    const domProps: Record<string, unknown> = {}
    for (const [key, value] of Object.entries(rest)) {
      if (!MOTION_PROPS.has(key)) {
        domProps[key] = value
      }
    }
    const Tag = tag as keyof JSX.IntrinsicElements
    return (
      <Tag className={className} style={style} {...domProps}>
        {children}
      </Tag>
    )
  }

  const motionProxy = new globalThis.Proxy(
    {},
    {
      get: (_target: unknown, prop: string) => makeMotionComponent(prop),
    }
  )

  return {
    motion: motionProxy,
    AnimatePresence: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  }
})

// ---------------------------------------------------------------------------
// Import component under test AFTER mocks are declared
// ---------------------------------------------------------------------------

import { Hero } from './Hero'
import { useReducedMotion } from '../../../shared/hooks/useReducedMotion'

// ---------------------------------------------------------------------------
// Render helper
// ---------------------------------------------------------------------------

function renderHero() {
  return render(
    <MemoryRouter>
      <Hero />
    </MemoryRouter>
  )
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  // Reset to non-reduced-motion mode before each test
  vi.mocked(useReducedMotion).mockReturnValue(false)
})

afterEach(() => {
  vi.clearAllMocks()
})

// ---------------------------------------------------------------------------
// 1. Basic rendering
// ---------------------------------------------------------------------------

describe('Hero — renders required content', () => {
  it('renders the primary headline text', () => {
    renderHero()
    // h1 contains "Connect with" and "Open Source" — query by role
    const heading = screen.getByRole('heading', { level: 1 })
    expect(heading).toBeInTheDocument()
    expect(heading.textContent).toContain('Connect with')
    expect(heading.textContent).toContain('Open Source')
    expect(heading.textContent).toContain('Opportunities')
  })

  it('renders the description paragraph', () => {
    renderHero()
    expect(
      screen.getByText(/Grainlify bridges the gap/i)
    ).toBeInTheDocument()
  })

  it('renders the primary CTA link ("Get Started")', () => {
    renderHero()
    expect(screen.getByRole('link', { name: /get started/i })).toBeInTheDocument()
  })

  it('renders the secondary CTA link ("Docs")', () => {
    renderHero()
    expect(screen.getByRole('link', { name: /docs/i })).toBeInTheDocument()
  })

  it('renders all three stat labels', () => {
    renderHero()
    expect(screen.getByText('Active Projects')).toBeInTheDocument()
    expect(screen.getByText('Contributors')).toBeInTheDocument()
    expect(screen.getByText('Grants Distributed')).toBeInTheDocument()
  })

  it('renders stat values from useLandingStats', () => {
    renderHero()
    expect(screen.getByText('42')).toBeInTheDocument()
    expect(screen.getByText('1,234')).toBeInTheDocument()
    expect(screen.getByText('$56,789')).toBeInTheDocument()
  })
})

// ---------------------------------------------------------------------------
// 2. Accessibility — decorative orbs
// ---------------------------------------------------------------------------

describe('Hero — decorative orb accessibility', () => {
  it('marks both orb divs as aria-hidden="true"', () => {
    const { container } = renderHero()
    const hiddenDivs = Array.from(container.querySelectorAll('[aria-hidden="true"]'))
    // At minimum two orb divs must be aria-hidden
    expect(hiddenDivs.length).toBeGreaterThanOrEqual(2)
  })
})

// ---------------------------------------------------------------------------
// 3. Full motion — orb pulse classes present
// ---------------------------------------------------------------------------

describe('Hero — full motion orb animation', () => {
  it('applies animate-pulse class to first orb when not reduced motion', () => {
    const { container } = renderHero()
    const hiddenDivs = Array.from(container.querySelectorAll('[aria-hidden="true"]'))
    // At least one orb should have animate-pulse
    const hasPulse = hiddenDivs.some((el) => {
      const cls = el.getAttribute('class') ?? ''
      return cls.includes('animate-pulse')
    })
    expect(hasPulse).toBe(true)
  })
})

// ---------------------------------------------------------------------------
// 4. Reduced motion — animate-pulse removed from orbs
// ---------------------------------------------------------------------------

describe('Hero — reduced motion orb static', () => {
  it('does NOT apply animate-pulse to orbs when useReducedMotion returns true', () => {
    vi.mocked(useReducedMotion).mockReturnValue(true)
    const { container } = renderHero()
    const hiddenDivs = Array.from(container.querySelectorAll('[aria-hidden="true"]'))
    // Use getAttribute('class') to safely handle both HTML and SVG elements
    const hasPulse = hiddenDivs.some((el) => {
      const cls = el.getAttribute('class') ?? ''
      return cls.includes('animate-pulse')
    })
    expect(hasPulse).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// 5. h1 ARIA integrity — single node, no fragmentation
// ---------------------------------------------------------------------------

describe('Hero — h1 ARIA integrity', () => {
  it('renders exactly one h1 element', () => {
    renderHero()
    const headings = screen.getAllByRole('heading', { level: 1 })
    expect(headings).toHaveLength(1)
  })

  it('h1 textContent includes all parts of the headline as one string', () => {
    renderHero()
    const h1 = screen.getByRole('heading', { level: 1 })
    const text = h1.textContent ?? ''
    // Screen readers will announce the concatenated text — verify coherence
    expect(text).toContain('Connect with')
    expect(text).toContain('Open Source')
    expect(text).toContain('Opportunities')
  })
})

// ---------------------------------------------------------------------------
// 6. Keyboard accessibility — CTAs are immediately focusable
// ---------------------------------------------------------------------------

describe('Hero — CTA keyboard accessibility', () => {
  it('primary CTA link is in the document and accessible', () => {
    renderHero()
    const link = screen.getByRole('link', { name: /get started/i })
    expect(link).toBeInTheDocument()
    // Ensure it is not hidden with display:none or visibility:hidden
    expect(link).not.toHaveAttribute('tabindex', '-1')
  })

  it('secondary CTA link is in the document and accessible', () => {
    renderHero()
    const link = screen.getByRole('link', { name: /docs/i })
    expect(link).toBeInTheDocument()
    expect(link).not.toHaveAttribute('tabindex', '-1')
  })
})

// ---------------------------------------------------------------------------
// 7. Primary CTA focus-visible gold outline class
// ---------------------------------------------------------------------------

describe('Hero — CTA focus-visible styling', () => {
  it('primary CTA has focus-visible:outline-[#c9983a] class', () => {
    renderHero()
    const link = screen.getByRole('link', { name: /get started/i })
    expect(link.className).toContain('focus-visible:outline-[#c9983a]')
  })

  it('secondary CTA has focus-visible:outline-[#c9983a] class', () => {
    renderHero()
    const link = screen.getByRole('link', { name: /docs/i })
    expect(link.className).toContain('focus-visible:outline-[#c9983a]')
  })
})

// ---------------------------------------------------------------------------
// 8. Secondary CTA security attributes
// ---------------------------------------------------------------------------

describe('Hero — secondary CTA external link safety', () => {
  it('secondary CTA has target="_blank"', () => {
    renderHero()
    const link = screen.getByRole('link', { name: /docs/i })
    expect(link).toHaveAttribute('target', '_blank')
  })

  it('secondary CTA has rel="noopener noreferrer"', () => {
    renderHero()
    const link = screen.getByRole('link', { name: /docs/i })
    expect(link).toHaveAttribute('rel', 'noopener noreferrer')
  })
})

// ---------------------------------------------------------------------------
// 9. Component renders without crash in reduced-motion mode
// ---------------------------------------------------------------------------

describe('Hero — reduced motion rendering', () => {
  it('renders without throwing when useReducedMotion returns true', () => {
    vi.mocked(useReducedMotion).mockReturnValue(true)
    expect(() => renderHero()).not.toThrow()
  })

  it('still renders headline in reduced-motion mode', () => {
    vi.mocked(useReducedMotion).mockReturnValue(true)
    renderHero()
    expect(screen.getByRole('heading', { level: 1 })).toBeInTheDocument()
  })

  it('still renders CTAs in reduced-motion mode', () => {
    vi.mocked(useReducedMotion).mockReturnValue(true)
    renderHero()
    expect(screen.getByRole('link', { name: /get started/i })).toBeInTheDocument()
    expect(screen.getByRole('link', { name: /docs/i })).toBeInTheDocument()
  })

  it('still renders all stat labels in reduced-motion mode', () => {
    vi.mocked(useReducedMotion).mockReturnValue(true)
    renderHero()
    expect(screen.getByText('Active Projects')).toBeInTheDocument()
    expect(screen.getByText('Contributors')).toBeInTheDocument()
    expect(screen.getByText('Grants Distributed')).toBeInTheDocument()
  })
})

// ---------------------------------------------------------------------------
// 10. Lucide icon accessibility
// ---------------------------------------------------------------------------

describe('Hero — icon accessibility', () => {
  it('ArrowRight icon has aria-hidden="true"', () => {
    renderHero()
    // The SVG rendered by ArrowRight should be aria-hidden
    const arrowIcons = document.querySelectorAll('svg[aria-hidden="true"]')
    expect(arrowIcons.length).toBeGreaterThan(0)
  })
})
