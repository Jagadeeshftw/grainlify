import { useEffect, useMemo, useState } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import {
  Search,
  Compass,
  Grid3x3,
  Calendar,
  Globe,
  Users,
  Trophy,
  Database,
  FileText,
  ChevronRight,
  Moon,
  Sun,
  Shield,
  X,
  Menu,
  ArrowUpRight,
} from "lucide-react";
import { useModeAnimation } from "react-theme-switch-animation";
import { useAuth } from "../../shared/contexts/AuthContext";
import grainlifyLogo from "../../assets/grainlify_log.svg";
import { useTheme } from "../../shared/contexts/ThemeContext";
import { UserProfileDropdown } from "../../shared/components/UserProfileDropdown";
import { NotificationsDropdown } from "../../shared/components/NotificationsDropdown";
import { RoleSwitcher } from "../../shared/components/RoleSwitcher";
import {
  Modal,
  ModalFooter,
  ModalButton,
  ModalInput,
} from "../../shared/components/ui/Modal";
import { bootstrapAdmin } from "../../shared/api/client";
import { ContributorsPage } from "./pages/ContributorsPage";
import { BrowsePage } from "./pages/BrowsePage";
import { DiscoverPage } from "./pages/DiscoverPage";
import { OpenSourceWeekPage } from "./pages/OpenSourceWeekPage";
import { OpenSourceWeekDetailPage } from "./pages/OpenSourceWeekDetailPage";
import { EcosystemsPage } from "./pages/EcosystemsPage";
import { EcosystemDetailPage } from "./pages/EcosystemDetailPage";
import { MaintainersPage } from "../maintainers/pages/MaintainersPage";
import { ProfilePage } from "./pages/ProfilePage";
import { DataPage } from "./pages/DataPage";
import { ProjectDetailPage } from "./pages/ProjectDetailPage";
import { IssueDetailPage } from "./pages/IssueDetailPage";
import { LeaderboardPage } from "../leaderboard/pages/LeaderboardPage";
import { BlogPage } from "../blog/pages/BlogPage";
import { SettingsPage } from "../settings/pages/SettingsPage";
import { AdminPage } from "../admin/pages/AdminPage";
import { SearchPage } from "./pages/SearchPage";
import { NotificationsPage } from "../notifications/NotificationsPage";
import { SettingsTabType } from "../settings/types";

type RoleType = "contributor" | "maintainer" | "admin";

type NavItem = {
  id: string;
  label: string;
  icon: any;
  disabled?: boolean;
  badge?: string;
};

