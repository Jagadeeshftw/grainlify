import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { renderHook } from '@testing-library/react'
import React from 'react'
import {
  ThemeProvider,
  useTheme,
  isDarkVariant,
  isA11yVariant,
  themeRootClass,
  FOCUS_RING_SPEC,
  HIGH_CONTRAST_TOKENS,
  REDUCED_MOTION_TOKENS,
  DARK_MODE_TOKENS,
  type Theme,
} from './ThemeContext'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderWithProvider(initialTheme?: string) {
  if (initialTheme) {
    localStorage.setItem('theme', initialTheme)
  }
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ThemeProvider>{children}</ThemeProvider>
  )
  return renderHook(() => useTheme(), { wrapper })
}

const ALL_THEMES: Theme[] = ['light', 'dark', 'high-contrast', 'reduced-motion']

// ---------------------------------------------------------------------------
// localStorage mock
// ---------------------------------------------------------------------------

beforeEach(() => {
  localStorage.clear()
  // Reset html class list between tests
  document.documentElement.className = ''
})

afterEach(() => {
  localStorage.clear()
  document.documentElement.className = ''
  vi.restoreAllMocks()
})

// ---------------------------------------------------------------------------
// ThemeProvider — initialisation
// ---------------------------------------------------------------------------

describe('ThemeProvider — initialisation', () => {
  it('defaults to "light" when localStorage is empty', () => {
    const { result } = renderWithProvider()
    expect(result.current.theme).toBe('light')
  })

  it.each(ALL_THEMES)('restores "%s" from localStorage', (savedTheme) => {
    const { result } = renderWithProvider(savedTheme)
    expect(result.current.theme).toBe(savedTheme)
  })

  it('ignores an invalid localStorage value and falls back to "light"', () => {
    const { result } = renderWithProvider('invalid-theme')
    expect(result.current.theme).toBe('light')
  })

  it('ignores an empty string in localStorage and falls back to "light"', () => {
    const { result } = renderWithProvider('')
    expect(result.current.theme).toBe('light')
  })
})

// ---------------------------------------------------------------------------
// ThemeProvider — toggleTheme cycle
// ---------------------------------------------------------------------------

describe('ThemeProvider — toggleTheme', () => {
  it('cycles light → dark → high-contrast → reduced-motion → light', () => {
    const { result } = renderWithProvider()
    expect(result.current.theme).toBe('light')

    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('dark')

    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('high-contrast')

    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('reduced-motion')

    act(() => result.current.toggleTheme())
    expect(result.current.theme).toBe('light')
  })
})

// ---------------------------------------------------------------------------
// ThemeProvider — setTheme
// ---------------------------------------------------------------------------

describe('ThemeProvider — setTheme', () => {
  it.each(ALL_THEMES)('setTheme("%s") sets the theme', (t) => {
    const { result } = renderWithProvider()
    act(() => result.current.setTheme(t))
    expect(result.current.theme).toBe(t)
  })

  it('persists the new theme to localStorage', () => {
    const { result } = renderWithProvider()
    act(() => result.current.setTheme('high-contrast'))
    expect(localStorage.getItem('theme')).toBe('high-contrast')
  })
})

// ---------------------------------------------------------------------------
// ThemeProvider — setThemeFromAnimation (compat shim)
// ---------------------------------------------------------------------------

describe('ThemeProvider — setThemeFromAnimation', () => {
  it('setThemeFromAnimation(true) sets theme to "dark"', () => {
    const { result } = renderWithProvider()
    act(() => result.current.setThemeFromAnimation(true))
    expect(result.current.theme).toBe('dark')
  })

  it('setThemeFromAnimation(false) sets theme to "light"', () => {
    const { result } = renderWithProvider('dark')
    act(() => result.current.setThemeFromAnimation(false))
    expect(result.current.theme).toBe('light')
  })
})

// ---------------------------------------------------------------------------
// ThemeProvider — HTML root class management
// ---------------------------------------------------------------------------

