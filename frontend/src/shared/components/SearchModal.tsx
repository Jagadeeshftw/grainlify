/**
 * SearchModal — global search with full keyboard navigation contract.
 *
 * Shortcut contract (implemented here + documented in
 * design/specs/search-modal-keyboard-navigation.md):
 *
 *  ⌘K / Ctrl+K  — toggle modal (registered globally on window)
 *  /             — focus search input from anywhere inside the modal
 *  j / ArrowDown — move selection down
 *  k / ArrowUp   — move selection up
 *  Enter         — navigate to selected result
 *  Esc           — dismiss modal (or close cheat-sheet first)
 *  ?             — toggle shortcut cheat-sheet overlay
 *
 * Accessibility:
 *  - role="dialog" + aria-modal + aria-label on the container
 *  - role="listbox" + aria-activedescendant on the results list
 *  - role="option" + aria-selected on each result item
 *  - aria-live="polite" region announces result counts on query change
 *  - Focus is trapped inside the modal while open
 *  - Focus returns to the previously focused element on close
 */
import {
  useState,
  useEffect,
  useRef,
  useCallback,
  type KeyboardEvent,
} from "react";
import { Search, ArrowRight, X, Keyboard } from "lucide-react";
import { useTheme } from "../contexts/ThemeContext";

/* ─── types ────────────────────────────────────────────────────────────────── */

interface SearchResult {
  id: string;
  label: string;
  href: string;
}

export interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/* ─── static data (replace with real async search in production) ──────────── */

const SUGGESTIONS: SearchResult[] = [
  {
    id: "s0",
    label: "Terminal-based markdown editors worth checking out",
    href: "/search?q=terminal+markdown+editors",
  },
  {
    id: "s1",
    label: "Unity projects for procedural terrain generation",
    href: "/search?q=unity+procedural+terrain",
  },
  {
    id: "s2",
    label: "Find the best GraphQL clients for TypeScript",
    href: "/search?q=graphql+typescript+clients",
  },
  {
    id: "s3",
    label: "AI-powered tools for reviewing pull requests",
    href: "/search?q=ai+pr+review",
  },
];

/* ─── shortcut cheat-sheet data ────────────────────────────────────────────── */

const SHORTCUTS = [
  { keys: ["⌘K", "Ctrl+K"], action: "Toggle search modal" },
  { keys: ["/"], action: "Focus search input" },
  { keys: ["j", "↓"], action: "Next result" },
  { keys: ["k", "↑"], action: "Previous result" },
  { keys: ["Enter"], action: "Open selected result" },
  { keys: ["Esc"], action: "Close modal / overlay" },
  { keys: ["?"], action: "Show this help" },
] as const;

/* ─── component ─────────────────────────────────────────────────────────────── */

