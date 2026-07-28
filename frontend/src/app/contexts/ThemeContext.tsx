/**
 * App-level ThemeContext re-export.
 *
 * This file is a thin re-export of the canonical implementation at
 * `shared/contexts/ThemeContext`. Components should prefer importing from
 * `shared/contexts/ThemeContext` or `shared/contexts` directly, but this
 * alias is kept for backward compatibility with existing imports at the
 * app context path.
 *
 * @see frontend/src/shared/contexts/ThemeContext.tsx
 */
export {
  ThemeProvider,
  useTheme,
  isDarkVariant,
  isA11yVariant,
  themeRootClass,
  DARK_MODE_TOKENS,
  HIGH_CONTRAST_TOKENS,
  REDUCED_MOTION_TOKENS,
  FOCUS_RING_SPEC,
  type Theme,
} from "../../shared/contexts/ThemeContext";