export function Dashboard() {
  const { logout, login } = useAuth();
  const { theme, setThemeFromAnimation } = useTheme();
  const location = useLocation();
  const navigate = useNavigate();

  const { ref: themeToggleRef, toggleSwitchTheme } = useModeAnimation({
    isDarkMode: theme === "dark",
    onDarkModeChange: (isDark) => setThemeFromAnimation(isDark),
  });

  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(() => {
    if (typeof window === "undefined") return null;
    const params = new URLSearchParams(window.location.search);
    return params.get("project");
  });

  const [projectBackTarget, setProjectBackTarget] = useState<string | null>(() => {
    if (typeof window === "undefined") return null;
    const params = new URLSearchParams(window.location.search);
    return params.get("from");
  });

  const [selectedIssue, setSelectedIssue] = useState<{
    issueId: string;
    projectId?: string;
  } | null>(null);

  const [selectedEcosystemId, setSelectedEcosystemId] = useState<string | null>(null);
  const [selectedEcosystemName, setSelectedEcosystemName] = useState<string | null>(null);
  const [selectedEcosystemDescription, setSelectedEcosystemDescription] = useState<string | null>(null);
  const [selectedEcosystemLogoUrl, setSelectedEcosystemLogoUrl] = useState<string | null>(null);

  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);
  const [selectedEventName, setSelectedEventName] = useState<string | null>(null);

  const [deviceWidth, setDeviceWidth] = useState<number>(
    typeof window !== "undefined" ? window.innerWidth : 1280,
  );

  const isDesktop = deviceWidth >= 1024;

  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    return window.innerWidth < 1280;
  });

  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const [activeRole, setActiveRole] = useState<RoleType>("contributor");
  const [viewingUserId, setViewingUserId] = useState<string | null>(() => {
    if (typeof window === "undefined") return null;
    const params = new URLSearchParams(window.location.search);
    const userParam = params.get("user");
    const tabParam = params.get("tab") || params.get("page");
    if (tabParam === "profile" && userParam) {
      const uuidRegex =
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      return uuidRegex.test(userParam) ? userParam : null;
    }
    return null;
  });

  const [viewingUserLogin, setViewingUserLogin] = useState<string | null>(() => {
    if (typeof window === "undefined") return null;
    const params = new URLSearchParams(window.location.search);
    const userParam = params.get("user");
    const tabParam = params.get("tab") || params.get("page");
    if (tabParam === "profile" && userParam) {
      const uuidRegex =
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      return uuidRegex.test(userParam) ? null : userParam;
    }
    return null;
  });

  const [settingsInitialTab, setSettingsInitialTab] =
    useState<SettingsTabType>("profile");

  const [currentPage, setCurrentPage] = useState(() => {
    if (typeof window === "undefined") return "discover";
    const params = new URLSearchParams(window.location.search);
    const tabFromUrl = params.get("tab");
    if (tabFromUrl) return tabFromUrl;
    return localStorage.getItem("dashboardTab") || "discover";
  });

  const [showAdminPasswordModal, setShowAdminPasswordModal] = useState(false);
  const [adminPassword, setAdminPassword] = useState("");
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [adminAuthenticated, setAdminAuthenticated] = useState(() => {
    if (typeof window === "undefined") return false;
    return sessionStorage.getItem("admin_authenticated") === "true";
  });

  const darkTheme = theme === "dark";

  useEffect(() => {
    const handleResize = () => {
      setDeviceWidth(window.innerWidth);

      if (window.innerWidth < 1024) {
        setIsSidebarCollapsed(false);
      } else if (window.innerWidth < 1280) {
        setIsSidebarCollapsed(true);
      }
    };

    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const userParam = params.get("user");
    const tabParam = params.get("tab") || params.get("page");

    if (tabParam === "profile" && userParam) {
      setCurrentPage("profile");

      const uuidRegex =
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

      if (uuidRegex.test(userParam)) {
        setViewingUserId(userParam);
        setViewingUserLogin(null);
      } else {
        setViewingUserLogin(userParam);
        setViewingUserId(null);
      }
    } else if (tabParam === "profile" && !userParam) {
      setViewingUserId(null);
      setViewingUserLogin(null);
    }
  }, [location.search]);

  useEffect(() => {
    if (typeof window === "undefined") return;

    const params = new URLSearchParams(window.location.search);
    const projectParam = params.get("project");
    const issueParam = params.get("issue");
    const tabParam = params.get("tab") || params.get("page");

    if (tabParam === "browse" && projectParam && issueParam) {
      setCurrentPage("browse");
      setSelectedProjectId(projectParam);
      setSelectedIssue({ issueId: issueParam, projectId: projectParam });
    }
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") return;

    const params = new URLSearchParams(window.location.search);
    params.set("tab", currentPage);

    if (currentPage === "profile" && (viewingUserId || viewingUserLogin)) {
      params.set("user", viewingUserId || viewingUserLogin || "");
    } else if (currentPage === "profile") {
      params.delete("user");
    }

    if (selectedProjectId) {
      params.set("project", selectedProjectId);
      if (projectBackTarget) {
        params.set("from", projectBackTarget);
      }
    } else {
      params.delete("project");
      params.delete("from");
    }

    if (selectedIssue?.issueId && selectedIssue?.projectId) {
      params.set("issue", selectedIssue.issueId);
    } else if (!params.get("issue")) {
      params.delete("issue");
    }

    const newUrl = `${window.location.pathname}?${params.toString()}`;
    window.history.replaceState({}, "", newUrl);
    localStorage.setItem("dashboardTab", currentPage);
  }, [
    currentPage,
    selectedProjectId,
    selectedIssue,
    viewingUserId,
    viewingUserLogin,
    projectBackTarget,
  ]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setCurrentPage("search");
        setMobileMenuOpen(false);
      }

      if (e.key === "Escape") {
        setMobileMenuOpen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (typeof document === "undefined") return;
    document.body.style.overflow = mobileMenuOpen && !isDesktop ? "hidden" : "";
    return () => {
      document.body.style.overflow = "";
    };
  }, [mobileMenuOpen, isDesktop]);

  const closeMobileNav = () => {
    setMobileMenuOpen(false);
  };

  const handleNavigation = (page: string) => {
    setCurrentPage(page);
    setSelectedProjectId(null);
    setProjectBackTarget(null);
    setSelectedIssue(null);
    setSelectedEcosystemId(null);
    setSelectedEcosystemName(null);
    setSelectedEcosystemDescription(null);
    setSelectedEcosystemLogoUrl(null);
    setSelectedEventId(null);
    setSelectedEventName(null);

    if (page === "profile") {
      setViewingUserId(null);
      setViewingUserLogin(null);
    }

    closeMobileNav();
  };

  const handleLogout = () => {
    logout();
    setAdminAuthenticated(false);
    sessionStorage.removeItem("admin_authenticated");
    navigate("/");
  };

  const handleAdminClick = () => {
    if (adminAuthenticated) {
      setActiveRole("admin");
      handleNavigation("admin");
      return;
    }
    setShowAdminPasswordModal(true);
  };

  const handleAdminPasswordSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!adminPassword.trim()) return;

    setIsAuthenticating(true);

    try {
      const response = await bootstrapAdmin(adminPassword.trim());
      await login(response.token);
      setAdminAuthenticated(true);
      sessionStorage.setItem("admin_authenticated", "true");
      setShowAdminPasswordModal(false);
      setAdminPassword("");
      setActiveRole("admin");
      handleNavigation("admin");
    } catch (error) {
      console.error("Admin authentication failed:", error);
      setAdminPassword("");
    } finally {
      setIsAuthenticating(false);
    }
  };

  const handleRoleChange = (role: RoleType) => {
    if (role === "admin") {
      if (adminAuthenticated) {
        setActiveRole("admin");
        handleNavigation("admin");
      } else {
        setShowAdminPasswordModal(true);
      }
      return;
    }

    setActiveRole(role);
    setSelectedProjectId(null);
    setSelectedIssue(null);
    setSelectedEcosystemId(null);
    setSelectedEcosystemName(null);
    setSelectedEcosystemDescription(null);
    setSelectedEcosystemLogoUrl(null);
    setSelectedEventId(null);
    setSelectedEventName(null);

    if (role === "maintainer") {
      setCurrentPage("maintainers");
    } else {
      setCurrentPage("discover");
    }

    closeMobileNav();
  };

  const handleEcosystemClick = (
    ecosystemId: string,
    ecosystemName: string,
    description?: string | null,
    logoUrl?: string | null,
  ) => {
    setSelectedEcosystemId(ecosystemId);
    setSelectedEcosystemName(ecosystemName);
    setSelectedEcosystemDescription(description ?? null);
    setSelectedEcosystemLogoUrl(logoUrl ?? null);
  };

  const handleBackFromEcosystem = () => {
    setSelectedEcosystemId(null);
    setSelectedEcosystemName(null);
    setSelectedEcosystemDescription(null);
    setSelectedEcosystemLogoUrl(null);
  };

  const primaryNavItems: NavItem[] = useMemo(
    () => [
      { id: "discover", icon: Compass, label: "Discover" },
      { id: "browse", icon: Grid3x3, label: "Browse" },
      { id: "osw", icon: Calendar, label: "Open Source Week" },
      { id: "ecosystems", icon: Globe, label: "Ecosystems" },
      activeRole === "maintainer" || activeRole === "admin"
        ? { id: "maintainers", icon: Users, label: "Maintainers" }
        : { id: "contributors", icon: Users, label: "Contributors" },
      { id: "leaderboard", icon: Trophy, label: "Leaderboard" },
    ],
    [activeRole],
  );

  const secondaryNavItems: NavItem[] = useMemo(() => {
    const items: NavItem[] = [
      { id: "blog", icon: FileText, label: "Blog" },
      { id: "settings", icon: Shield, label: "Settings" },
    ];

    if (activeRole === "admin") {
      items.unshift({ id: "data", icon: Database, label: "Data" });
    }

    return items;
  }, [activeRole]);

  const currentPageLabelMap: Record<string, string> = {
    discover: "Discover",
    browse: "Browse",
    osw: "Open Source Week",
    ecosystems: "Ecosystems",
    contributors: "Contributors",
    maintainers: "Maintainers",
    leaderboard: "Leaderboard",
    blog: "Blog",
    settings: "Settings",
    search: "Search",
    profile: "Profile",
    data: "Data",
    admin: "Admin",
  };

  const breadcrumbs = useMemo(() => {
    const items = [{ label: "Dashboard", key: "dashboard" }];

    if (selectedIssue) {
      items.push({ label: currentPageLabelMap[currentPage] || "Browse", key: "section" });
      items.push({ label: "Project", key: "project" });
      items.push({ label: "Issue", key: "issue" });
      return items;
    }

    if (selectedProjectId) {
      items.push({ label: currentPageLabelMap[currentPage] || "Section", key: "section" });
      items.push({ label: "Project", key: "project" });
      return items;
    }

    if (currentPage === "ecosystems" && selectedEcosystemId) {
      items.push({ label: "Ecosystems", key: "ecosystems" });
      items.push({ label: selectedEcosystemName || "Detail", key: "detail" });
      return items;
    }

    if (currentPage === "osw" && selectedEventId) {
      items.push({ label: "Open Source Week", key: "osw" });
      items.push({ label: selectedEventName || "Event", key: "event" });
      return items;
    }

    items.push({ label: currentPageLabelMap[currentPage] || "Discover", key: "current" });
    return items;
  }, [
    currentPage,
    selectedIssue,
    selectedProjectId,
    selectedEcosystemId,
    selectedEcosystemName,
    selectedEventId,
    selectedEventName,
  ]);

  const renderNavButton = (
    item: NavItem,
    opts?: { mobile?: boolean; collapsed?: boolean },
  ) => {
    const isActive = currentPage === item.id;
    const collapsed = !!opts?.collapsed;
    const mobile = !!opts?.mobile;
    const Icon = item.icon;

    return (
      <button
        key={item.id}
        type="button"
        onClick={() => !item.disabled && handleNavigation(item.id)}
        aria-current={isActive ? "page" : undefined}
        aria-disabled={item.disabled ? "true" : undefined}
        disabled={item.disabled}
        className={`group w-full flex items-center rounded-[14px] transition-all duration-300 border ${
          collapsed
            ? "justify-center h-[52px] px-0"
            : "justify-between px-4 py-3"
        } ${
          isActive
            ? "bg-[#c9983a] border-[rgba(245,239,235,0.2)] shadow-[inset_0px_0px_4px_rgba(255,255,255,0.25)]"
            : darkTheme
              ? "bg-transparent border-transparent hover:bg-white/[0.08] hover:border-white/10"
              : "bg-transparent border-transparent hover:bg-white/[0.10] hover:border-white/20"
        } ${item.disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}`}
        title={collapsed ? item.label : undefined}
      >
        <div className={`flex items-center ${collapsed ? "" : "min-w-0"}`}>
          <Icon
            className={`w-5 h-5 flex-shrink-0 ${
              isActive
                ? "text-white"
                : darkTheme
                  ? "text-[#e8c77f]"
                  : "text-[#a2792c]"
            }`}
          />
          {!collapsed && (
            <span
              className={`ml-3 text-sm font-medium truncate ${
                isActive
                  ? "text-white"
                  : darkTheme
                    ? "text-[#d4c5b0]"
                    : "text-[#6b5d4d]"
              }`}
            >
              {item.label}
            </span>
          )}
        </div>

        {!collapsed && item.badge && (
          <span
            className={`ml-3 rounded-full px-2 py-0.5 text-[10px] font-semibold ${
              isActive
                ? "bg-white/20 text-white"
                : darkTheme
                  ? "bg-white/10 text-[#f3deb1]"
                  : "bg-black/10 text-[#7a5a1e]"
            }`}
          >
            {item.badge}
          </span>
        )}

        {mobile && item.disabled && !collapsed && (
          <span className="ml-3 text-[10px] font-semibold text-[#c9983a]">
            Soon
          </span>
        )}
      </button>
    );
  };

  return (
    <div
      className={`min-h-screen relative overflow-hidden transition-colors ${
        darkTheme
          ? "bg-gradient-to-br from-[#1a1512] via-[#231c17] to-[#2d241d]"
          : "bg-gradient-to-br from-[#c4b5a0] via-[#b8a590] to-[#a89780]"
      }`}
    >
      <div className="fixed inset-0 opacity-40 pointer-events-none">
        <div
          className={`absolute top-0 left-0 w-[800px] h-[800px] bg-gradient-radial blur-[100px] ${
            darkTheme
              ? "from-[#c9983a]/10 to-transparent"
              : "from-[#d4c4b0]/30 to-transparent"
          }`}
        />
        <div
          className={`absolute bottom-0 right-0 w-[900px] h-[900px] bg-gradient-radial blur-[120px] ${
            darkTheme
              ? "from-[#c9983a]/5 to-transparent"
              : "from-[#b8a898]/20 to-transparent"
          }`}
        />
      </div>

      {isDesktop && (
        <aside
          className={`fixed top-2 left-2 bottom-2 z-40 transition-all duration-300 ${
            isSidebarCollapsed ? "w-[84px]" : "w-64"
          }`}
        >
          <button
            type="button"
            onClick={() => setIsSidebarCollapsed((prev) => !prev)}
            className={`absolute -right-3 top-[58px] z-50 rounded-full border w-6 h-6 shadow-md hover:shadow-lg transition-all flex items-center justify-center ${
              darkTheme
                ? "bg-[#2d2820]/[0.9] border-[rgba(201,152,58,0.2)]"
                : "bg-white/[0.9] border-[rgba(245,239,235,0.32)]"
            }`}
            aria-label={isSidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            <ChevronRight
              className={`w-3 h-3 text-[#c9983a] transition-transform duration-300 ${
                isSidebarCollapsed ? "" : "rotate-180"
              }`}
            />
          </button>

          <div
            className={`h-full backdrop-blur-[90px] rounded-[29px] border shadow-[0px_4px_4px_rgba(0,0,0,0.25)] overflow-hidden transition-colors ${
              darkTheme
                ? "bg-[#2d2820]/[0.42] border-white/10"
                : "bg-white/[0.35] border-white/20"
            }`}
          >
            <div className="flex flex-col h-full py-8">
              <div
                className={`flex items-center mb-6 ${
                  isSidebarCollapsed ? "px-4 justify-center" : "px-4 justify-start"
                }`}
              >
                {isSidebarCollapsed ? (
                  <img
                    src={grainlifyLogo}
                    alt="Grainlify"
                    className="w-11 h-11 grainlify-logo"
                  />
                ) : (
                  <div className="flex items-center space-x-3 min-w-0">
                    <img
                      src={grainlifyLogo}
                      alt="Grainlify"
                      className="w-11 h-11 grainlify-logo flex-shrink-0"
                    />
                    <span
                      className={`text-[20px] font-bold truncate ${
                        darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"
                      }`}
                    >
                      Grainlify
                    </span>
                  </div>
                )}
              </div>

              <div
                className="h-px opacity-20 mb-6 mx-4"
                style={{
                  background:
                    "linear-gradient(90deg, transparent 0%, rgba(201,152,58,0.8) 50%, transparent 100%)",
                }}
              />

              <div className="flex-1 overflow-y-auto px-3 scrollbar-hide">
                <div className="mb-6">
                  {!isSidebarCollapsed && (
                    <p
                      className={`px-2 mb-2 text-[11px] font-semibold uppercase tracking-[0.14em] ${
                        darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
                      }`}
                    >
                      Primary
                    </p>
                  )}
                  <nav className="space-y-2" aria-label="Primary navigation">
                    {primaryNavItems.map((item) =>
                      renderNavButton(item, {
                        collapsed: isSidebarCollapsed,
                      }),
                    )}
                  </nav>
                </div>

                <div>
                  {!isSidebarCollapsed && (
                    <p
                      className={`px-2 mb-2 text-[11px] font-semibold uppercase tracking-[0.14em] ${
                        darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
                      }`}
                    >
                      Secondary
                    </p>
                  )}
                  <nav className="space-y-2" aria-label="Secondary navigation">
                    {secondaryNavItems.map((item) =>
                      renderNavButton(item, {
                        collapsed: isSidebarCollapsed,
                      }),
                    )}
                  </nav>

                  {!isSidebarCollapsed && (
                    <div
                      className={`mt-4 rounded-[16px] border px-4 py-3 ${
                        darkTheme
                          ? "bg-white/[0.05] border-white/10"
                          : "bg-white/[0.25] border-white/30"
                      }`}
                    >
                      <div className="flex items-center justify-between gap-3">
                        <div className="min-w-0">
                          <p
                            className={`text-sm font-semibold ${
                              darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"
                            }`}
                          >
                            Docs
                          </p>
                          <p
                            className={`text-xs truncate ${
                              darkTheme ? "text-[#cdbda6]" : "text-[#6b5d4d]"
                            }`}
                          >
                            Product structure & guidelines
                          </p>
                        </div>

                        <a
                          href="https://github.com/Jagadeeshftw/grainlify"
                          target="_blank"
                          rel="noreferrer"
                          className={`inline-flex items-center justify-center rounded-full w-9 h-9 transition ${
                            darkTheme
                              ? "bg-white/[0.08] hover:bg-white/[0.12] text-[#f3deb1]"
                              : "bg-black/[0.06] hover:bg-black/[0.1] text-[#7a5a1e]"
                          }`}
                          aria-label="Open Grainlify repository in a new tab"
                        >
                          <ArrowUpRight className="w-4 h-4" />
                        </a>
                      </div>
                    </div>
                  )}
                </div>
              </div>

              <div className="px-3 pt-4">
                <button
                  type="button"
                  onClick={handleAdminClick}
                  className={`w-full rounded-[14px] border px-4 py-3 flex items-center ${
                    isSidebarCollapsed ? "justify-center" : "justify-between"
                  } transition ${
                    darkTheme
                      ? "bg-white/[0.04] hover:bg-white/[0.08] border-white/10"
                      : "bg-white/[0.18] hover:bg-white/[0.26] border-white/25"
                  }`}
                >
                  <div className="flex items-center min-w-0">
                    <Shield className="w-5 h-5 text-[#c9983a] flex-shrink-0" />
                    {!isSidebarCollapsed && (
                      <span
                        className={`ml-3 text-sm font-medium truncate ${
                          darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"
                        }`}
                      >
                        Admin
                      </span>
                    )}
                  </div>

                  {!isSidebarCollapsed && !adminAuthenticated && (
                    <span className="text-[10px] font-semibold text-[#c9983a]">
                      Secure
                    </span>
                  )}
                </button>
              </div>
            </div>
          </div>
        </aside>
      )}

      {mobileMenuOpen && !isDesktop && (
        <div className="fixed inset-0 z-[9998] lg:hidden">
          <button
            type="button"
            aria-label="Close navigation drawer"
            className="absolute inset-0 bg-black/55 backdrop-blur-[2px]"
            onClick={closeMobileNav}
          />
          <aside
            className={`absolute inset-y-0 left-0 w-[min(90vw,360px)] border-r shadow-2xl backdrop-blur-[90px] ${
              darkTheme
                ? "bg-[#221b16]/[0.96] border-white/10"
                : "bg-[#efe5d6]/[0.96] border-white/40"
            }`}
          >
            <div className="flex h-full flex-col">
              <div className="flex items-center justify-between gap-3 px-4 py-4 border-b border-white/10">
                <div className="flex items-center min-w-0 gap-3">
                  <img
                    src={grainlifyLogo}
                    alt="Grainlify"
                    className="w-10 h-10 grainlify-logo flex-shrink-0"
                  />
                  <div className="min-w-0">
                    <p
                      className={`font-semibold truncate ${
                        darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"
                      }`}
                    >
                      Grainlify
                    </p>
                    <p
                      className={`text-xs ${
                        darkTheme ? "text-[#cdbda6]" : "text-[#6b5d4d]"
                      }`}
                    >
                      App navigation
                    </p>
                  </div>
                </div>

                <button
                  type="button"
                  onClick={closeMobileNav}
                  className={`rounded-full p-2 transition ${
                    darkTheme
                      ? "hover:bg-white/[0.08] text-[#f5efe5]"
                      : "hover:bg-black/[0.06] text-[#2d2820]"
                  }`}
                  aria-label="Close menu"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <div className="px-4 py-4 border-b border-white/10">
                <button
                  type="button"
                  onClick={() => {
                    setCurrentPage("search");
                    closeMobileNav();
                  }}
                  className={`w-full h-12 rounded-[16px] border flex items-center px-4 text-left transition ${
                    darkTheme
                      ? "bg-white/[0.05] border-white/10 hover:bg-white/[0.08]"
                      : "bg-white/[0.28] border-white/30 hover:bg-white/[0.4]"
                  }`}
                >
                  <Search
                    className={`w-4 h-4 mr-3 ${
                      darkTheme ? "text-[#d7c4a3]" : "text-[#7a5a1e]"
                    }`}
                  />
                  <span
                    className={`text-sm ${
                      darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"
                    }`}
                  >
                    Search projects, issues, contributors
                  </span>
                </button>
              </div>

              <div className="flex-1 overflow-y-auto px-4 py-4">
                <div className="mb-6">
                  <p
                    className={`mb-2 text-[11px] font-semibold uppercase tracking-[0.14em] ${
                      darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Primary
                  </p>
                  <nav className="space-y-2" aria-label="Mobile primary navigation">
                    {primaryNavItems.map((item) => renderNavButton(item, { mobile: true }))}
                  </nav>
                </div>

                <div className="mb-6">
                  <p
                    className={`mb-2 text-[11px] font-semibold uppercase tracking-[0.14em] ${
                      darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Secondary
                  </p>
                  <nav className="space-y-2" aria-label="Mobile secondary navigation">
                    {secondaryNavItems.map((item) => renderNavButton(item, { mobile: true }))}
                  </nav>
                </div>

                <div
                  className={`rounded-[16px] border p-4 ${
                    darkTheme
                      ? "bg-white/[0.05] border-white/10"
                      : "bg-white/[0.28] border-white/30"
                  }`}
                >
                  <p
                    className={`text-sm font-semibold mb-1 ${
                      darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"
                    }`}
                  >
                    Docs
                  </p>
                  <p
                    className={`text-xs mb-3 ${
                      darkTheme ? "text-[#cdbda6]" : "text-[#6b5d4d]"
                    }`}
                  >
                    Open project docs and repository in a new tab.
                  </p>
                  <a
                    href="https://github.com/Jagadeeshftw/grainlify"
                    target="_blank"
                    rel="noreferrer"
                    className={`inline-flex items-center gap-2 text-sm font-medium ${
                      darkTheme ? "text-[#f3deb1]" : "text-[#7a5a1e]"
                    }`}
                  >
                    Open docs
                    <ArrowUpRight className="w-4 h-4" />
                  </a>
                </div>
              </div>

              <div className="border-t border-white/10 px-4 py-4 space-y-3">
                <RoleSwitcher
                  currentRole={activeRole}
                  isSmallDevice={true}
                  showMobileNav={true}
                  closeMobileNav={closeMobileNav}
                  onRoleChange={handleRoleChange}
                />

                <div className="flex gap-3">
                  <button
                    ref={themeToggleRef}
                    type="button"
                    onClick={() => {
                      toggleSwitchTheme();
                    }}
                    className={`flex-1 h-12 rounded-[16px] border flex items-center justify-center gap-2 transition ${
                      darkTheme
                        ? "bg-white/[0.05] border-white/10 text-[#f5efe5]"
                        : "bg-white/[0.28] border-white/30 text-[#2d2820]"
                    }`}
                  >
                    {darkTheme ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
                    <span className="text-sm font-medium">
                      {darkTheme ? "Light mode" : "Dark mode"}
                    </span>
                  </button>

                  <button
                    type="button"
                    onClick={handleAdminClick}
                    className={`h-12 px-4 rounded-[16px] border flex items-center justify-center transition ${
                      darkTheme
                        ? "bg-white/[0.05] border-white/10 text-[#f5efe5]"
                        : "bg-white/[0.28] border-white/30 text-[#2d2820]"
                    }`}
                    aria-label="Open admin authentication"
                  >
                    <Shield className="w-4 h-4 text-[#c9983a]" />
                  </button>
                </div>

                <button
                  type="button"
                  onClick={handleLogout}
                  className={`w-full h-12 rounded-[16px] border text-sm font-medium transition ${
                    darkTheme
                      ? "bg-white/[0.05] border-white/10 text-[#f5efe5] hover:bg-white/[0.08]"
                      : "bg-white/[0.28] border-white/30 text-[#2d2820] hover:bg-white/[0.4]"
                  }`}
                >
                  Log out
                </button>
              </div>
            </div>
          </aside>
        </div>
      )}

      <main
        className={`relative z-10 my-2 mr-2 transition-all duration-300 ${
          isDesktop ? (isSidebarCollapsed ? "ml-[94px]" : "ml-[274px]") : "ml-2"
        }`}
      >
        <div className="max-w-[1440px] mx-auto">
          <header
            className={`fixed top-2 right-2 z-[9997] rounded-[26px] border backdrop-blur-[90px] shadow-[0px_4px_4px_rgba(0,0,0,0.25)] transition-all duration-300 ${
              darkTheme
                ? "bg-[#2d2820]/[0.42] border-white/10"
                : "bg-white/[0.35] border-white/20"
            }`}
            style={{
              left: isDesktop
                ? isSidebarCollapsed
                  ? "94px"
                  : "274px"
                : "8px",
            }}
          >
            <div className="flex items-center gap-2 px-3 py-3 min-h-[56px]">
              {!isDesktop && (
                <>
                  <button
                    type="button"
                    onClick={() => setMobileMenuOpen(true)}
                    className={`inline-flex items-center justify-center rounded-full w-10 h-10 transition ${
                      darkTheme
                        ? "hover:bg-white/[0.08] text-[#f5efe5]"
                        : "hover:bg-black/[0.06] text-[#2d2820]"
                    }`}
                    aria-label="Open navigation menu"
                  >
                    <Menu className="w-5 h-5" />
                  </button>

                  <Link to="/" className="flex items-center gap-2 min-w-0 mr-auto">
                    <img
                      src={grainlifyLogo}
                      alt="Grainlify"
                      className="w-8 h-8 grainlify-logo flex-shrink-0"
                    />
                    <span
                      className={`text-base font-semibold truncate ${
                        darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"
                      }`}
                    >
                      Grainlify
                    </span>
                  </Link>
                </>
              )}

              <button
                type="button"
                onClick={() => {
                  setCurrentPage("search");
                  closeMobileNav();
                }}
                className={`relative h-[46px] rounded-[23px] overflow-hidden border transition-all hover:scale-[1.01] ${
                  darkTheme
                    ? "bg-[#2d2820] border-white/10"
                    : "bg-[#d4c5b0] border-white/30"
                } ${isDesktop ? "flex-1" : "hidden sm:flex flex-1"}`}
              >
                <div className="relative h-full flex items-center px-4 justify-between">
                  <div className="flex items-center flex-1 min-w-0">
                    <Search
                      className={`w-4 h-4 mr-3 flex-shrink-0 ${
                        darkTheme
                          ? "text-[rgba(255,255,255,0.69)]"
                          : "text-[rgba(45,40,32,0.75)]"
                      }`}
                    />
                    <span
                      className={`truncate text-[13px] ${
                        darkTheme
                          ? "text-[rgba(255,255,255,0.5)]"
                          : "text-[rgba(45,40,32,0.5)]"
                      }`}
                    >
                      Search projects, issues, contributors...
                    </span>
                  </div>

                  {isDesktop && (
                    <div
                      className="hidden xl:flex items-center gap-1.5 px-2 py-1 rounded border"
                      style={{
                        backgroundColor: darkTheme
                          ? "rgba(255,255,255,0.08)"
                          : "rgba(0,0,0,0.08)",
                        borderColor: darkTheme
                          ? "rgba(255,255,255,0.2)"
                          : "rgba(0,0,0,0.15)",
                      }}
                    >
                      <span
                        className="text-[11px] font-medium"
                        style={{
                          color: darkTheme
                            ? "rgba(255,255,255,0.7)"
                            : "rgba(0,0,0,0.7)",
                        }}
                      >
                        ⌘
                      </span>
                      <span
                        className="text-[11px] font-medium"
                        style={{
                          color: darkTheme
                            ? "rgba(255,255,255,0.7)"
                            : "rgba(0,0,0,0.7)",
                        }}
                      >
                        K
                      </span>
                    </div>
                  )}
                </div>
              </button>

              {isDesktop && (
                <RoleSwitcher
                  currentRole={activeRole}
                  isSmallDevice={!isDesktop}
                  showMobileNav={false}
                  closeMobileNav={closeMobileNav}
                  onRoleChange={handleRoleChange}
                />
              )}

              {isDesktop && (
                <button
                  ref={themeToggleRef}
                  type="button"
                  onClick={() => toggleSwitchTheme()}
                  className={`h-[46px] w-[46px] rounded-full relative items-center justify-center backdrop-blur-[40px] transition-all hover:scale-105 hidden lg:flex ${
                    darkTheme ? "bg-[#2d2820] text-[#e8dfd0]" : "bg-[#d4c5b0] text-[#2d2820]"
                  }`}
                  title={darkTheme ? "Switch to light mode" : "Switch to dark mode"}
                >
                  {darkTheme ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
                </button>
              )}

              <NotificationsDropdown
                showMobileNav={false}
                closeMobileNav={closeMobileNav}
              />

              <UserProfileDropdown
                onPageChange={handleNavigation}
                showMobileNav={false}
              />
            </div>
          </header>

          <div className="pt-[74px]">
            <div
              className={`mb-4 rounded-[22px] border px-4 py-3 backdrop-blur-[60px] ${
                darkTheme
                  ? "bg-white/[0.05] border-white/10"
                  : "bg-white/[0.25] border-white/25"
              }`}
            >
              <div className="flex flex-wrap items-center gap-2 text-sm">
                {breadcrumbs.map((item, index) => {
                  const isLast = index === breadcrumbs.length - 1;

                  return (
                    <div key={item.key} className="flex items-center gap-2 min-w-0">
                      <span
                        className={`truncate max-w-[180px] ${
                          isLast
                            ? darkTheme
                              ? "text-[#f5efe5] font-medium"
                              : "text-[#2d2820] font-medium"
                            : darkTheme
                              ? "text-[#cdbda6]"
                              : "text-[#6b5d4d]"
                        }`}
                      >
                        {item.label}
                      </span>
                      {!isLast && (
                        <ChevronRight
                          className={`w-4 h-4 ${
                            darkTheme ? "text-[#9f8b74]" : "text-[#8e7d6d]"
                          }`}
                        />
                      )}
                    </div>
                  );
                })}
              </div>
            </div>

            {selectedIssue ? (
              <IssueDetailPage
                issueId={selectedIssue.issueId}
                projectId={selectedIssue.projectId}
                onClose={() => setSelectedIssue(null)}
              />
            ) : selectedProjectId ? (
              <ProjectDetailPage
                projectId={selectedProjectId}
                backLabel={
                  projectBackTarget === "browse"
                    ? "Back to Browse"
                    : projectBackTarget === "profile"
                      ? "Back to Profile"
                      : projectBackTarget === "leaderboard"
                        ? "Back to Leaderboard"
                        : projectBackTarget === "ecosystems"
                          ? "Back to Ecosystems"
                          : projectBackTarget === "discover"
                            ? "Back to Discover"
                            : "Back"
                }
                onBack={() => {
                  setSelectedProjectId(null);
                  if (projectBackTarget) {
                    setCurrentPage(projectBackTarget as any);
                  }
                  setProjectBackTarget(null);
                }}
                onIssueClick={(issueId, projectId) =>
                  setSelectedIssue({ issueId, projectId })
                }
              />
            ) : (
              <>
                {currentPage === "discover" && (
                  <DiscoverPage
                    onGoToBilling={() => {
                      setSettingsInitialTab("billing");
                      setCurrentPage("settings");
                    }}
                    onGoToOpenSourceWeek={() => setCurrentPage("osw")}
                  />
                )}

                {currentPage === "browse" && (
                  <BrowsePage
                    onProjectClick={(id) => {
                      setSelectedProjectId(id);
                      setProjectBackTarget("browse");
                    }}
                  />
                )}

                {currentPage === "osw" && !selectedEventId && (
                  <OpenSourceWeekPage
                    onEventClick={(id, name) => {
                      setSelectedEventId(id);
                      setSelectedEventName(name);
                    }}
                  />
                )}

                {currentPage === "osw" && selectedEventId && selectedEventName && (
                  <OpenSourceWeekDetailPage
                    eventId={selectedEventId}
                    eventName={selectedEventName}
                    onBack={() => {
                      setSelectedEventId(null);
                      setSelectedEventName(null);
                    }}
                  />
                )}

                {currentPage === "ecosystems" && !selectedEcosystemId && (
                  <EcosystemsPage onEcosystemClick={handleEcosystemClick} />
                )}

                {currentPage === "ecosystems" &&
                  selectedEcosystemId &&
                  selectedEcosystemName && (
                    <EcosystemDetailPage
                      ecosystemId={selectedEcosystemId}
                      ecosystemName={selectedEcosystemName}
                      initialDescription={selectedEcosystemDescription}
                      initialLogoUrl={selectedEcosystemLogoUrl}
                      onBack={handleBackFromEcosystem}
                      onProjectClick={(id) => {
                        setSelectedProjectId(id);
                        setProjectBackTarget("ecosystems");
                      }}
                    />
                  )}

                {currentPage === "contributors" && <ContributorsPage />}
                {currentPage === "maintainers" && <MaintainersPage />}

                {currentPage === "profile" && (
                  <ProfilePage
                    viewingUserId={viewingUserId}
                    viewingUserLogin={viewingUserLogin}
                    onBack={() => {
                      setViewingUserId(null);
                      setViewingUserLogin(null);
                      setCurrentPage("leaderboard");
                      window.history.replaceState({}, "", "/dashboard?page=leaderboard");
                    }}
                    onProjectClick={(id) => {
                      setSelectedProjectId(id);
                      setProjectBackTarget("profile");
                      setCurrentPage("discover");
                    }}
                    onIssueClick={(issueId, projectId) => {
                      setSelectedProjectId(projectId);
                      setSelectedIssue({ issueId, projectId });
                      setCurrentPage("discover");
                    }}
                  />
                )}

                {currentPage === "data" && adminAuthenticated && <DataPage />}
                {currentPage === "leaderboard" && <LeaderboardPage />}
                {currentPage === "blog" && <BlogPage />}

                {currentPage === "settings" && (
                  <SettingsPage initialTab={settingsInitialTab} />
                )}

                {currentPage === "admin" && adminAuthenticated && <AdminPage />}

                {currentPage === "admin" && !adminAuthenticated && (
                  <div className="flex items-center justify-center min-h-[60vh]">
                    <div
                      className={`text-center p-8 rounded-[24px] backdrop-blur-[40px] border ${
                        darkTheme
                          ? "bg-white/[0.08] border-white/10 text-[#d4d4d4]"
                          : "bg-white/[0.15] border-white/25 text-[#7a6b5a]"
                      }`}
                    >
                      <Shield className="w-16 h-16 mx-auto mb-4 text-[#c9983a]" />
                      <h2
                        className={`text-2xl font-bold mb-2 ${
                          darkTheme ? "text-[#f5f5f5]" : "text-[#2d2820]"
                        }`}
                      >
                        Admin Access Required
                      </h2>
                      <p className="mb-4">Enter the admin password to continue.</p>
                      <button
                        type="button"
                        onClick={() => setShowAdminPasswordModal(true)}
                        className="px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_10px_30px_rgba(162,121,44,0.5)] transition-all"
                      >
                        Authenticate
                      </button>
                    </div>
                  </div>
                )}

                {currentPage === "notifications" && <NotificationsPage />}

                {currentPage === "search" && (
                  <SearchPage
                    onBack={() => setCurrentPage("discover")}
                    onIssueClick={(id) => {
                      setSelectedIssue({ issueId: id });
                      setCurrentPage("discover");
                    }}
                    onProjectClick={(id) => {
                      setSelectedProjectId(id);
                      setProjectBackTarget("discover");
                      setCurrentPage("discover");
                    }}
                    onContributorClick={() => {
                      setCurrentPage("contributors");
                    }}
                  />
                )}
              </>
            )}
          </div>
        </div>
      </main>

      <Modal
        isOpen={showAdminPasswordModal}
        onClose={() => {
          setShowAdminPasswordModal(false);
          setAdminPassword("");
        }}
        title="Admin Authentication"
        icon={<Shield className="w-6 h-6 text-[#c9983a]" />}
        width="md"
      >
        <form onSubmit={handleAdminPasswordSubmit}>
          <div className="space-y-4">
            <p className={`text-sm ${darkTheme ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}>
              Enter the admin password to access the admin panel.
            </p>

            <ModalInput
              type="password"
              placeholder="Enter admin password"
              value={adminPassword}
              onChange={(value) => setAdminPassword(value)}
              required
              autoFocus
            />

            <p className={`text-xs ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
              Tip: This must match the backend `ADMIN_BOOTSTRAP_TOKEN`.
            </p>
          </div>

          <ModalFooter>
            <ModalButton
              variant="secondary"
              onClick={() => {
                setShowAdminPasswordModal(false);
                setAdminPassword("");
              }}
              disabled={isAuthenticating}
            >
              Cancel
            </ModalButton>

            <ModalButton
              variant="primary"
              type="submit"
              disabled={isAuthenticating || !adminPassword.trim()}
            >
              {isAuthenticating ? "Authenticating..." : "Authenticate"}
            </ModalButton>
          </ModalFooter>
        </form>
      </Modal>
    </div>
  );
}