export function SearchModal({ isOpen, onClose }: SearchModalProps) {
  const { theme } = useTheme();
  const dark = theme === "dark";

  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(-1);
  const [showCheatSheet, setShowCheatSheet] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const modalRef = useRef<HTMLDivElement>(null);
  const previousFocusRef = useRef<Element | null>(null);

  /* filtered results — swap with real search hook as needed */
  const results: SearchResult[] =
    query.trim().length === 0
      ? SUGGESTIONS
      : SUGGESTIONS.filter((s) =>
          s.label.toLowerCase().includes(query.toLowerCase())
        );

  /* aria-live announcement text */
  const announcement =
    query.trim().length > 0
      ? `${results.length} result${results.length !== 1 ? "s" : ""} for '${query}'`
      : "";

  /* ── open/close lifecycle ──────────────────────────────────────────────── */

  useEffect(() => {
    if (isOpen) {
      previousFocusRef.current = document.activeElement;
      document.body.style.overflow = "hidden";
      /* defer focus so the modal is rendered first */
      setTimeout(() => inputRef.current?.focus(), 0);
    } else {
      document.body.style.overflow = "";
      setQuery("");
      setActiveIndex(-1);
      setShowCheatSheet(false);
      /* return focus to the element that had it before opening */
      if (previousFocusRef.current instanceof HTMLElement) {
        previousFocusRef.current.focus();
      }
    }
  }, [isOpen]);

  /* ── global ⌘K / Ctrl+K + / shortcut ───────────────────────────────────── */

  useEffect(() => {
    const onWindowKey = (e: globalThis.KeyboardEvent) => {
      /* ⌘K or Ctrl+K — toggle modal */
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        if (isOpen) {
          onClose();
        }
        /* The parent is responsible for opening; expose the toggle via onClose
           when already open. Opening is triggered by the parent listening to
           the same shortcut. */
        return;
      }

      /* / — if modal is closed and focus is not in a text field, open modal */
      if (
        e.key === "/" &&
        !isOpen &&
        document.activeElement?.tagName !== "INPUT" &&
        document.activeElement?.tagName !== "TEXTAREA"
      ) {
        e.preventDefault();
        /* Signal parent to open. We dispatch a custom event the parent can
           listen to, keeping this component decoupled. */
        window.dispatchEvent(new CustomEvent("grainlify:open-search"));
      }
    };

    window.addEventListener("keydown", onWindowKey);
    return () => window.removeEventListener("keydown", onWindowKey);
  }, [isOpen, onClose]);

  /* ── focus trap ──────────────────────────────────────────────────────────── */

  const handleModalKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      if (!isOpen) return;

      switch (e.key) {
        case "Escape":
          e.preventDefault();
          e.stopPropagation();
          if (showCheatSheet) {
            setShowCheatSheet(false);
          } else {
            onClose();
          }
          break;

        case "?":
          /* only toggle cheat-sheet when input is NOT focused to avoid
             intercepting typed '?' in the search box */
          if (document.activeElement !== inputRef.current) {
            e.preventDefault();
            setShowCheatSheet((prev) => !prev);
          }
          break;

        case "/":
          e.preventDefault();
          inputRef.current?.focus();
          break;

        case "j":
        case "ArrowDown":
          if (document.activeElement !== inputRef.current || e.key === "ArrowDown") {
            e.preventDefault();
            setActiveIndex((i) => Math.min(i + 1, results.length - 1));
          }
          break;

        case "k":
        case "ArrowUp":
          if (document.activeElement !== inputRef.current || e.key === "ArrowUp") {
            e.preventDefault();
            setActiveIndex((i) => Math.max(i - 1, -1));
          }
          break;

        case "Enter":
          if (activeIndex >= 0 && results[activeIndex]) {
            e.preventDefault();
            window.location.href = results[activeIndex].href;
            onClose();
          }
          break;

        case "Tab": {
          /* standard focus-trap: keep focus inside modal */
          const focusable = modalRef.current?.querySelectorAll<HTMLElement>(
            'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
          );
          if (!focusable || focusable.length === 0) break;
          const first = focusable[0];
          const last = focusable[focusable.length - 1];
          if (e.shiftKey) {
            if (document.activeElement === first) {
              e.preventDefault();
              last.focus();
            }
          } else {
            if (document.activeElement === last) {
              e.preventDefault();
              first.focus();
            }
          }
          break;
        }

        default:
          break;
      }
    },
    [isOpen, showCheatSheet, onClose, activeIndex, results]
  );

  /* ── reset selection when query changes ──────────────────────────────────── */

  useEffect(() => {
    setActiveIndex(-1);
  }, [query]);

  if (!isOpen) return null;

  /* ── shared style helpers ─────────────────────────────────────────────────── */

  const surface = dark
    ? "bg-[#2d2820] border-white/15"
    : "bg-white/95 border-white/30";

  const text = dark ? "text-[#f5efe5]" : "text-[#2d2820]";
  const muted = dark ? "text-[#b8a898]/80" : "text-[#6b5d4d]/80";
  const outline = dark ? "focus:outline-[#f1b400]" : "focus:outline-[#a2792c]";
  const iconColor = dark ? "text-white/50" : "text-black/50";
  const pillBase = dark
    ? "bg-[#2d2820]/40 border-white/5 hover:bg-[#2d2820]/60 hover:border-white/10"
    : "bg-white/40 border-black/5 hover:bg-white/60 hover:border-black/10";

  /* ── render ──────────────────────────────────────────────────────────────── */

  return (
    <div
      className="fixed inset-0 z-[200] flex items-start justify-center pt-[10vh]"
      role="presentation"
    >
      {/* ── hidden ARIA live region ────────────────────────────────────────── */}
      <div
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
        data-testid="search-live-region"
      >
        {announcement}
      </div>

      {/* ── backdrop ──────────────────────────────────────────────────────── */}
      <div
        className={`absolute inset-0 transition-colors ${dark ? "bg-black/70" : "bg-black/40"}`}
        onClick={onClose}
        style={{ backdropFilter: "blur(12px)" }}
        aria-hidden="true"
      />

      {/* ── dialog ────────────────────────────────────────────────────────── */}
      <div
        ref={modalRef}
        role="dialog"
        aria-modal="true"
        aria-label="Global search"
        className={`relative w-full max-w-[900px] mx-4 rounded-[32px] border shadow-2xl ${surface}`}
        style={{ backdropFilter: "blur(90px)" }}
        onKeyDown={handleModalKeyDown}
        data-testid="search-modal"
      >
        {/* close button */}
        <button
          onClick={onClose}
          aria-label="Close search"
          className={`absolute top-6 right-6 w-10 h-10 rounded-full flex items-center justify-center transition-all hover:scale-105 focus:outline-2 focus:outline-offset-2 ${dark ? "bg-white/5 hover:bg-white/10 text-white/60 hover:text-white/80 focus:outline-[#f1b400]" : "bg-black/5 hover:bg-black/10 text-black/60 hover:text-black/80 focus:outline-[#a2792c]"}`}
          data-testid="close-button"
        >
          <X className="w-5 h-5" aria-hidden="true" />
        </button>

        {/* cheat-sheet button */}
        <button
          onClick={() => setShowCheatSheet((p) => !p)}
          aria-label="Keyboard shortcuts"
          aria-expanded={showCheatSheet}
          aria-controls="cheat-sheet"
          className={`absolute top-6 right-20 w-10 h-10 rounded-full flex items-center justify-center transition-all hover:scale-105 focus:outline-2 focus:outline-offset-2 ${dark ? "bg-white/5 hover:bg-white/10 text-white/60 hover:text-white/80 focus:outline-[#f1b400]" : "bg-black/5 hover:bg-black/10 text-black/60 hover:text-black/80 focus:outline-[#a2792c]"}`}
          data-testid="cheat-sheet-button"
        >
          <Keyboard className="w-5 h-5" aria-hidden="true" />
        </button>

        <div className="p-12">
          {/* heading */}
          <h1 className={`text-[42px] font-bold text-center mb-4 leading-tight ${text}`}>
            Search Open Source Projects and
            <br />
            Build Your Confidence
          </h1>

          <p className={`text-center text-[15px] mb-8 ${muted}`}>
            Build your open source portfolio to optimize your chances of getting
            funded.
            <br />
            Explore projects that help you stand out.
          </p>

          {/* search input */}
          <div
            className={`relative h-[64px] rounded-[32px] mb-12 ${dark ? "bg-[#3a3428]/80 border border-white/20" : "bg-white/60 border border-black/10"}`}
            style={{ backdropFilter: "blur(40px)" }}
          >
            <div className="absolute inset-0 flex items-center px-6">
              <Search
                className={`w-5 h-5 mr-4 flex-shrink-0 ${iconColor}`}
                aria-hidden="true"
              />
              <input
                ref={inputRef}
                type="search"
                role="combobox"
                aria-expanded={results.length > 0}
                aria-autocomplete="list"
                aria-controls="search-listbox"
                aria-activedescendant={
                  activeIndex >= 0 ? results[activeIndex]?.id : undefined
                }
                aria-label="Search query"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Search projects, issues, contributors…"
                autoComplete="off"
                spellCheck={false}
                className={`flex-1 bg-transparent text-[16px] focus:outline-2 focus:outline-offset-2 ${dark ? "text-white placeholder:text-white/40 focus:outline-[#f1b400]" : "text-[#2d2820] placeholder:text-black/40 focus:outline-[#a2792c]"}`}
                data-testid="search-input"
              />
              <button
                aria-label="Submit search"
                onClick={() => {
                  if (activeIndex >= 0 && results[activeIndex]) {
                    window.location.href = results[activeIndex].href;
                    onClose();
                  }
                }}
                className={`w-10 h-10 rounded-full flex items-center justify-center ml-4 flex-shrink-0 transition-all hover:scale-105 focus:outline-2 focus:outline-offset-2 bg-[#c9983a] hover:bg-[#d4a645] ${outline}`}
              >
                <ArrowRight className="w-5 h-5 text-white" aria-hidden="true" />
              </button>
            </div>
          </div>

          {/* results / suggestions */}
          <section aria-label="Search suggestions">
            <h2 className={`text-[13px] font-semibold uppercase tracking-wider mb-1 ${text}`}>
              {query.trim() ? "Results" : "Search suggestions"}
            </h2>
            <p className={`text-[13px] mb-4 ${muted}`}>
              {query.trim()
                ? `${results.length} result${results.length !== 1 ? "s" : ""} for "${query}"`
                : "Discover interesting projects across different technologies"}
            </p>

            <ul
              id="search-listbox"
              role="listbox"
              aria-label="Search results"
              className="grid grid-cols-1 md:grid-cols-2 gap-3"
              data-testid="search-results"
            >
              {results.length === 0 ? (
                <li
                  role="option"
                  aria-selected="false"
                  className={`px-5 py-4 rounded-[16px] ${muted}`}
                >
                  No results found
                </li>
              ) : (
                results.map((result, idx) => (
                  <li
                    key={result.id}
                    id={result.id}
                    role="option"
                    aria-selected={idx === activeIndex}
                    className={`group flex items-center justify-between px-5 py-4 rounded-[16px] transition-all cursor-pointer border focus-within:ring-2 ${pillBase} ${
                      idx === activeIndex
                        ? dark
                          ? "ring-2 ring-[#f1b400] bg-[#2d2820]/80"
                          : "ring-2 ring-[#a2792c] bg-white/80"
                        : ""
                    }`}
                    style={{ backdropFilter: "blur(20px)" }}
                    onClick={() => {
                      window.location.href = result.href;
                      onClose();
                    }}
                    onMouseEnter={() => setActiveIndex(idx)}
                    data-testid={`result-item-${idx}`}
                  >
                    <span className={`text-left text-[14px] ${dark ? "text-[#d4c5b0]" : "text-[#6b5d4d]"}`}>
                      {result.label}
                    </span>
                    <ArrowRight
                      className={`w-4 h-4 ml-3 flex-shrink-0 transition-all group-hover:translate-x-1 ${dark ? "text-[#c9983a]" : "text-[#a2792c]"}`}
                      aria-hidden="true"
                    />
                  </li>
                ))
              )}
            </ul>
          </section>
        </div>

        {/* ── cheat-sheet overlay ───────────────────────────────────────────── */}
        {showCheatSheet && (
          <div
            id="cheat-sheet"
            role="dialog"
            aria-label="Keyboard shortcuts"
            aria-modal="false"
            className={`absolute inset-0 rounded-[32px] flex flex-col items-center justify-center p-10 z-10 ${dark ? "bg-[#2d2820]/96" : "bg-white/96"}`}
            style={{ backdropFilter: "blur(40px)" }}
            data-testid="cheat-sheet"
          >
            <h2 className={`text-2xl font-bold mb-6 ${text}`}>
              Keyboard Shortcuts
            </h2>

            <table className="w-full max-w-sm text-[14px]" role="table">
              <thead className="sr-only">
                <tr>
                  <th>Keys</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {SHORTCUTS.map(({ keys, action }) => (
                  <tr key={action} className="border-b border-white/5 last:border-0">
                    <td className="py-3 pr-6 w-40 whitespace-nowrap">
                      {keys.map((k) => (
                        <kbd
                          key={k}
                          className={`inline-block px-2 py-0.5 mr-1 rounded text-[12px] font-mono border ${dark ? "bg-white/10 border-white/20 text-white/80" : "bg-black/5 border-black/15 text-black/70"}`}
                        >
                          {k}
                        </kbd>
                      ))}
                    </td>
                    <td className={`py-3 ${muted}`}>{action}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            <button
              onClick={() => setShowCheatSheet(false)}
              autoFocus
              aria-label="Close keyboard shortcuts"
              className={`mt-8 px-6 py-2 rounded-full text-[14px] font-medium transition-all hover:scale-105 focus:outline-2 focus:outline-offset-2 bg-[#c9983a] hover:bg-[#d4a645] text-white ${outline}`}
              data-testid="close-cheat-sheet"
            >
              Close (Esc)
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
