import { ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import {
  LayoutDashboard,
  Layers,
  Award,
  Settings,
  BookOpen,
  ChevronLeft,
  ChevronRight,
  Menu,
  X,
} from "lucide-react";

// ─── Types ────────────────────────────────────────────────────────────────────

type NavItem = {
  name: string;
  path?: string;
  icon: ReactNode;
  external?: boolean;
  disabled?: boolean;
  badge?: string;
};

type AppShellProps = {
  children: ReactNode;
  title?: string;
  secondaryNavItems?: Omit<NavItem, "icon">[];
  contextualActions?: ReactNode;
};

// ─── Focus trap hook ──────────────────────────────────────────────────────────

function useFocusTrap(active: boolean) {
  const containerRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!active || !containerRef.current) return;

    const container = containerRef.current;
    const focusable = container.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    // Move focus into the drawer
    first?.focus();

    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== "Tab") return;
      if (focusable.length === 0) { e.preventDefault(); return; }

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    }

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [active]);

  return containerRef;
}

// ─── Skip-nav link ────────────────────────────────────────────────────────────

function SkipNav() {
  return (
    <a
      href="#main-content"
      className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[100] focus:rounded-md focus:bg-gray-900 focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-white focus:shadow-lg"
    >
      Skip to main content
    </a>
  );
}

// ─── Nav item renderer ────────────────────────────────────────────────────────

type RenderNavItemOptions = {
  item: NavItem;
  isActive: boolean;
  collapsed?: boolean;
  onClose?: () => void;
};

function NavItemEl({ item, isActive, collapsed = false, onClose }: RenderNavItemOptions) {
  // Collapsed icon-only button (sidebar)
  if (collapsed) {
    const inner = (
      <span
        className={[
          "flex h-11 w-11 items-center justify-center rounded-md transition-colors",
          "focus:outline-none focus:ring-2 focus:ring-inset focus:ring-gray-400",
          item.disabled
            ? "cursor-not-allowed opacity-40"
            : isActive
              ? "bg-white text-gray-900"
              : "text-gray-300 hover:bg-gray-700 hover:text-white",
        ].join(" ")}
        aria-label={item.name}
      >
        {item.icon}
      </span>
    );

    if (item.disabled) {
      return (
        <div
          key={item.name}
          className="group relative flex justify-center"
          aria-disabled="true"
        >
          {inner}
          <Tooltip label={`${item.name}${item.badge ? ` (${item.badge})` : ""}`} />
        </div>
      );
    }

    if (item.external && item.path) {
      return (
        <div key={item.name} className="group relative flex justify-center">
          <a
            href={item.path}
            target="_blank"
            rel="noreferrer"
            className="flex h-11 w-11 items-center justify-center rounded-md text-gray-300 transition-colors hover:bg-gray-700 hover:text-white focus:outline-none focus:ring-2 focus:ring-inset focus:ring-gray-400"
            aria-label={`${item.name} (opens in new tab)`}
          >
            {item.icon}
          </a>
          <Tooltip label={item.name} />
        </div>
      );
    }

    return (
      <div key={item.name} className="group relative flex justify-center">
        <Link
          to={item.path || "#"}
          aria-current={isActive ? "page" : undefined}
          aria-label={item.name}
          className={[
            "flex h-11 w-11 items-center justify-center rounded-md transition-colors",
            "focus:outline-none focus:ring-2 focus:ring-inset focus:ring-gray-400",
            isActive
              ? "bg-white text-gray-900"
              : "text-gray-300 hover:bg-gray-700 hover:text-white",
          ].join(" ")}
        >
          {item.icon}
        </Link>
        <Tooltip label={item.name} />
      </div>
    );
  }

  // Expanded nav item (sidebar expanded or mobile drawer)
  const baseClass = [
    "flex min-h-[44px] w-full items-center gap-3 rounded-md px-4 py-2.5 text-sm font-medium transition-colors",
    "focus:outline-none focus:ring-2 focus:ring-inset focus:ring-gray-400",
    item.disabled
      ? "cursor-not-allowed opacity-40"
      : isActive
        ? "bg-white text-gray-900 shadow-sm"
        : "text-gray-300 hover:bg-gray-700 hover:text-white",
  ].join(" ");

  if (item.disabled) {
    return (
      <div key={item.name} className={baseClass} aria-disabled="true" role="link">
        <span className="shrink-0">{item.icon}</span>
        <span className="flex-1 truncate">{item.name}</span>
        {item.badge && (
          <span className="ml-auto shrink-0 rounded-full bg-gray-700 px-2 py-0.5 text-xs text-gray-300">
            {item.badge}
          </span>
        )}
      </div>
    );
  }

  if (item.external && item.path) {
    return (
      <a
        key={item.name}
        href={item.path}
        target="_blank"
        rel="noreferrer"
        className={baseClass}
      >
        <span className="shrink-0">{item.icon}</span>
        <span className="flex-1 truncate">{item.name}</span>
        <span className="ml-auto shrink-0 text-xs opacity-60" aria-hidden="true">↗</span>
      </a>
    );
  }

  return (
    <Link
      key={item.name}
      to={item.path || "#"}
      className={baseClass}
      aria-current={isActive ? "page" : undefined}
      onClick={onClose}
    >
      <span className="shrink-0">{item.icon}</span>
      <span className="flex-1 truncate">{item.name}</span>
      {item.badge && (
        <span className="ml-auto shrink-0 rounded-full bg-gray-700 px-2 py-0.5 text-xs text-gray-300">
          {item.badge}
        </span>
      )}
    </Link>
  );
}