describe('ThemeProvider — HTML root class', () => {
  it('applies no class for "light"', () => {
    renderWithProvider('light')
    expect(document.documentElement.classList.contains('dark')).toBe(false)
    expect(document.documentElement.classList.contains('high-contrast')).toBe(false)
    expect(document.documentElement.classList.contains('reduced-motion')).toBe(false)
  })

  it('applies "dark" class for "dark"', () => {
    renderWithProvider('dark')
    expect(document.documentElement.classList.contains('dark')).toBe(true)
  })

  it('applies "high-contrast" class for "high-contrast"', () => {
    renderWithProvider('high-contrast')
    expect(document.documentElement.classList.contains('high-contrast')).toBe(true)
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('applies "dark" and "reduced-motion" classes for "reduced-motion"', () => {
    renderWithProvider('reduced-motion')
    expect(document.documentElement.classList.contains('dark')).toBe(true)
    expect(document.documentElement.classList.contains('reduced-motion')).toBe(true)
  })

  it('removes previous class before applying new one', () => {
    const { result } = renderWithProvider('dark')
    expect(document.documentElement.classList.contains('dark')).toBe(true)

    act(() => result.current.setTheme('high-contrast'))
    expect(document.documentElement.classList.contains('dark')).toBe(false)
    expect(document.documentElement.classList.contains('high-contrast')).toBe(true)
  })

  it('removes all classes when switching to "light"', () => {
    const { result } = renderWithProvider('reduced-motion')
    act(() => result.current.setTheme('light'))
    expect(document.documentElement.className).toBe('')
  })
})

// ---------------------------------------------------------------------------
// useTheme — error boundary
// ---------------------------------------------------------------------------

describe('useTheme', () => {
  it('throws when used outside ThemeProvider', () => {
    // Suppress expected React error output
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(() => renderHook(() => useTheme())).toThrow(
      'useTheme must be used within a ThemeProvider'
    )
    spy.mockRestore()
  })

  it('exposes theme, toggleTheme, setTheme, setThemeFromAnimation', () => {
    const { result } = renderWithProvider()
    expect(result.current.theme).toBeDefined()
    expect(typeof result.current.toggleTheme).toBe('function')
    expect(typeof result.current.setTheme).toBe('function')
    expect(typeof result.current.setThemeFromAnimation).toBe('function')
  })
})

// ---------------------------------------------------------------------------
// isDarkVariant
// ---------------------------------------------------------------------------

describe('isDarkVariant', () => {
  it('returns true for "dark"', () => expect(isDarkVariant('dark')).toBe(true))
  it('returns true for "reduced-motion"', () => expect(isDarkVariant('reduced-motion')).toBe(true))
  it('returns false for "light"', () => expect(isDarkVariant('light')).toBe(false))
  it('returns false for "high-contrast"', () => expect(isDarkVariant('high-contrast')).toBe(false))
})

// ---------------------------------------------------------------------------
// isA11yVariant
// ---------------------------------------------------------------------------

describe('isA11yVariant', () => {
  it('returns true for "high-contrast"', () => expect(isA11yVariant('high-contrast')).toBe(true))
  it('returns true for "reduced-motion"', () => expect(isA11yVariant('reduced-motion')).toBe(true))
  it('returns false for "light"', () => expect(isA11yVariant('light')).toBe(false))
  it('returns false for "dark"', () => expect(isA11yVariant('dark')).toBe(false))
})

// ---------------------------------------------------------------------------
// themeRootClass
// ---------------------------------------------------------------------------

describe('themeRootClass', () => {
  it('returns "" for "light"', () => expect(themeRootClass('light')).toBe(''))
  it('returns "dark" for "dark"', () => expect(themeRootClass('dark')).toBe('dark'))
  it('returns "high-contrast" for "high-contrast"', () =>
    expect(themeRootClass('high-contrast')).toBe('high-contrast'))
  it('returns "dark reduced-motion" for "reduced-motion"', () =>
    expect(themeRootClass('reduced-motion')).toBe('dark reduced-motion'))
})

// ---------------------------------------------------------------------------
// FOCUS_RING_SPEC.className
// ---------------------------------------------------------------------------

describe('FOCUS_RING_SPEC.className', () => {
  it('returns 3px yellow ring for "high-contrast"', () => {
    const cls = FOCUS_RING_SPEC.className('high-contrast')
    expect(cls).toContain('#ffff00')
    expect(cls).toContain('[3px]')
  })

  it('returns gold 2px ring for "dark"', () => {
    const cls = FOCUS_RING_SPEC.className('dark')
    expect(cls).toContain('#f1b400')
    expect(cls).toContain('outline-2')
  })

  it('returns gold 2px ring for "reduced-motion"', () => {
    const cls = FOCUS_RING_SPEC.className('reduced-motion')
    expect(cls).toContain('#f1b400')
  })

  it('returns brown 2px ring for "light"', () => {
    const cls = FOCUS_RING_SPEC.className('light')
    expect(cls).toContain('#a2792c')
    expect(cls).toContain('outline-2')
  })
})

// ---------------------------------------------------------------------------
// Token shape guards — HIGH_CONTRAST_TOKENS
// ---------------------------------------------------------------------------

describe('HIGH_CONTRAST_TOKENS', () => {
  it('has background.surfacePrimary as pure black', () => {
    expect(HIGH_CONTRAST_TOKENS.background.surfacePrimary).toBe('#000000')
  })

  it('has text.primary as pure white', () => {
    expect(HIGH_CONTRAST_TOKENS.text.primary).toBe('#ffffff')
  })

  it('has focusRing as yellow #ffff00', () => {
    expect(HIGH_CONTRAST_TOKENS.interactive.focusRing).toBe('#ffff00')
  })

  it('has fully opaque border.default (no rgba)', () => {
    expect(HIGH_CONTRAST_TOKENS.border.default).not.toContain('rgba')
  })

  it('has all 5 semantic colour keys', () => {
    const keys = Object.keys(HIGH_CONTRAST_TOKENS.semantic)
    expect(keys).toEqual(
      expect.arrayContaining(['accentPrimary', 'accentHover', 'success', 'warning', 'error'])
    )
  })
})

// ---------------------------------------------------------------------------
// Token shape guards — REDUCED_MOTION_TOKENS
// ---------------------------------------------------------------------------

describe('REDUCED_MOTION_TOKENS', () => {
  it('inherits dark mode background tokens', () => {
    expect(REDUCED_MOTION_TOKENS.background.surfacePrimary).toBe(
      DARK_MODE_TOKENS.background.surfacePrimary
    )
  })

  it('has motion.transitionDuration of "0ms"', () => {
    expect(REDUCED_MOTION_TOKENS.motion.transitionDuration).toBe('0ms')
  })

  it('has motion.opacityFadeDuration of "150ms"', () => {
    expect(REDUCED_MOTION_TOKENS.motion.opacityFadeDuration).toBe('150ms')
  })

  it('has motion.skeletonShimmer as "static"', () => {
    expect(REDUCED_MOTION_TOKENS.motion.skeletonShimmer).toBe('static')
  })

  it('has motion.modalEntrance as "opacity-only"', () => {
    expect(REDUCED_MOTION_TOKENS.motion.modalEntrance).toBe('opacity-only')
  })

  it('has motion.pageTransition as "instant"', () => {
    expect(REDUCED_MOTION_TOKENS.motion.pageTransition).toBe('instant')
  })
})
