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

  it('resolves xl-specific token at large desktop breakpoint', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1280px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 1, md: 2, lg: 3, xl: 5 }
    const { result } = renderHook(() => useResponsiveToken(tokens, 1))
    expect(result.current).toBe(5)
  })

  it('falls back from xl to lg when xl token is not defined', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1280px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 1, md: 2, lg: 4 }
    const { result } = renderHook(() => useResponsiveToken(tokens, 1))
    expect(result.current).toBe(4)
  })

  it('falls back through the full chain xl → lg → md → sm → default', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1280px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 100 }
    const { result } = renderHook(() => useResponsiveToken(tokens, 999))
    expect(result.current).toBe(100)
  })

  it('returns defaultValue when token map is empty even at xl', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1280px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useResponsiveToken({}, 'fallback'))
    expect(result.current).toBe('fallback')
  })

  it('does NOT cascade up to larger breakpoints — only down', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { md: 'tablet-value', lg: 'desktop-value' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'fallback'))
    expect(result.current).toBe('fallback')
  })

  it('at mobile (sm) with no tokens defined anywhere, returns defaultValue', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const { result } = renderHook(() => useResponsiveToken({}, 'default-val'))
    expect(result.current).toBe('default-val')
  })

  it('cascades from lg down to sm when only sm token is defined', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 'only-mobile' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'fallback'))
    expect(result.current).toBe('only-mobile')
  })

  it('treats undefined values in token map as missing (skips them in cascade)', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens: Record<string, string | undefined> = { sm: 'mobile', md: undefined, lg: undefined }
    const { result } = renderHook(() => useResponsiveToken(tokens as Partial<Record<string, string>>, 'fallback'))
    expect(result.current).toBe('mobile')
  })

  it('handles numeric zero (0) as a valid defined value', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { sm: 0, md: 1 }
    const { result } = renderHook(() => useResponsiveToken(tokens, -1))
    expect(result.current).toBe(1)
  })

  it('handles false as a valid defined boolean value', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { lg: false }
    const { result } = renderHook(() => useResponsiveToken(tokens, true))
    expect(result.current).toBe(false)
  })

  it('at tablet (md), xl-only token map returns default (no cascade up)', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 768px) and (max-width: 1023px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { xl: 'large-desktop-only' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'fallback'))
    expect(result.current).toBe('fallback')
  })

  it('at large desktop (xl), cascades to lg when lg token exists and xl does not', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1280px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens = { lg: 'desktop-value' }
    const { result } = renderHook(() => useResponsiveToken(tokens, 'fallback'))
    expect(result.current).toBe('desktop-value')
  })

  it('at large desktop (xl), cascades past lg when lg is undefined', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(min-width: 1280px)') return mockMatchMedia(true)
        if (query === '(min-width: 1024px)') return mockMatchMedia(true)
        return mockMatchMedia(false)
      }),
    )
    const tokens: Record<string, string | undefined> = { lg: undefined, md: 'tablet-value' }
    const { result } = renderHook(() => useResponsiveToken(tokens as Partial<Record<string, string>>, 'fallback'))
    expect(result.current).toBe('tablet-value')
  })
})
