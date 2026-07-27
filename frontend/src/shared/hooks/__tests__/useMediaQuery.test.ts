import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useMediaQuery } from '../useMediaQuery'

function mockMatchMedia(initialMatches: boolean, query: string) {
  const listeners = new Set<EventListener>()
  let matches = initialMatches
  return {
    get matches() { return matches },
    set matches(v: boolean) { matches = v },
    media: query,
    addEventListener: (_type: string, listener: EventListener) => {
      listeners.add(listener)
    },
    removeEventListener: (_type: string, listener: EventListener) => {
      listeners.delete(listener)
    },
    dispatchEvent: (_event: Event) => {
      listeners.forEach((l) => l({ matches } as unknown as MediaQueryListEvent))
    },
  }
}

beforeEach(() => {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation((query: string) => mockMatchMedia(false, query)),
  )
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('useMediaQuery', () => {
  it('returns false when the media query does not match', () => {
    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    expect(result.current).toBe(false)
  })

  it('returns true when the media query matches', () => {
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => mockMatchMedia(true, query)),
    )
    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    expect(result.current).toBe(true)
  })

  it('initializes with the correct value on first render (deterministic)', () => {
    const { result } = renderHook(() => useMediaQuery('(prefers-reduced-motion: reduce)'))
    expect(result.current).toBe(false)
  })

  it('updates when the media query status changes', () => {
    const mql = mockMatchMedia(false, '(max-width: 767px)')
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation(() => mql),
    )

    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    expect(result.current).toBe(false)

    act(() => {
      mql.matches = true
      mql.dispatchEvent(new Event('change'))
    })

    expect(result.current).toBe(true)
  })

  it('handles dynamic query changes via rerender', () => {
    const { result, rerender } = renderHook(
      (q: string) => useMediaQuery(q),
      { initialProps: '(max-width: 767px)' },
    )
    expect(result.current).toBe(false)

    rerender('(min-width: 1024px)')
    expect(result.current).toBe(false)
  })

  it('reads current matchMedia value synchronously (no async flash)', () => {
    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    expect(result.current).toBe(false)

    const { result: resultTrue } = renderHook(() => useMediaQuery('(min-width: 1024px)'))
    expect(resultTrue.current).toBe(false)
  })

  it('survives rapid change events and settles on the last emitted value', () => {
    const mql = mockMatchMedia(false, '(max-width: 767px)')
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation(() => mql),
    )

    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))

    const TOGGLE_COUNT = 20
    for (let i = 0; i < TOGGLE_COUNT; i++) {
      act(() => {
        mql.matches = i % 2 === 0
        mql.dispatchEvent(new Event('change'))
      })
    }

    const lastExpected = (TOGGLE_COUNT - 1) % 2 === 0
    expect(result.current).toBe(lastExpected)
  })

  it('handles bidirectional changes: match → no-match → match', () => {
    const mql = mockMatchMedia(false, '(max-width: 767px)')
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation(() => mql),
    )

    const { result } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    expect(result.current).toBe(false)

    act(() => {
      mql.matches = true
      mql.dispatchEvent(new Event('change'))
    })
    expect(result.current).toBe(true)

    act(() => {
      mql.matches = false
      mql.dispatchEvent(new Event('change'))
    })
    expect(result.current).toBe(false)

    act(() => {
      mql.matches = true
      mql.dispatchEvent(new Event('change'))
    })
    expect(result.current).toBe(true)
  })

  it('handles multiple independent instances with different queries', () => {
    const narrowMql = mockMatchMedia(true, '(max-width: 767px)')
    const wideMql = mockMatchMedia(false, '(min-width: 1024px)')
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation((query: string) => {
        if (query === '(max-width: 767px)') return narrowMql
        if (query === '(min-width: 1024px)') return wideMql
        return mockMatchMedia(false)
      }),
    )

    const { result: narrowResult } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    const { result: wideResult } = renderHook(() => useMediaQuery('(min-width: 1024px)'))

    expect(narrowResult.current).toBe(true)
    expect(wideResult.current).toBe(false)

    act(() => {
      narrowMql.matches = false
      narrowMql.dispatchEvent(new Event('change'))
      wideMql.matches = true
      wideMql.dispatchEvent(new Event('change'))
    })

    expect(narrowResult.current).toBe(false)
    expect(wideResult.current).toBe(true)
  })

  it('cleans up the change listener on unmount', () => {
    const removeSpy = vi.fn()
    const mql = {
      matches: false,
      media: '(max-width: 767px)',
      addEventListener: vi.fn(),
      removeEventListener: removeSpy,
    }
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockImplementation(() => mql),
    )

    const { unmount } = renderHook(() => useMediaQuery('(max-width: 767px)'))
    unmount()
    expect(removeSpy).toHaveBeenCalledTimes(1)
  })
})
