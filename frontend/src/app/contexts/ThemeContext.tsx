import {
  createContext,
  useContext,
  useState,
  useEffect,
  ReactNode,
} from "react";

type Theme = "light" | "dark";

interface ThemeContextType {
  theme: Theme;
  toggleTheme: () => void;
  setThemeFromAnimation: (isDark: boolean) => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

/*
 * Export semantic tokens for app-level imports.
 * Mirrors shared/contexts/ThemeContext to provide a single contract
 * for components that reference the app context path.
 */
export const DARK_MODE_TOKENS = {
  background: {
    surfacePrimary: "#1a1714",
    surfaceSecondary: "#2d2820",
    surfaceTertiary: "#3a3428",
    overlayHard: "#1a1714",
    overlayMedium: "rgba(26, 23, 20, 0.8)",
    overlaySoft: "rgba(26, 23, 20, 0.5)",
    glassStrong: "rgba(255, 255, 255, 0.12)",
    glassMedium: "rgba(255, 255, 255, 0.08)",
    glassLight: "rgba(255, 255, 255, 0.06)",
  },
  text: {
    primary: "#f5f5f5",
    secondary: "#d4d4d4",
    tertiary: "#b8a898",
    muted: "#9b8d7f",
    disabled: "#978e82",
  },
  border: {
    subtle: "rgba(255, 255, 255, 0.08)",
    default: "rgba(255, 255, 255, 0.10)",
    prominent: "rgba(255, 255, 255, 0.15)",
    interactive: "rgba(255, 255, 255, 0.20)",
  },
  interactive: {
    hover: "rgba(255,255,255,0.10)",
    active: "rgba(255,255,255,0.15)",
    focusRing: "#f1b400",
    disabled: "rgba(255,255,255,0.05)",
  },
  semantic: {
    accentPrimary: "#c9983a",
    accentHover: "#e8c77f",
    success: "#22c55e",
    warning: "#f59e0b",
    error: "#ef4444",
  },
} as const;

export const FOCUS_RING_SPEC = {
  light: "outline-2 outline-offset-2 focus:outline-[#a2792c]",
  dark: "outline-2 outline-offset-2 focus:outline-[#f1b400]",
  tailwind: (isDark: boolean) =>
    isDark
      ? "focus:outline-2 focus:outline-offset-2 focus:outline-[#f1b400]"
      : "focus:outline-2 focus:outline-offset-2 focus:outline-[#a2792c]",
} as const;

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>(() => {
    const savedTheme = localStorage.getItem("theme") as Theme;
    return savedTheme || "light";
  });

  useEffect(() => {
    localStorage.setItem("theme", theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme((prev) => (prev === "light" ? "dark" : "light"));
  };

  const setThemeFromAnimation = (isDark: boolean) => {
    setTheme(isDark ? "dark" : "light");
  };

  return (
    <ThemeContext.Provider
      value={{ theme, toggleTheme, setThemeFromAnimation }}
    >
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return context;
}
