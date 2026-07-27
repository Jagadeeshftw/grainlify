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
})
