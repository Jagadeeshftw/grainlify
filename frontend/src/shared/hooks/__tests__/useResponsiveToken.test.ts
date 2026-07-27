import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useResponsiveToken } from '../useResponsiveToken'

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

describe('useResponsiveToken', () => {
  it('returns the value for the current breakpoint when defined', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 'mobile-value', md: 'tablet-value', lg: 'desktop-value' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'fallback'))
    expect(result.current).toBe('mobile-value')
  })

  it('falls back to the nearest smaller breakpoint when current is not defined', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(false)
        if (query === '(min-width: 768px) and (max-width: 1023px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 'mobile-value' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'fallback'))
    expect(result.current).toBe('mobile-value')
  })

  it('returns the default when no breakpoint token is defined', () => {
    const { result } = renderHook(() => useResponsiveToken({}, 'fallback'))
    expect(result.current).toBe('fallback')
  })

  it('uses the exact breakpoint match when available', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 1, md: 2, lg: 3 }
    const { result } = renderHook(() => useResponsiveToken(tokens, 0))
    expect(result.current).toBe(3)
  })

  it('skips over undefined breakpoints to find the nearest defined one', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 'a', lg: 'c' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'z'))
    expect(result.current).toBe('c')
  })

  it('is deterministic — returns same value on every render for same breakpoint', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 'x', md: 'y' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'z'))
    expect(result.current).toBe('x')

    const { result: result2 } = renderHook(() => useResponsiveToken(tokens, 'z'))
    expect(result2.current).toBe('x')
  })
})
