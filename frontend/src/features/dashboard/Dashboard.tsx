import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import {
  Search,
  Bell,
  Compass,
  Grid3x3,
  Calendar,
  Globe,
  Users,
  FolderGit2,
  Trophy,
  Database,
  Plus,
  FileText,
  ChevronRight,
  Sparkles,
  Heart,
  Star,
  GitFork,
  ArrowUpRight,
  Target,
  Zap,
  ChevronDown,
  CircleDot,
  Clock,
  Moon,
  Sun,
  Shield,
  Code,
} from "lucide-react";
import { useAuth } from "../../shared/contexts/AuthContext";
import grainlifyLogo from "../../assets/grainlify_log.svg";
import { useTheme } from "../../shared/contexts/ThemeContext";
import { LanguageIcon } from "../../shared/components/LanguageIcon";
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
import { SettingsTabType } from "../settings/types";

export function Dashboard() {
  const { userRole, logout, login } = useAuth();
  const { theme, toggleTheme } = useTheme();
  const navigate = useNavigate();
  // const [currentPage, setCurrentPage] = useState('discover');
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(
    null,
  );
  const [selectedIssue, setSelectedIssue] = useState<{
    issueId: string;
    projectId?: string;
  } | null>(null);
  const [selectedEcosystemId, setSelectedEcosystemId] = useState<string | null>(
    null,
  );
  const [selectedEcosystemName, setSelectedEcosystemName] = useState<
    string | null
  >(null);
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);
  const [selectedEventName, setSelectedEventName] = useState<string | null>(
    null,
  );
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [activeRole, setActiveRole] = useState<
    "contributor" | "maintainer" | "admin"
  >("contributor");
  const [isSearchModalOpen, setIsSearchModalOpen] = useState(false);
  const [viewingUserId, setViewingUserId] = useState<string | null>(null);
  const [viewingUserLogin, setViewingUserLogin] = useState<string | null>(null);
  const [settingsInitialTab, setSettingsInitialTab] =
    useState<SettingsTabType>("profile");

  // ******************************************

  // Persist current tab across reload
  const [currentPage, setCurrentPage] = useState(() => {
    const params = new URLSearchParams(window.location.search);
    const tabFromUrl = params.get("tab");
    if (tabFromUrl) return tabFromUrl;

    return localStorage.getItem("dashboardTab") || "discover";
  });

  // Admin password gating (bootstrap token)
  const [showAdminPasswordModal, setShowAdminPasswordModal] = useState(false);
  const [adminPassword, setAdminPassword] = useState("");
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [adminAuthenticated, setAdminAuthenticated] = useState(() => {
    return sessionStorage.getItem("admin_authenticated") === "true";
  });
  const [pendingAdminTarget, setPendingAdminTarget] = useState<
    "nav" | "role" | null
  >(null);

  // Check URL params for viewing other users' profiles
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const userParam = params.get("user");
    const pageParam = params.get("page");

    if (pageParam === "profile" && userParam) {
      setCurrentPage("profile");
      // Check if it's a UUID (user_id) or a username (login)
      const uuidRegex =
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      if (uuidRegex.test(userParam)) {
        setViewingUserId(userParam);
        setViewingUserLogin(null);
      } else {
        setViewingUserLogin(userParam);
        setViewingUserId(null);
      }
    }
  }, []);

  // *******************************
  useEffect(() => {
    // Save tab in URL + localStorage whenever it changes
    const params = new URLSearchParams(window.location.search);
    params.set("tab", currentPage);
    const newUrl = `${window.location.pathname}?${params.toString()}`;
    window.history.replaceState({}, "", newUrl);

    localStorage.setItem("dashboardTab", currentPage);
  }, [currentPage]);

  // Example tab list
  const tabs = [
    "discover",
    "browse",
    "open-source-week",
    "ecosystems",
    "contributors",
    "settings",
  ];

  // Keyboard shortcut for search (Cmd+K / Ctrl+K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setCurrentPage("search");
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleNavigation = (page: string) => {
    setCurrentPage(page);
    setSelectedProjectId(null);
    setSelectedIssue(null);
    setSelectedEcosystemId(null);
    setSelectedEcosystemName(null);
    setSelectedEventId(null);
    setSelectedEventName(null);
  };

  const handleLogout = () => {
    logout();
    setAdminAuthenticated(false);
    sessionStorage.removeItem("admin_authenticated");
    navigate("/");
  };

  const openAdminAuthModal = (target: "nav" | "role") => {
    setPendingAdminTarget(target);
    setShowAdminPasswordModal(true);
  };

  const handleAdminClick = () => {
    if (adminAuthenticated) {
      setActiveRole("admin");
      handleNavigation("admin");
      return;
    }
    openAdminAuthModal("nav");
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
      // Keep UI clean: show a simple message; avoid browser alert spam.
      // The ModalInput will remain so user can retry.
      setAdminPassword("");
    } finally {
      setIsAuthenticating(false);
      setPendingAdminTarget(null);
    }
  };

  const handleRoleChange = (role: "contributor" | "maintainer" | "admin") => {
    if (role === "admin") {
      if (adminAuthenticated) {
        setActiveRole("admin");
        handleNavigation("admin");
      } else {
        // Non-admin users can click admin, but must authenticate first.
        openAdminAuthModal("role");
      }
      return;
    }
    setActiveRole(role);
    // Auto-navigate based on role and clear selections
    setSelectedProjectId(null);
    setSelectedIssue(null);
    setSelectedEcosystemId(null);
    setSelectedEcosystemName(null);
    setSelectedEventId(null);
    setSelectedEventName(null);
    if (role === "maintainer") {
      setCurrentPage("maintainers");
    } else {
      setCurrentPage("discover");
    }
  };

  const handleEcosystemClick = (ecosystemId: string, ecosystemName: string) => {
    setSelectedEcosystemId(ecosystemId);
    setSelectedEcosystemName(ecosystemName);
  };

  const handleBackFromEcosystem = () => {
    setSelectedEcosystemId(null);
    setSelectedEcosystemName(null);
  };

  // Role-based navigation items
  const navItems = [
    { id: "discover", icon: Compass, label: "Discover" },
    { id: "browse", icon: Grid3x3, label: "Browse" },
    { id: "osw", icon: Calendar, label: "Open-Source Week" },
    { id: "ecosystems", icon: Globe, label: "Ecosystems" },
    // Show Contributors for contributors, Maintainers for maintainers
    activeRole === "maintainer" || activeRole === "admin"
      ? { id: "maintainers", icon: Users, label: "Maintainers" }
      : { id: "contributors", icon: Users, label: "Contributors" },
    // Data page is only visible to admin
    ...(activeRole === "admin"
      ? [{ id: "data", icon: Database, label: "Data" }]
      : []),
    { id: "leaderboard", icon: Trophy, label: "Leaderboard" },
    { id: "blog", icon: FileText, label: "Grainlify Blog" },
  ];

  const darkTheme = theme === "dark";

  return (
    <div
      className={`min-h-screen relative overflow-hidden transition-colors ${
        darkTheme
          ? "bg-gradient-to-br from-[#1a1512] via-[#231c17] to-[#2d241d]"
          : "bg-gradient-to-br from-[#c4b5a0] via-[#b8a590] to-[#a89780]"
      }`}
    >
      {/* Subtle Background Texture */}
      <div className="fixed inset-0 opacity-40">
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

      {/* Sidebar */}
      <aside
        className={`fixed top-2 left-2 bottom-2 z-50 transition-all duration-300 ${isSidebarCollapsed ? "w-[65px] mr-2" : "w-56 mr-2"}`}
      >
        {/* Toggle Arrow Button - positioned at top of sidebar aligned with header */}
        <button
          onClick={() => setIsSidebarCollapsed(!isSidebarCollapsed)}
          className={`absolute z-[100] backdrop-blur-[90px] rounded-full border-[0.5px] w-6 h-6 shadow-md hover:shadow-lg transition-all flex items-center justify-center ${
            isSidebarCollapsed ? "-right-3 top-[60px]" : "-right-3 top-[60px]"
          } ${
            darkTheme
              ? "bg-[#2d2820]/[0.85] border-[rgba(201,152,58,0.2)]"
              : "bg-white/[0.85] border-[rgba(245,239,235,0.32)]"
          }`}
        >
          <ChevronRight
            className={`w-3 h-3 text-[#c9983a] transition-transform duration-300 ${isSidebarCollapsed ? "" : "rotate-180"}`}
          />
        </button>

        <div
          className={`h-full backdrop-blur-[90px] rounded-[29px] border shadow-[0px_4px_4px_0px_rgba(0,0,0,0.25)] relative overflow-y-auto scrollbar-hide transition-colors ${
            darkTheme
              ? "bg-[#2d2820]/[0.4] border-white/10"
              : "bg-white/[0.35] border-white/20"
          }`}
        >
          <div className="flex flex-col h-full px-0 py-[40px]">
            {/* Logo/Avatar */}
            <div
              className={`flex items-center mb-6 transition-all ${isSidebarCollapsed ? "px-[8px] justify-center" : "px-2 justify-start"}`}
            >
              {isSidebarCollapsed ? (
                <img
                  src={grainlifyLogo}
                  alt="Grainlify"
                  className="w-12 h-12 grainlify-logo"
                />
              ) : (
                <div className="flex items-center space-x-3">
                  <img
                    src={grainlifyLogo}
                    alt="Grainlify"
                    className="w-12 h-12 grainlify-logo"
                  />
                  <span
                    className={`text-[20px] font-bold transition-colors ${
                      darkTheme ? "text-[#f5efe5]" : "text-[#2d2820]"
                    }`}
                  >
                    Grainlify
                  </span>
                </div>
              )}
            </div>

            {/* Divider */}
            <div
              className="h-[0.5px] opacity-[0.24] mb-6 mx-auto"
              style={{
                width: isSidebarCollapsed ? "60px" : "100%",
                backgroundImage:
                  'url(\'data:image/svg+xml;utf8,<svg viewBox="0 0 104 0.5" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="none"><rect x="0" y="0" height="100%" width="100%" fill="url(%23grad)" opacity="1"/><defs><radialGradient id="grad" gradientUnits="userSpaceOnUse" cx="0" cy="0" r="10" gradientTransform="matrix(3.1841e-16 0.025 -5.2 1.5308e-18 52 0.25)"><stop stop-color="rgba(67,44,44,1)" offset="0"/><stop stop-color="rgba(80,28,28,0)" offset="1"/></radialGradient></defs></svg>\')',
              }}
            />

            {/* Main Navigation */}
            <nav
              className={`space-y-2 mb-auto ${isSidebarCollapsed ? "px-[8px]" : "px-2"}`}
            >
              {navItems.map((item) => {
                const isActive = currentPage === item.id;
                const Icon = item.icon as any;
                return (
                  <button
                    key={item.id}
                    onClick={() => handleNavigation(item.id)}
                    className={`group w-full flex items-center rounded-[12px] transition-all duration-300 ${
                      isSidebarCollapsed
                        ? "justify-center px-0 h-[49px]"
                        : "justify-start px-3 py-2.5"
                    } ${
                      isActive
                        ? "bg-[#c9983a] shadow-[inset_0px_0px_4px_0px_rgba(255,255,255,0.25)] border-[0.5px] border-[rgba(245,239,235,0.16)]"
                        : darkTheme
                          ? "hover:bg-white/[0.08]"
                          : "hover:bg-white/[0.1]"
                    }`}
                    title={isSidebarCollapsed ? item.label : ""}
                  >
                    <Icon
                      className={`w-6 h-6 transition-colors ${isSidebarCollapsed ? "" : "flex-shrink-0"} ${
                        isActive
                          ? "text-white"
                          : darkTheme
                            ? "text-[#e8c77f]"
                            : "text-[#a2792c]"
                      }`}
                    />
                    {!isSidebarCollapsed && (
                      <span
                        className={`ml-3 font-medium text-[14px] ${
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
                  </button>
                );
              })}
            </nav>
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <main
        className={`mr-2 my-2 relative z-10 transition-all duration-300 ${isSidebarCollapsed ? "ml-[81px]" : "ml-[240px]"}`}
      >
        <div className="max-w-[1400px] mx-auto">
          {/* Premium Pill-Style Header - Greatest of All Time */}
          <div
            className={`fixed top-2 right-2 left-auto z-[9999] flex items-center gap-3 h-[52px] py-3 rounded-[26px] shadow-[0px_4px_4px_0px_rgba(0,0,0,0.25)] backdrop-blur-[90px] border transition-all duration-300 ${
              isSidebarCollapsed ? "ml-[81px]" : "ml-[240px]"
            } ${
              darkTheme
                ? "bg-[#2d2820]/[0.4] border-white/10 shadow-[inset_0px_0px_9px_0px_rgba(201,152,58,0.1)]"
                : "bg-white/[0.35] border-white shadow-[inset_0px_0px_9px_0px_rgba(255,255,255,0.5)]"
            }`}
            style={{
              width: `calc(100vw - ${isSidebarCollapsed ? "81px" : "240px"} - 8px - 8px)`,
            }}
          >
            {/* Search - Premium Pill Style */}
            <button
              onClick={() => setCurrentPage("search")}
              className={`relative h-[46px] flex-1 rounded-[23px] overflow-visible backdrop-blur-[40px] shadow-[0px_6px_6.5px_-1px_rgba(0,0,0,0.36),0px_0px_4.2px_0px_rgba(0,0,0,0.69)] ml-[3px] transition-all hover:scale-[1.01] cursor-pointer ${
                darkTheme ? "bg-[#2d2820]" : "bg-[#d4c5b0]"
              }`}
            >
              <div
                className={`absolute inset-0 pointer-events-none rounded-[23px] ${
                  darkTheme
                    ? "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.5),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.11)]"
                    : "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.15),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.35)]"
                }`}
              />
              <div className="relative h-full flex items-center px-5 justify-between">
                <div className="flex items-center flex-1">
                  <Search
                    className={`w-4 h-4 mr-3 flex-shrink-0 transition-colors ${
                      darkTheme
                        ? "text-[rgba(255,255,255,0.69)]"
                        : "text-[rgba(45,40,32,0.75)]"
                    }`}
                  />
                  <span
                    className={`text-[13px] transition-colors ${
                      darkTheme
                        ? "text-[rgba(255,255,255,0.5)]"
                        : "text-[rgba(45,40,32,0.5)]"
                    }`}
                  >
                    <span className="sm:hidden">Search</span>
                    <span className="hidden sm:inline md:hidden">
                      Search projects
                    </span>
                    <span className="hidden md:inline lg:hidden">
                      Search projects, issues
                    </span>
                    <span className="hidden lg:inline">
                      Search projects, issues, contributors...
                    </span>
                  </span>
                </div>

                <div
                  className="flex items-center gap-1.5 px-2 py-1 rounded border"
                  style={{
                    backgroundColor: darkTheme
                      ? "rgba(255, 255, 255, 0.08)"
                      : "rgba(0, 0, 0, 0.08)",
                    borderColor: darkTheme
                      ? "rgba(255, 255, 255, 0.2)"
                      : "rgba(0, 0, 0, 0.15)",
                  }}
                >
                  <span
                    className="text-[11px] font-medium"
                    style={{
                      color: darkTheme
                        ? "rgba(255, 255, 255, 0.7)"
                        : "rgba(0, 0, 0, 0.7)",
                    }}
                  >
                    ⌘
                  </span>
                  <span
                    className="text-[11px] font-medium"
                    style={{
                      color: darkTheme
                        ? "rgba(255, 255, 255, 0.7)"
                        : "rgba(0, 0, 0, 0.7)",
                    }}
                  >
                    K
                  </span>
                </div>
              </div>
            </button>

            {/* Role Switcher */}
            <RoleSwitcher
              currentRole={activeRole}
              onRoleChange={handleRoleChange}
            />

            {/* Theme Toggle - Separate Pill Button */}
            <button
              onClick={toggleTheme}
              className={`h-[46px] w-[46px] rounded-full overflow-clip relative flex items-center justify-center backdrop-blur-[40px] transition-all hover:scale-105 shadow-[0px_6px_6.5px_-1px_rgba(0,0,0,0.36),0px_0px_4.2px_0px_rgba(0,0,0,0.69)] ${
                darkTheme ? "bg-[#2d2820]" : "bg-[#d4c5b0]"
              }`}
              title={darkTheme ? "Switch to light mode" : "Switch to dark mode"}
            >
              <div
                className={`absolute inset-0 pointer-events-none rounded-full ${
                  darkTheme
                    ? "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.5),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.11)]"
                    : "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.15),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.35)]"
                }`}
              />
              {darkTheme ? (
                <Sun
                  className={`w-4 h-4 relative z-10 transition-colors ${
                    darkTheme
                      ? "text-[rgba(255,255,255,0.69)]"
                      : "text-[rgba(45,40,32,0.75)]"
                  }`}
                />
              ) : (
                <Moon
                  className={`w-4 h-4 relative z-10 transition-colors ${
                    darkTheme
                      ? "text-[rgba(255,255,255,0.69)]"
                      : "text-[rgba(45,40,32,0.75)]"
                  }`}
                />
              )}
            </button>

            {/* Notifications Dropdown */}
            <NotificationsDropdown />

            {/* User Profile Dropdown - Shows profile when authenticated, Sign In when not */}
            <UserProfileDropdown onPageChange={handleNavigation} />
          </div>

          {/* Page Content */}
          <div className="pt-[68px]">
            {selectedIssue ? (
              <IssueDetailPage
                issueId={selectedIssue.issueId}
                projectId={selectedIssue.projectId}
                onClose={() => setSelectedIssue(null)}
              />
            ) : selectedProjectId ? (
              <ProjectDetailPage
                projectId={selectedProjectId}
                onBack={() => setSelectedProjectId(null)}
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
                    onProjectClick={(id) => setSelectedProjectId(id)}
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
                {currentPage === "osw" &&
                  selectedEventId &&
                  selectedEventName && (
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
                      onBack={handleBackFromEcosystem}
                      onProjectClick={(id) => setSelectedProjectId(id)}
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
                      window.history.replaceState(
                        {},
                        "",
                        "/dashboard?page=leaderboard",
                      );
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
                        className={`text-2xl font-bold mb-2 ${darkTheme ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
                      >
                        Admin Access Required
                      </h2>
                      <p className="mb-4">
                        Enter the admin password to continue.
                      </p>
                      <button
                        onClick={() => openAdminAuthModal("nav")}
                        className="px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_10px_30px_rgba(162,121,44,0.5)] transition-all"
                      >
                        Authenticate
                      </button>
                    </div>
                  </div>
                )}
                {currentPage === "search" && (
                  <SearchPage
                    onBack={() => setCurrentPage("discover")}
                    onIssueClick={(id) => {
                      setSelectedIssue({ issueId: id });
                      setCurrentPage("discover");
                    }}
                    onProjectClick={(id) => {
                      setSelectedProjectId(id);
                      setCurrentPage("discover");
                    }}
                    onContributorClick={(id) => {
                      // Navigate to profile page or contributors page with selected contributor
                      setCurrentPage("contributors");
                    }}
                  />
                )}
              </>
            )}
          </div>
        </div>
      </main>

      {/* Admin Password Modal */}
      <Modal
        isOpen={showAdminPasswordModal}
        onClose={() => {
          setShowAdminPasswordModal(false);
          setAdminPassword("");
          setPendingAdminTarget(null);
        }}
        title="Admin Authentication"
        icon={<Shield className="w-6 h-6 text-[#c9983a]" />}
        width="md"
      >
        <form onSubmit={handleAdminPasswordSubmit}>
          <div className="space-y-4">
            <p
              className={`text-sm ${darkTheme ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
            >
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
            <p
              className={`text-xs ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}
            >
              Tip: This must match the backend `ADMIN_BOOTSTRAP_TOKEN`.
            </p>
          </div>
          <ModalFooter>
            <ModalButton
              variant="secondary"
              onClick={() => {
                setShowAdminPasswordModal(false);
                setAdminPassword("");
                setPendingAdminTarget(null);
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
