import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useResponsiveBreakpoint, useReducedMotion, usePrefersDarkMode } from '../useReducedMotion'

function mockMatchMedia(matches: boolean) {
  return {
    matches,
    media: '',
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }
}

beforeEach(() => {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation(() => mockMatchMedia(false)),
  )
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('useResponsiveBreakpoint', () => {
  it('returns isMobile=true and breakpoint="sm" below 768px', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(true)
        if (query === '(min-width: 768px) and (max-width: 1023px)') return mockMatchMedia(false)
        if (query === '(min-width: 1024px)') return mockMatchMedia(false)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useResponsiveBreakpoint())
    expect(result.current.isMobile).toBe(true)
    expect(result.current.isTablet).toBe(false)
    expect(result.current.isDesktop).toBe(false)
    expect(result.current.breakpoint).toBe('sm')
  })

  it('returns isTablet=true and breakpoint="md" between 768px-1023px', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(false)
        if (query === '(min-width: 768px) and (max-width: 1023px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(false)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useResponsiveBreakpoint())
    expect(result.current.isMobile).toBe(false)
    expect(result.current.isTablet).toBe(true)
    expect(result.current.isDesktop).toBe(false)
    expect(result.current.breakpoint).toBe('md')
  })

  it('returns isDesktop=true and breakpoint="lg" at 1024px+', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(false)
        if (query === '(min-width: 768px) and (max-width: 1023px)') return mockMatchMedia(false)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useResponsiveBreakpoint())
    expect(result.current.isMobile).toBe(false)
    expect(result.current.isTablet).toBe(false)
    expect(result.current.isDesktop).toBe(true)
    expect(result.current.breakpoint).toBe('lg')
  })

  it('returns isLargeDesktop=true and breakpoint="xl" at 1280px+', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1280px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useResponsiveBreakpoint())
    expect(result.current.isLargeDesktop).toBe(true)
    expect(result.current.isDesktop).toBe(true)
    expect(result.current.isTablet).toBe(false)
    expect(result.current.isMobile).toBe(false)
    expect(result.current.breakpoint).toBe('xl')
  })

  it('keeps isDesktop=false when explicitly mobile (backward compat)', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useResponsiveBreakpoint())
    expect(result.current.isDesktop).toBe(false)
    expect(result.current.isLargeDesktop).toBe(false)
    expect(result.current.isMobile).toBe(true)
    expect(result.current.breakpoint).toBe('sm')
  })

  it('is deterministic — same value across sequential renders for same viewport', () => {
    const { result, rerender } = renderHook(() => useResponsiveBreakpoint())
    const first = { ...result.current }
    rerender()
    const second = { ...result.current }
    expect(first).toEqual(second)
  })
})

describe('useReducedMotion', () => {
  it('returns false by default', () => {
    const { result } = renderHook(() => useReducedMotion())
    expect(result.current).toBe(false)
  })

  it('returns true when prefers-reduced-motion matches', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(prefers-reduced-motion: reduce)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useReducedMotion())
    expect(result.current).toBe(true)
  })
})

describe('usePrefersDarkMode', () => {
  it('returns false by default', () => {
    const { result } = renderHook(() => usePrefersDarkMode())
    expect(result.current).toBe(false)
  })

  it('returns true when prefers-color-scheme matches', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(prefers-color-scheme: dark)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => usePrefersDarkMode())
    expect(result.current).toBe(true)
  })
})
