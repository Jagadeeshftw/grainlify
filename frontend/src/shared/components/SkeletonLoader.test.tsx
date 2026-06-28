import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { render } from '@testing-library/react'
import React from 'react'
import { ThemeProvider, type Theme } from '../contexts/ThemeContext'
import { SkeletonLoader } from './SkeletonLoader'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderSkeleton(theme: Theme, props: React.ComponentProps<typeof SkeletonLoader> = {}) {
  localStorage.setItem('theme', theme)
  return render(
    <ThemeProvider>
      <SkeletonLoader {...props} />
    </ThemeProvider>
  )
}

beforeEach(() => {
  localStorage.clear()
  document.documentElement.className = ''
})

afterEach(() => {
  localStorage.clear()
  document.documentElement.className = ''
})

// ---------------------------------------------------------------------------
// Accessibility attributes
// ---------------------------------------------------------------------------

describe('SkeletonLoader — accessibility', () => {
  it('has role="presentation"', () => {
    const { container } = renderSkeleton('light')
    expect(container.firstChild).toHaveAttribute('role', 'presentation')
  })

  it('has aria-hidden="true"', () => {
    const { container } = renderSkeleton('light')
    expect(container.firstChild).toHaveAttribute('aria-hidden', 'true')
  })
})

// ---------------------------------------------------------------------------
// Shimmer child element — light and dark themes show it
// ---------------------------------------------------------------------------

describe('SkeletonLoader — shimmer presence', () => {
  it('renders shimmer child in "light" theme', () => {
    const { container } = renderSkeleton('light')
    const wrapper = container.firstChild as HTMLElement
    expect(wrapper.children.length).toBe(1)
    expect(wrapper.firstElementChild?.className).toContain('animate-shimmer')
  })

  it('renders shimmer child in "dark" theme', () => {
    const { container } = renderSkeleton('dark')
    const wrapper = container.firstChild as HTMLElement
    expect(wrapper.children.length).toBe(1)
    expect(wrapper.firstElementChild?.className).toContain('animate-shimmer')
  })

  it('does NOT render shimmer child in "high-contrast" theme', () => {
    const { container } = renderSkeleton('high-contrast')
    const wrapper = container.firstChild as HTMLElement
    expect(wrapper.children.length).toBe(0)
  })

  it('does NOT render shimmer child in "reduced-motion" theme', () => {
    const { container } = renderSkeleton('reduced-motion')
    const wrapper = container.firstChild as HTMLElement
    expect(wrapper.children.length).toBe(0)
  })
})

// ---------------------------------------------------------------------------
// Background classes per theme
// ---------------------------------------------------------------------------

describe('SkeletonLoader — background classes', () => {
  it('applies solid #333333 background in "high-contrast"', () => {
    const { container } = renderSkeleton('high-contrast')
    expect(container.firstChild).toHaveClass('bg-[#333333]')
  })

  it('applies skeleton-surface class in "high-contrast" for CSS targeting', () => {
    const { container } = renderSkeleton('high-contrast')
    expect(container.firstChild).toHaveClass('skeleton-surface')
  })

  it('applies bg-white/[0.08] in "dark" theme', () => {
    const { container } = renderSkeleton('dark')
    expect(container.firstChild).toHaveClass('bg-white/[0.08]')
  })

  it('applies bg-white/[0.08] in "reduced-motion" theme (dark palette)', () => {
    const { container } = renderSkeleton('reduced-motion')
    expect(container.firstChild).toHaveClass('bg-white/[0.08]')
  })

  it('applies bg-white/[0.12] in "light" theme', () => {
    const { container } = renderSkeleton('light')
    expect(container.firstChild).toHaveClass('bg-white/[0.12]')
  })
})

// ---------------------------------------------------------------------------
// Shimmer gradient direction per palette
// ---------------------------------------------------------------------------

describe('SkeletonLoader — shimmer gradient', () => {
  it('uses dark gradient (via-white/[0.15]) for "dark" theme', () => {
    const { container } = renderSkeleton('dark')
    const shimmer = container.firstChild?.firstChild as HTMLElement
    expect(shimmer.className).toContain('via-white/[0.15]')
  })

  it('uses light gradient (via-white/[0.25]) for "light" theme', () => {
    const { container } = renderSkeleton('light')
    const shimmer = container.firstChild?.firstChild as HTMLElement
    expect(shimmer.className).toContain('via-white/[0.25]')
  })
})

// ---------------------------------------------------------------------------
// Shape variants
// ---------------------------------------------------------------------------

describe('SkeletonLoader — shape variants', () => {
  it('applies rounded-full for variant="circle"', () => {
    const { container } = renderSkeleton('light', { variant: 'circle' })
    expect(container.firstChild).toHaveClass('rounded-full')
  })

  it('applies rounded-[100px] for variant="text"', () => {
    const { container } = renderSkeleton('light', { variant: 'text' })
    expect(container.firstChild).toHaveClass('rounded-[100px]')
  })

  it('applies rounded-[12px] for default variant', () => {
    const { container } = renderSkeleton('light')
    expect(container.firstChild).toHaveClass('rounded-[12px]')
  })

  it('applies rounded-[12px] for explicit variant="default"', () => {
    const { container } = renderSkeleton('light', { variant: 'default' })
    expect(container.firstChild).toHaveClass('rounded-[12px]')
  })
})

// ---------------------------------------------------------------------------
// Custom dimensions
// ---------------------------------------------------------------------------

describe('SkeletonLoader — dimensions', () => {
  it('applies custom width via style prop', () => {
    const { container } = renderSkeleton('light', { width: '200px' })
    expect((container.firstChild as HTMLElement).style.width).toBe('200px')
  })

  it('applies custom height via style prop', () => {
    const { container } = renderSkeleton('light', { height: '48px' })
    expect((container.firstChild as HTMLElement).style.height).toBe('48px')
  })

  it('applies both width and height together', () => {
    const { container } = renderSkeleton('light', { width: '100%', height: '24px' })
    const el = container.firstChild as HTMLElement
    expect(el.style.width).toBe('100%')
    expect(el.style.height).toBe('24px')
  })

  it('has no inline style when neither width nor height is provided', () => {
    const { container } = renderSkeleton('light')
    const el = container.firstChild as HTMLElement
    expect(el.style.width).toBe('')
    expect(el.style.height).toBe('')
  })
})

// ---------------------------------------------------------------------------
// className passthrough
// ---------------------------------------------------------------------------

describe('SkeletonLoader — className passthrough', () => {
  it('forwards a custom className to the root element', () => {
    const { container } = renderSkeleton('light', { className: 'my-custom-class' })
    expect(container.firstChild).toHaveClass('my-custom-class')
  })

  it('renders without error when className is undefined', () => {
    expect(() => renderSkeleton('light')).not.toThrow()
  })
})

// ---------------------------------------------------------------------------
// Structural invariants
// ---------------------------------------------------------------------------

describe('SkeletonLoader — structure', () => {
  it('always has overflow-hidden on the wrapper', () => {
    for (const theme of ['light', 'dark', 'high-contrast', 'reduced-motion'] as Theme[]) {
      const { container } = renderSkeleton(theme)
      expect(container.firstChild).toHaveClass('overflow-hidden')
    }
  })

  it('always has relative positioning on the wrapper', () => {
    for (const theme of ['light', 'dark', 'high-contrast', 'reduced-motion'] as Theme[]) {
      const { container } = renderSkeleton(theme)
      expect(container.firstChild).toHaveClass('relative')
    }
  })
})