// ─── Tooltip (for collapsed sidebar) ─────────────────────────────────────────

function Tooltip({ label }: { label: string }) {
  return (
    <span
      role="tooltip"
      className={[
        "pointer-events-none absolute left-full top-1/2 z-[60] ml-2 -translate-y-1/2",
        "whitespace-nowrap rounded-md bg-gray-800 px-2.5 py-1.5 text-xs font-medium text-white shadow-lg",
        "opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100",
      ].join(" ")}
    >
      {label}
    </span>
  );
}

// ─── AppShell ─────────────────────────────────────────────────────────────────

export default function AppShell({
  children,
  title = "Dashboard",
  secondaryNavItems = [],
  contextualActions,
}: AppShellProps) {
  const location = useLocation();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const drawerRef = useFocusTrap(drawerOpen) as React.RefObject<HTMLElement>;
  const hamburgerRef = useRef<HTMLButtonElement>(null);

  const primaryNavItems: NavItem[] = useMemo(
    () => [
      { name: "Dashboard", path: "/dashboard", icon: <LayoutDashboard size={20} aria-hidden="true" /> },
      { name: "Programs", path: "/programs", icon: <Layers size={20} aria-hidden="true" />, disabled: true, badge: "Soon" },
      { name: "Bounties", path: "/bounties", icon: <Award size={20} aria-hidden="true" />, disabled: true, badge: "Soon" },
      { name: "Settings", path: "/settings", icon: <Settings size={20} aria-hidden="true" />, disabled: true, badge: "Soon" },
      { name: "Docs", path: "https://docs.grainlify.com", icon: <BookOpen size={20} aria-hidden="true" />, external: true },
    ],
    [],
  );

  const isActiveRoute = useCallback(
    (path?: string) => {
      if (!path || path.startsWith("http")) return false;
      return location.pathname === path || location.pathname.startsWith(`${path}/`);
    },
    [location.pathname],
  );

  // Close drawer on Escape
  useEffect(() => {
    if (!drawerOpen) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") closeDrawer();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [drawerOpen]);

  // Prevent body scroll when drawer is open
  useEffect(() => {
    document.body.style.overflow = drawerOpen ? "hidden" : "";
    return () => { document.body.style.overflow = ""; };
  }, [drawerOpen]);

  function openDrawer() { setDrawerOpen(true); }
  function closeDrawer() {
    setDrawerOpen(false);
    // Return focus to hamburger button
    hamburgerRef.current?.focus();
  }

  const breadcrumbs = useMemo(() => {
    const segments = location.pathname.split("/").filter(Boolean);
    if (segments.length <= 1) return [];
    return segments.map((segment, index) => {
      const path = `/${segments.slice(0, index + 1).join("/")}`;
      const label = segment.charAt(0).toUpperCase() + segment.slice(1);
      return { label, path };
    });
  }, [location.pathname]);

  return (
    <>
      <SkipNav />

      <div className="flex min-h-screen bg-gray-100">
        {/* ── Desktop sidebar ─────────────────────────────────────────────── */}
        <aside
          className={[
            "hidden md:flex min-h-screen flex-col bg-gray-900 text-white transition-[width] duration-200 ease-in-out z-20",
            sidebarCollapsed ? "w-16" : "w-72",
          ].join(" ")}
          aria-label="Primary navigation"
        >
          {/* Workspace header */}
          <div
            className={[
              "flex items-center border-b border-gray-800 px-3 py-4",
              sidebarCollapsed ? "justify-center" : "gap-3",
            ].join(" ")}
          >
            {!sidebarCollapsed && (
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-bold text-white">Grainlify Workspace</div>
                <div className="truncate text-xs text-gray-400">Very Long Organization Name Example</div>
              </div>
            )}
            {/* Collapse toggle — 44×44px */}
            <button
              type="button"
              onClick={() => setSidebarCollapsed((v) => !v)}
              className="flex h-11 w-11 shrink-0 items-center justify-center rounded-md text-gray-400 transition-colors hover:bg-gray-700 hover:text-white focus:outline-none focus:ring-2 focus:ring-inset focus:ring-gray-400"
              aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
              aria-expanded={!sidebarCollapsed}
            >
              {sidebarCollapsed
                ? <ChevronRight size={18} aria-hidden="true" />
                : <ChevronLeft size={18} aria-hidden="true" />}
            </button>
          </div>

          {/* Nav items */}
          <nav
            className={["flex flex-1 flex-col gap-1 p-2", sidebarCollapsed ? "items-center" : ""].join(" ")}
            aria-label="Primary navigation"
          >
            {primaryNavItems.map((item) => (
              <NavItemEl
                key={item.name}
                item={item}
                isActive={isActiveRoute(item.path)}
                collapsed={sidebarCollapsed}
              />
            ))}
          </nav>

          {!sidebarCollapsed && (
            <div className="border-t border-gray-800 px-4 py-3 text-xs text-gray-500">
              Docs opens in a new tab
            </div>
          )}
        </aside>

        {/* ── Main content area ────────────────────────────────────────────── */}
        <div className="flex min-h-screen flex-1 flex-col overflow-hidden">

          {/* ── Mobile top bar ─────────────────────────────────────────────── */}
          <header className="sticky top-0 z-30 border-b bg-white md:hidden">
            <div className="flex h-14 items-center justify-between px-4">
              {/* Hamburger — 44×44px */}
              <button
                ref={hamburgerRef}
                type="button"
                onClick={openDrawer}
                className="flex h-11 w-11 items-center justify-center rounded-md text-gray-700 transition-colors hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-gray-400"
                aria-label="Open navigation menu"
                aria-expanded={drawerOpen}
                aria-controls="mobile-drawer"
                aria-haspopup="dialog"
              >
                <Menu size={22} aria-hidden="true" />
              </button>

              <span className="truncate text-base font-semibold text-gray-900">Grainlify</span>

              {/* Spacer to balance hamburger */}
              <div className="h-11 w-11" aria-hidden="true" />
            </div>
          </header>

          {/* ── Mobile drawer ───────────────────────────────────────────────── */}
          {/* Overlay */}
          <div
            className={[
              "fixed inset-0 z-40 bg-black/50 transition-opacity duration-200 md:hidden",
              drawerOpen ? "opacity-100" : "pointer-events-none opacity-0",
            ].join(" ")}
            aria-hidden="true"
            onClick={closeDrawer}
          />

          {/* Drawer panel */}
          <aside
            id="mobile-drawer"
            ref={drawerRef as React.RefObject<HTMLElement>}
            role="dialog"
            aria-modal="true"
            aria-label="Navigation menu"
            className={[
              "fixed left-0 top-0 z-50 flex h-full w-80 max-w-[85vw] flex-col bg-gray-900 text-white shadow-2xl",
              "transition-transform duration-300 ease-out md:hidden",
              drawerOpen ? "translate-x-0" : "-translate-x-full",
            ].join(" ")}
          >
            {/* Drawer header */}
            <div className="flex items-center justify-between border-b border-gray-800 px-4 py-3">
              <div className="min-w-0">
                <div className="font-bold text-white">Grainlify</div>
                <div className="max-w-[200px] truncate text-xs text-gray-400">
                  Very Long Organization Name Example
                </div>
              </div>
              {/* Close — 44×44px */}
              <button
                type="button"
                onClick={closeDrawer}
                className="flex h-11 w-11 shrink-0 items-center justify-center rounded-md text-gray-400 transition-colors hover:bg-gray-700 hover:text-white focus:outline-none focus:ring-2 focus:ring-inset focus:ring-gray-400"
                aria-label="Close navigation menu"
              >
                <X size={20} aria-hidden="true" />
              </button>
            </div>

            {/* Drawer nav */}
            <nav
              className="flex flex-1 flex-col gap-1 overflow-y-auto p-2"
              aria-label="Mobile primary navigation"
            >
              {primaryNavItems.map((item) => (
                <NavItemEl
                  key={item.name}
                  item={item}
                  isActive={isActiveRoute(item.path)}
                  onClose={closeDrawer}
                />
              ))}
            </nav>

            <div className="border-t border-gray-800 px-4 py-3 text-xs text-gray-500">
              Docs opens in a new tab
            </div>
          </aside>

          {/* ── Page header (breadcrumbs + title + actions) ──────────────── */}
          <div className="border-b bg-white px-4 py-4 md:px-6">
            {breadcrumbs.length > 0 && (
              <nav
                className="mb-2 flex flex-wrap items-center gap-1.5 text-sm text-gray-500"
                aria-label="Breadcrumb"
              >
                <Link to="/dashboard" className="hover:text-gray-700 focus:outline-none focus:underline">
                  Dashboard
                </Link>
                {breadcrumbs.map((crumb, index) => (
                  <span key={crumb.path} className="flex items-center gap-1.5">
                    <span aria-hidden="true">/</span>
                    {index === breadcrumbs.length - 1 ? (
                      <span className="font-medium text-gray-700" aria-current="page">
                        {crumb.label}
                      </span>
                    ) : (
                      <Link to={crumb.path} className="hover:text-gray-700 focus:outline-none focus:underline">
                        {crumb.label}
                      </Link>
                    )}
                  </span>
                ))}
              </nav>
            )}

            <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
              <h1 className="text-xl font-semibold text-gray-900">{title}</h1>
              {contextualActions && (
                <div className="flex flex-wrap items-center gap-2">{contextualActions}</div>
              )}
            </div>

            {secondaryNavItems.length > 0 && (
              <div
                className="mt-4 flex flex-wrap gap-2"
                role="navigation"
                aria-label="Secondary navigation"
              >
                {secondaryNavItems.map((item) => {
                  const active = isActiveRoute(item.path);

                  if (item.disabled) {
                    return (
                      <div
                        key={item.name}
                        className="flex min-h-[44px] items-center rounded-md border border-dashed px-3 py-2 text-sm text-gray-400"
                        aria-disabled="true"
                        role="link"
                      >
                        {item.name}
                      </div>
                    );
                  }

                  if (item.external && item.path) {
                    return (
                      <a
                        key={item.name}
                        href={item.path}
                        target="_blank"
                        rel="noreferrer"
                        className="flex min-h-[44px] items-center rounded-md border px-3 py-2 text-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-gray-400"
                      >
                        {item.name} ↗
                      </a>
                    );
                  }

                  return (
                    <Link
                      key={item.name}
                      to={item.path || "#"}
                      aria-current={active ? "page" : undefined}
                      className={[
                        "flex min-h-[44px] items-center rounded-md border px-3 py-2 text-sm transition-colors",
                        "focus:outline-none focus:ring-2 focus:ring-gray-400",
                        active
                          ? "border-gray-900 bg-gray-900 text-white"
                          : "hover:bg-gray-50",
                      ].join(" ")}
                    >
                      {item.name}
                    </Link>
                  );
                })}
              </div>
            )}
          </div>

          {/* ── Main content ─────────────────────────────────────────────── */}
          <main id="main-content" className="flex-1 p-4 md:p-6" tabIndex={-1}>
            {children}
          </main>
        </div>
      </div>
    </>
  );
}
