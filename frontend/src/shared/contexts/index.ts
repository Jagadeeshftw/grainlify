// Shared contexts barrel export
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
} from './ThemeContext';
export { AuthProvider, useAuth, type UserRole, type User } from './AuthContext';