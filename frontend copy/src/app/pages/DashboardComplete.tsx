import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { 
  Search, Bell, Settings, LogOut, Compass, Grid3x3, Calendar, 
  Globe, Users, FolderGit2, Trophy, User, Database, Plus, 
  FileText, ChevronRight, Sparkles, Heart, 
  Star, GitFork, ArrowUpRight, Target, Zap, ChevronDown, 
  CircleDot, Clock, Moon, Sun, Shield, Send, X
} from 'lucide-react';
import grainlifyLogo from '../../assets/grainlify_log.svg';
import { useAuth } from '../../shared/contexts/AuthContext';
import { useTheme } from '../../shared/contexts/ThemeContext';
import { LanguageIcon } from '../../shared/components/LanguageIcon';
import { UserProfileDropdown } from '../../shared/components/UserProfileDropdown';
import { ContributorsPage } from '../../features/dashboard/pages/ContributorsPage';
import { BrowsePage } from '../../features/dashboard/pages/BrowsePage';
import { MaintainersPage } from '../../features/maintainers/pages/MaintainersPage';
import { ProfilePage } from '../../features/dashboard/pages/ProfilePage';
import { DataPage } from '../../features/dashboard/pages/DataPage';
import { LeaderboardPage } from '../../features/leaderboard/pages/LeaderboardPage';
import { BlogPage } from '../../features/blog/pages/BlogPage';
import { SettingsPage } from '../../features/settings/pages/SettingsPage';
import { bootstrapAdmin, setAuthToken } from '../../shared/api/client';
import { Modal, ModalFooter, ModalButton, ModalInput } from '../../shared/components/ui/Modal';

export function DashboardComplete() {
  const { userRole, logout } = useAuth();
  const { theme, toggleTheme } = useTheme();
  const navigate = useNavigate();
  const [currentPage, setCurrentPage] = useState('discover');
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [showAdminPasswordModal, setShowAdminPasswordModal] = useState(false);
  const [adminPassword, setAdminPassword] = useState('');
  const [isAuthenticating, setIsAuthenticating] = useState(false);
  const [adminAuthenticated, setAdminAuthenticated] = useState(() => {
    return sessionStorage.getItem('admin_authenticated') === 'true';
  });

  const handleLogout = () => {
    // Clear admin authentication on logout
    setAdminAuthenticated(false);
    sessionStorage.removeItem('admin_authenticated');
    logout();
    navigate('/');
  };

  const handleAdminClick = () => {
    // If already authenticated as admin in this session, go directly to admin page
    if (adminAuthenticated) {
      setCurrentPage('admin');
      return;
    }
    // Always show password modal for non-authenticated users (even if they have admin role)
    setShowAdminPasswordModal(true);
  };

  const handleAdminPasswordSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!adminPassword.trim()) return;

    setIsAuthenticating(true);
    try {
      const response = await bootstrapAdmin(adminPassword.trim());
      
      // Update token with the new admin token
      setAuthToken(response.token);
      
      // Mark as authenticated in this session
      setAdminAuthenticated(true);
      sessionStorage.setItem('admin_authenticated', 'true');
      
      // Close modal and navigate to admin page
      setShowAdminPasswordModal(false);
      setAdminPassword('');
      setCurrentPage('admin');
      
      // Refresh page to update auth context with new role
      window.location.reload();
    } catch (error) {
      console.error('Admin authentication failed:', error);
      alert(error instanceof Error ? error.message : 'Invalid password. Please try again.');
      setAdminPassword('');
    } finally {
      setIsAuthenticating(false);
    }
  };

  // Role-based navigation items
  const navItems = [
    { id: 'discover', icon: Compass, label: 'Discover' },
    { id: 'browse', icon: Grid3x3, label: 'Browse' },
    { id: 'osw', icon: Calendar, label: 'Open-Source Week' },
    { id: 'ecosystems', icon: Globe, label: 'Ecosystems' },
    // Show Contributors for contributors, Maintainers for maintainers
    userRole === 'maintainer' 
      ? { id: 'maintainers', icon: Users, label: 'Maintainers' }
      : { id: 'contributors', icon: Users, label: 'Contributors' },
    { id: 'profile', icon: User, label: 'Public Profile' },
    { id: 'data', icon: Database, label: 'Data' },
    { id: 'leaderboard', icon: Trophy, label: 'Leaderboard' },
    { id: 'blog', icon: FileText, label: 'Grainlify Blog' },
  ];

  return (
    <div className={`min-h-screen relative overflow-hidden transition-colors ${
      theme === 'dark'
        ? 'bg-gradient-to-br from-[#1a1512] via-[#231c17] to-[#2d241d]'
        : 'bg-gradient-to-br from-[#c4b5a0] via-[#b8a590] to-[#a89780]'
    }`}>
      {/* Subtle Background Texture */}
      <div className="fixed inset-0 opacity-40">
        <div className={`absolute top-0 left-0 w-[800px] h-[800px] bg-gradient-radial blur-[100px] ${
          theme === 'dark' ? 'from-[#c9983a]/10 to-transparent' : 'from-[#d4c4b0]/30 to-transparent'
        }`} />
        <div className={`absolute bottom-0 right-0 w-[900px] h-[900px] bg-gradient-radial blur-[120px] ${
          theme === 'dark' ? 'from-[#c9983a]/5 to-transparent' : 'from-[#b8a898]/20 to-transparent'
        }`} />
      </div>

      {/* Sidebar */}
      <aside className={`fixed top-4 left-4 bottom-4 z-50 transition-all duration-300 ${isSidebarCollapsed ? 'w-[93px]' : 'w-72'}`}>
        {/* Toggle Arrow Button - positioned at top of sidebar aligned with header */}
        <button
          onClick={() => setIsSidebarCollapsed(!isSidebarCollapsed)}
          className={`absolute z-[100] backdrop-blur-[90px] rounded-full border-[0.5px] w-8 h-8 shadow-md hover:shadow-lg transition-all flex items-center justify-center ${
            isSidebarCollapsed ? '-right-4 top-[60px]' : '-right-4 top-[60px]'
          } ${
            theme === 'dark'
              ? 'bg-[#2d2820]/[0.85] border-[rgba(201,152,58,0.2)]'
              : 'bg-white/[0.85] border-[rgba(245,239,235,0.32)]'
          }`}
        >
          <ChevronRight 
            className={`w-5 h-5 text-[#c9983a] transition-transform duration-300 ${isSidebarCollapsed ? '' : 'rotate-180'}`} 
          />
        </button>

        <div className={`h-full backdrop-blur-[90px] rounded-[29px] border shadow-[0px_4px_4px_0px_rgba(0,0,0,0.25)] relative overflow-y-auto scrollbar-hide transition-colors ${
          theme === 'dark'
            ? 'bg-[#2d2820]/[0.4] border-white/10 shadow-[inset_0px_0px_9px_0px_rgba(201,152,58,0.1)]'
            : 'bg-white/[0.35] border-white shadow-[inset_0px_0px_9px_0px_rgba(255,255,255,0.5)]'
        }`}>
          <div className="flex flex-col h-full px-0 py-[40px]">
            {/* Logo/Avatar */}
            <div className={`flex items-center mb-6 transition-all ${isSidebarCollapsed ? 'px-[18px] justify-center' : 'px-6 justify-start'}`}>
              {isSidebarCollapsed ? (
                <img src={grainlifyLogo} alt="Grainlify" className="w-12 h-12 grainlify-logo" />
              ) : (
                <div className="flex items-center space-x-3">
                  <img src={grainlifyLogo} alt="Grainlify" className="w-12 h-12 grainlify-logo" />
                  <span className={`text-[20px] font-bold transition-colors ${
                    theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>Grainlify</span>
                </div>
              )}
            </div>

            {/* Divider */}
            <div className="h-[0.5px] opacity-[0.24] mb-6 mx-auto" style={{ 
              width: isSidebarCollapsed ? '60px' : '100%',
              backgroundImage: "url('data:image/svg+xml;utf8,<svg viewBox=\\'0 0 104 0.5\\' xmlns=\\'http://www.w3.org/2000/svg\\' preserveAspectRatio=\\'none\\'><rect x=\\'0\\' y=\\'0\\' height=\\'100%\\' width=\\'100%\\' fill=\\'url(%23grad)\\' opacity=\\'1\\'/><defs><radialGradient id=\\'grad\\' gradientUnits=\\'userSpaceOnUse\\' cx=\\'0\\' cy=\\'0\\' r=\\'10\\' gradientTransform=\\'matrix(3.1841e-16 0.025 -5.2 1.5308e-18 52 0.25)\\'><stop stop-color=\\'rgba(67,44,44,1)\\' offset=\\'0\\'/><stop stop-color=\\'rgba(80,28,28,0)\\' offset=\\'1\\'/></radialGradient></defs></svg>')"
            }} />

            {/* Main Navigation */}
            <nav className={`space-y-2 mb-auto ${isSidebarCollapsed ? 'px-[18px]' : 'px-6'}`}>
              {navItems.map((item) => {
                const isActive = currentPage === item.id;
                return (
                  <button
                    key={item.id}
                    onClick={() => setCurrentPage(item.id)}
                    className={`group w-full flex items-center rounded-[12px] transition-all duration-300 ${
                      isSidebarCollapsed ? 'justify-center px-0 py-4' : 'justify-start px-3 py-2.5'
                    } ${
                      isActive
                        ? 'bg-[#c9983a] shadow-[inset_0px_0px_4px_0px_rgba(255,255,255,0.25)] border-[0.5px] border-[rgba(245,239,235,0.16)]'
                        : theme === 'dark'
                          ? 'hover:bg-white/[0.08]'
                          : 'hover:bg-white/[0.1]'
                    }`}
                    title={isSidebarCollapsed ? item.label : ''}
                  >
                    <item.icon className={`w-6 h-6 transition-colors ${isSidebarCollapsed ? '' : 'flex-shrink-0'} ${
                      isActive ? 'text-white' : theme === 'dark' ? 'text-[#e8c77f]' : 'text-[#a2792c]'
                    }`} />
                    {!isSidebarCollapsed && (
                      <span className={`ml-3 font-medium text-[14px] ${
                        isActive ? 'text-white' : theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#6b5d4d]'
                      }`}>
                        {item.label}
                      </span>
                    )}
                  </button>
                );
              })}
            </nav>

            {/* Bottom Section with Settings */}
            <div className={`mt-auto ${isSidebarCollapsed ? 'px-[18px]' : 'px-6'} pt-6 space-y-2`}>
              <button
                onClick={() => setCurrentPage('settings')}
                className={`w-full flex items-center rounded-[12px] transition-all py-4 hover:bg-white/[0.1] ${
                  isSidebarCollapsed ? 'justify-center px-0' : 'justify-start px-3'
                } ${
                  theme === 'dark' ? 'text-[#d4af37]' : 'text-[#a2792c]'
                }`}
                title={isSidebarCollapsed ? 'Settings' : ''}
              >
                <Settings className={`w-6 h-6 ${isSidebarCollapsed ? '' : 'flex-shrink-0'}`} />
                {!isSidebarCollapsed && <span className={`ml-3 font-medium text-[14px] ${
                  theme === 'dark' ? 'text-[#b8a898]' : 'text-[#6b5d4d]'
                }`}>Settings</span>}
              </button>

              <button
                onClick={handleLogout}
                className={`w-full flex items-center rounded-[12px] transition-all py-4 hover:bg-white/[0.1] ${
                  isSidebarCollapsed ? 'justify-center px-0' : 'justify-start px-3'
                } ${
                  theme === 'dark' ? 'text-[#d4af37]' : 'text-[#a2792c]'
                }`}
                title={isSidebarCollapsed ? 'Logout' : ''}
              >
                <LogOut className={`w-6 h-6 ${isSidebarCollapsed ? '' : 'flex-shrink-0'}`} />
                {!isSidebarCollapsed && <span className={`ml-3 font-medium text-[14px] ${
                  theme === 'dark' ? 'text-[#b8a898]' : 'text-[#6b5d4d]'
                }`}>Logout</span>}
              </button>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Content */}
      <main className={`mr-4 my-4 relative z-10 transition-all duration-300 ${isSidebarCollapsed ? 'ml-[117px]' : 'ml-80'}`}>
        <div className="max-w-[1400px] mx-auto">
          {/* Top Bar */}
          <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6 mb-6 transition-colors overflow-visible ${
            theme === 'dark'
              ? 'bg-[#2d2820]/[0.4] border-white/10'
              : 'bg-white/[0.12] border-white/20'
          }`}>
            <div className="flex items-center justify-between">
              {/* Search */}
              <div className="flex-1 max-w-lg">
                <div className="relative">
                  <Search className={`absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 transition-colors ${
                    theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`} />
                  <input
                    type="text"
                    placeholder="Search projects, issues, contributors..."
                    className={`w-full pl-12 pr-4 py-3.5 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0] placeholder-[#b8a898] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                  />
                </div>
              </div>

              {/* Right Actions */}
              <div className="flex items-center space-x-3 relative z-50">
                {/* Theme Toggle */}
                <button
                  onClick={toggleTheme}
                  className={`p-3 rounded-[14px] backdrop-blur-[30px] border transition-all shadow-[0_4px_12px_rgba(0,0,0,0.04)] ${
                    theme === 'dark'
                      ? 'bg-white/[0.08] border-white/15 text-[#d4af37] hover:bg-white/[0.12]'
                      : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                  }`}
                  title={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
                >
                  {theme === 'dark' ? <Sun className="w-5 h-5" /> : <Moon className="w-5 h-5" />}
                </button>

                <button className={`relative p-3 rounded-[14px] backdrop-blur-[30px] border transition-all shadow-[0_4px_12px_rgba(0,0,0,0.04)] ${
                  theme === 'dark'
                    ? 'bg-white/[0.08] border-white/15 text-[#d4af37] hover:bg-white/[0.12]'
                    : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                }`}>
                  <Bell className="w-5 h-5" />
                  <div className="absolute top-2 right-2 w-2 h-2 bg-[#c9983a] rounded-full shadow-[0_0_8px_rgba(201,152,58,0.6)]" />
                </button>

                <button
                  onClick={() => setCurrentPage('settings')}
                  className={`p-3 rounded-[14px] backdrop-blur-[30px] border transition-all shadow-[0_4px_12px_rgba(0,0,0,0.04)] ${
                    theme === 'dark'
                      ? 'bg-white/[0.08] border-white/15 text-[#d4af37] hover:bg-white/[0.12]'
                      : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                  }`}
                >
                  <Settings className="w-5 h-5" />
                </button>

                {/* User Profile Dropdown */}
                <UserProfileDropdown />
              </div>
            </div>
          </div>

          {/* Page Content */}
          {currentPage === 'discover' && <DiscoverPage />}
          {currentPage === 'browse' && <BrowsePage />}
          {currentPage === 'osw' && <OpenSourceWeekPage />}
          {currentPage === 'ecosystems' && <EcosystemsPage />}
          {currentPage === 'contributors' && <ContributorsPage />}
          {currentPage === 'maintainers' && <MaintainersPage onNavigate={setCurrentPage} />}
          {currentPage === 'profile' && <ProfilePage />}
          {currentPage === 'data' && <DataPage />}
          {currentPage === 'leaderboard' && <LeaderboardPage />}
          {currentPage === 'blog' && <BlogPage />}
          {currentPage === 'admin' && adminAuthenticated && <AdminPage />}
          {currentPage === 'admin' && !adminAuthenticated && (
            <div className="flex items-center justify-center h-full">
              <div className={`text-center p-8 rounded-[24px] backdrop-blur-[40px] border ${
                theme === 'dark'
                  ? 'bg-white/[0.08] border-white/10 text-[#d4d4d4]'
                  : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
              }`}>
                <Shield className="w-16 h-16 mx-auto mb-4 text-[#c9983a]" />
                <h2 className="text-2xl font-bold mb-2">Admin Access Required</h2>
                <p className="mb-4">Please authenticate to access the admin panel.</p>
                <button
                  onClick={() => setShowAdminPasswordModal(true)}
                  className="px-6 py-2 bg-[#c9983a] text-white rounded-[12px] hover:bg-[#a67c2e] transition-colors"
                >
                  Authenticate
                </button>
              </div>
            </div>
          )}
          {currentPage === 'settings' && <SettingsPage />}
        </div>
      </main>

      {/* Admin Password Modal */}
      <Modal
        isOpen={showAdminPasswordModal}
        onClose={() => {
          setShowAdminPasswordModal(false);
          setAdminPassword('');
        }}
        title="Admin Authentication"
        icon={<Shield className="w-6 h-6 text-[#c9983a]" />}
        width="md"
      >
        <form onSubmit={handleAdminPasswordSubmit}>
          <div className="space-y-4">
            <p className={`text-sm ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Enter the admin password to access the admin panel.
            </p>
            <ModalInput
              type="password"
              placeholder="Enter admin password"
              value={adminPassword}
              onChange={(e) => setAdminPassword(e.target.value)}
              required
              autoFocus
            />
          </div>
          <ModalFooter>
            <ModalButton
              variant="secondary"
              onClick={() => {
                setShowAdminPasswordModal(false);
                setAdminPassword('');
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
              {isAuthenticating ? 'Authenticating...' : 'Authenticate'}
            </ModalButton>
          </ModalFooter>
        </form>
      </Modal>
    </div>
  );
}

function DiscoverPage() {
  const { theme } = useTheme();
  const projects = [
    {
      id: 1,
      name: 'React Ecosystem',
      icon: '⚛️',
      stars: '4.9M',
      forks: '2.6M',
      issues: 650,
      description: 'A modern React ecosystem for building user interfaces with enhanced UI/UX modules.',
      tags: ['TypeScript', 'good first issue'],
      color: 'from-blue-500 to-cyan-500',
    },
    {
      id: 2,
      name: 'Nextjs Framework',
      icon: '▲',
      stars: '120K',
      forks: '24K',
      issues: 480,
      description: 'The React framework for production with server-side rendering.',
      tags: ['Frontend'],
      color: 'from-purple-500 to-pink-500',
    },
    {
      id: 3,
      name: 'Vue.js',
      icon: 'V',
      stars: '214K',
      forks: '36K',
      issues: 552,
      description: 'Progressive JavaScript framework for building user interfaces.',
      tags: ['Framework'],
      color: 'from-green-500 to-emerald-500',
    },
  ];

  return (
    <div className="space-y-6">
      {/* Hero Section */}
      <div className={`backdrop-blur-[40px] rounded-[28px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-12 text-center transition-colors ${
        theme === 'dark'
          ? 'bg-gradient-to-br from-white/[0.08] to-white/[0.04] border-white/10'
          : 'bg-gradient-to-br from-white/[0.15] to-white/[0.08] border-white/20'
      }`}>
        <h1 className={`text-[36px] font-bold mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
        }`}>
          Get matched to your next
        </h1>
        <h2 className="text-[42px] font-bold bg-gradient-to-r from-[#c9983a] via-[#a67c2e] to-[#8b7355] bg-clip-text text-transparent mb-6">
          Open source contributions!
        </h2>
        <p className={`text-[16px] mb-8 max-w-2xl mx-auto transition-colors ${
          theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}>
          Get recommendations based on your profile and past contributions.
        </p>
        <button className="px-8 py-4 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[16px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all inline-flex items-center space-x-2 border border-white/10">
          <span>You didn't link your wallet (1/3)</span>
          <ArrowUpRight className="w-5 h-5" />
        </button>
      </div>

      {/* Embark on ODQuest */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-gradient-to-br from-white/[0.1] to-white/[0.06] border-white/15'
          : 'bg-gradient-to-br from-white/[0.18] to-white/[0.12] border-white/25'
      }`}>
        <div className="flex items-center justify-between">
          <div className="flex-1">
            <h3 className={`text-[28px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Embark on an <span className="text-[#c9983a]">ODQuest</span>
            </h3>
            <p className={`text-[16px] mb-6 transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Learn about the ecosystem onboarding quest and track your progress directly on our onboarding Quest
            </p>
            <button className="px-6 py-3 rounded-[14px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.4)] transition-all border border-white/10">
              Let's go
            </button>
          </div>
          <div className="w-24 h-24 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15">
            <Target className="w-12 h-12 text-white" />
          </div>
        </div>
      </div>

      {/* Recommended Projects */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <div className="flex items-center space-x-3 mb-2">
          <Zap className="w-6 h-6 text-[#c9983a] drop-shadow-sm" />
          <h3 className={`text-[24px] font-bold transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>
            Recommended Projects ({projects.length})
          </h3>
        </div>
        <p className={`text-[14px] mb-6 transition-colors ${
          theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}>
          Finding best suited your interests and expertise
        </p>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {projects.map((project) => (
            <div
              key={project.id}
              className={`backdrop-blur-[30px] rounded-[20px] border p-6 transition-all cursor-pointer ${
                theme === 'dark'
                  ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12] hover:shadow-[0_8px_24px_rgba(201,152,58,0.15)]'
                  : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:shadow-[0_8px_24px_rgba(0,0,0,0.08)]'
              }`}
            >
              <div className="flex items-start justify-between mb-4">
                <div className={`w-12 h-12 rounded-[14px] bg-gradient-to-br ${project.color} flex items-center justify-center shadow-md text-2xl`}>
                  {project.icon}
                </div>
                <button className="text-[#c9983a] hover:text-[#a67c2e] transition-colors">
                  <Heart className="w-5 h-5" />
                </button>
              </div>

              <h4 className={`text-[18px] font-bold mb-2 transition-colors ${
                theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>{project.name}</h4>
              <p className={`text-[13px] mb-4 line-clamp-2 transition-colors ${
                theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>{project.description}</p>

              <div className={`flex items-center space-x-4 text-[13px] mb-4 transition-colors ${
                theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>
                <div className="flex items-center space-x-1">
                  <Star className="w-3.5 h-3.5 text-[#c9983a]" />
                  <span>{project.stars}</span>
                </div>
                <div className="flex items-center space-x-1">
                  <GitFork className="w-3.5 h-3.5 text-[#c9983a]" />
                  <span>{project.forks}</span>
                </div>
              </div>

              <div className="flex flex-wrap gap-2">
                {project.tags.map((tag, idx) => (
                  <span
                    key={idx}
                    className={`px-3 py-1.5 rounded-[10px] border text-[12px] font-semibold shadow-[0_2px_8px_rgba(201,152,58,0.15)] ${
                      theme === 'dark'
                        ? 'bg-[#c9983a]/15 border-[#c9983a]/30 text-[#f5c563]'
                        : 'bg-[#c9983a]/20 border-[#c9983a]/35 text-[#8b6f3a]'
                    }`}
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
      {/* Recommended Issues */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/10'
          : 'bg-white/[0.12] border-white/20'
  }`}>
  {/* Section Header */}
  <div className="mb-6">
    <h3
      className={`text-[24px] font-bold transition-colors ${
        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}
    >
      Recommended Issues
    </h3>
    <p
      className={`text-[14px] mt-1 transition-colors ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
      }`}
    >
      Issues that match your interests and expertise
    </p>
  </div>

  {/* Grid */}
  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5 items-stretch">
    {/* Issue Card */}
    <div
      className={`backdrop-blur-[30px] rounded-[16px] border p-5 transition-all h-full min-h-[180px] flex flex-col ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12]'
          : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2]'
      }`}
    >
      {/* Header */}
      <div className="flex items-start justify-between mb-3 gap-3 min-h-[3.5rem]">
        <h4
          className={`text-[16px] font-semibold leading-snug line-clamp-2 ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>
          Add support for new React hooks
        </h4>
        <span
          className={`shrink-0 px-2.5 py-1 rounded-[8px] text-[11px] font-semibold whitespace-nowrap ${
            theme === 'dark'
              ? 'bg-green-500/30 border border-green-500/50 text-green-300'
              : 'bg-green-500/20 border border-green-600/30 text-green-800'
          }`}>
          good first issue
        </span>
      </div>

      {/* Body */}
      

      {/* Footer */}
      <div
        className={`flex items-center justify-between text-[12px] mt-auto ${
          theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}>
        <span className="flex items-center gap-1.5">
          <LanguageIcon language="TypeScript" className="w-3.5 h-3.5" />
          <span>TypeScript</span>
        </span>
        <span>7 days left</span>
      </div>
    </div>

  </div>
</div>


    </div>
  );
}

function OpenSourceWeekPage() {
  const { theme } = useTheme();
  const events = [
    {
      id: 1,
      title: 'Open-Source Week Demo',
      contributors: 320,
      applicants: 'NaN/140',
      projects: 0,
      duration: '7 days',
      startDate: '20 Dec 2025',
      startTime: '10:30AM UTC',
      endDate: '3 Jan 2026',
      endTime: '10:30AM UTC',
      location: 'Worldwide',
      status: 'Upcoming soon',
    },
    {
      id: 2,
      title: 'Open-Source Week Summer Edition',
      contributors: 320,
      applicants: 'NaN/140',
      projects: 0,
      duration: '10 days',
      startDate: '30 Dec 2025',
      startTime: '10:30AM UTC',
      endDate: '8 Jan 2026',
      endTime: '10:30AM UTC',
      location: 'Worldwide',
      status: 'Running soon',
    },
  ];

  const pastEvents = [
    {
      id: 3,
      title: 'Open-Source Week',
      subtitle: 'Spring Edition',
      contributors: 320,
      applicants: 'NaN/140',
      projects: 0,
      startDate: '28 Oct 2025',
      startTime: '10:30AM UTC',
      endDate: '4 Nov 2025',
      endTime: '10:30AM UTC',
    },
    {
      id: 4,
      title: 'Open-Source Week',
      subtitle: 'Winter Edition',
      contributors: 320,
      applicants: 'NaN/140',
      projects: 0,
      startDate: '23 Aug 2025',
      startTime: '10:30AM UTC',
      endDate: '5 Sep 2025',
      endTime: '10:30AM UTC',
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className={`text-[32px] font-bold mb-2 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Open-Source Week</h1>
          <p className={`text-[16px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>
            Gear-round Hack is a week for developers with focus on rewarding.
          </p>
        </div>
        <div className="w-20 h-20 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15">
          <Calendar className="w-10 h-10 text-white" />
        </div>
      </div>

      {/* Main Events */}
      <div className="space-y-5">
        {events.map((event) => (
          <div
            key={event.id}
            className={`backdrop-blur-[40px] rounded-[24px] border p-8 shadow-[0_8px_32px_rgba(0,0,0,0.08)] transition-colors ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/10'
                : 'bg-white/[0.15] border-white/25'
            }`}
          >
            <div className="flex items-start justify-between mb-6">
              <div className="flex items-start space-x-4">
                <div className="w-14 h-14 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-md border border-white/10">
                  <Calendar className="w-7 h-7 text-white" />
                </div>
                <div>
                  <h3 className={`text-[22px] font-bold mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>{event.title}</h3>
                  <span className={`px-3 py-1.5 rounded-[10px] text-[12px] font-semibold ${
                    theme === 'dark'
                      ? 'bg-green-500/30 border border-green-500/50 text-green-300'
                      : 'bg-green-500/20 border border-green-600/30 text-green-800'
                  }`}>
                    {event.status}
                  </span>
                </div>
              </div>
              <button className="px-6 py-3 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[14px] font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.4)] transition-all border border-white/10">
                Join the Open-Source Week
              </button>
            </div>

            <div className="grid grid-cols-4 gap-8 mb-6">
              <div>
                <div className={`text-[12px] mb-2 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Contributors</div>
                <div className={`text-[28px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.contributors}</div>
              </div>
              <div>
                <div className={`text-[12px] mb-2 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Applicants</div>
                <div className={`text-[28px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.applicants}</div>
              </div>
              <div>
                <div className={`text-[12px] mb-2 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Projects</div>
                <div className={`text-[28px] font-bold flex items-center space-x-2 transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  <CircleDot className="w-6 h-6 text-[#c9983a]" />
                  <span>{event.projects}</span>
                </div>
              </div>
              <div>
                <div className={`text-[12px] mb-2 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Left</div>
                <div className={`text-[28px] font-bold flex items-center space-x-2 transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  <Clock className="w-6 h-6 text-[#c9983a]" />
                  <span>{event.duration}</span>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-between pt-6 border-t border-white/10">
              <div className="grid grid-cols-2 gap-8">
                <div>
                  <div className={`text-[12px] mb-1 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Start date</div>
                  <div className={`text-[15px] font-semibold transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>{event.startDate}</div>
                  <div className={`text-[12px] transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>{event.startTime}</div>
                </div>
                <div>
                  <div className={`text-[12px] mb-1 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>End date</div>
                  <div className={`text-[15px] font-semibold transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>{event.endDate}</div>
                  <div className={`text-[12px] transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>{event.endTime}</div>
                </div>
              </div>
              <div>
                <div className={`text-[12px] mb-1 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Location</div>
                <div className={`text-[15px] font-semibold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.location}</div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Additional Events Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
        {pastEvents.map((event) => (
          <div
            key={event.id}
            className={`backdrop-blur-[40px] rounded-[20px] border p-6 shadow-[0_8px_32px_rgba(0,0,0,0.08)] transition-all ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/10 hover:bg-white/[0.12]'
                : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2]'
            }`}
          >
            <div className="flex items-start space-x-3 mb-5">
              <div className="w-12 h-12 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-md border border-white/10">
                <Calendar className="w-6 h-6 text-white" />
              </div>
              <div>
                <h4 className={`text-[18px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.title}</h4>
                <h5 className={`text-[14px] transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>{event.subtitle}</h5>
              </div>
            </div>

            <div className="grid grid-cols-3 gap-4 mb-5">
              <div>
                <div className={`text-[16px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.contributors}</div>
                <div className={`text-[11px] transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Contributors</div>
              </div>
              <div>
                <div className={`text-[16px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.applicants}</div>
                <div className={`text-[11px] transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Applicants</div>
              </div>
              <div>
                <div className={`text-[16px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{event.projects}</div>
                <div className={`text-[11px] transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Projects</div>
              </div>
            </div>

            <div className={`flex items-center justify-between text-[12px] pt-4 border-t border-white/10 transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              <div>
                <div className="mb-0.5">{event.startDate}</div>
                <div className={theme === 'dark' ? 'text-[#b8a898]' : 'text-[#8b7a6a]'}>{event.startTime}</div>
              </div>
              <div className="text-right">
                <div className="mb-0.5">{event.endDate}</div>
                <div className={theme === 'dark' ? 'text-[#b8a898]' : 'text-[#8b7a6a]'}>{event.endTime}</div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function EcosystemsPage() {
  console.log('=== EcosystemsPage FUNCTION CALLED ===');
  console.log('EcosystemsPage component rendered');
  const { theme } = useTheme();
  console.log('Theme:', theme);
  const [showAddModal, setShowAddModal] = useState(false);
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    status: 'active',
    websiteUrl: ''
  });
  const [ecosystems, setEcosystems] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');

  // Fetch ecosystems function
  const fetchEcosystems = async () => {
    console.log('fetchEcosystems function called');
    setIsLoading(true);
    try {
      const { getEcosystems } = await import('../../shared/api/client');
      console.log('Fetching ecosystems from API...');
      const response = await getEcosystems();
      console.log('Ecosystems API response:', response);
      console.log('Response type:', typeof response);
      console.log('Response.ecosystems:', response?.ecosystems);
      console.log('Is array?', Array.isArray(response?.ecosystems));
      
      // Handle different response structures
      let ecosystemsArray: any[] = [];
      
      if (response && Array.isArray(response)) {
        // Response is directly an array
        ecosystemsArray = response;
        console.log('Response is direct array');
      } else if (response && response.ecosystems && Array.isArray(response.ecosystems)) {
        // Response has ecosystems property
        ecosystemsArray = response.ecosystems;
        console.log('Response has ecosystems property');
      } else if (response && typeof response === 'object') {
        // Try to find any array property
        const keys = Object.keys(response);
        console.log('Response keys:', keys);
        for (const key of keys) {
          if (Array.isArray((response as any)[key])) {
            ecosystemsArray = (response as any)[key];
            console.log(`Found array in key: ${key}`);
            break;
          }
        }
      }
      
      if (ecosystemsArray.length === 0) {
        console.warn('No ecosystems found in response:', response);
        setEcosystems([]);
        setIsLoading(false);
        return;
      }
      
      // Transform API response to match UI format
      const transformed = ecosystemsArray.map((eco: any) => {
        const firstLetter = eco.name ? eco.name.charAt(0).toUpperCase() : '?';
        const colors = [
          'from-[#c9983a] to-[#a67c2e]',
          'from-[#8b5cf6] to-[#7c3aed]',
          'from-[#06b6d4] to-[#0891b2]',
          'from-[#10b981] to-[#059669]',
          'from-[#f59e0b] to-[#d97706]',
          'from-[#ef4444] to-[#dc2626]',
        ];
        const colorIndex = eco.name ? eco.name.length % colors.length : 0;
        return {
          id: eco.id,
          name: eco.name || 'Unnamed Ecosystem',
          slug: eco.slug || '',
          description: eco.description || 'No description available.',
          projects: eco.project_count || 0,
          contributors: eco.user_count || 0,
          website_url: eco.website_url || null,
          status: eco.status || 'active',
          letter: firstLetter,
          color: colors[colorIndex],
          languages: [] // Can be populated later if needed
        };
      });
      console.log('Transformed ecosystems:', transformed);
      setEcosystems(transformed);
    } catch (error) {
      console.error('Failed to fetch ecosystems:', error);
      console.error('Error details:', error instanceof Error ? error.message : error);
      setEcosystems([]);
    } finally {
      setIsLoading(false);
    }
  };

  // Fetch ecosystems on mount and when updated
  useEffect(() => {
    console.log('EcosystemsPage useEffect running');
    console.log('Calling fetchEcosystems...');
    fetchEcosystems();
    
    // Listen for ecosystem updates
    const handleUpdate = () => {
      console.log('Ecosystems updated event received');
      fetchEcosystems();
    };
    window.addEventListener('ecosystems-updated', handleUpdate);
    
    return () => {
      window.removeEventListener('ecosystems-updated', handleUpdate);
    };
  }, []);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Handle form submission here
    console.log('Form data:', formData);
    setShowAddModal(false);
    // Reset form
    setFormData({
      name: '',
      description: '',
      status: 'active',
      websiteUrl: ''
    });
  };

  const [showRequestModal, setShowRequestModal] = useState(false);
  const [requestData, setRequestData] = useState({
    userName: '',
    userEmail: '',
    ecosystemName: '',
    reason: '',
    additionalInfo: ''
  });

  const handleRequestSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Handle request submission
    console.log('Request data:', requestData);
    setShowRequestModal(false);
    // Reset form
    setRequestData({
      userName: '',
      userEmail: '',
      ecosystemName: '',
      reason: '',
      additionalInfo: ''
    });
  };

  // Filter ecosystems based on search query
  const filteredEcosystems = ecosystems.filter(eco =>
    eco.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    (eco.description && eco.description.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  return (
    <div className="space-y-6">
      {/* Header Section */}
      <div className={`backdrop-blur-[40px] bg-gradient-to-br rounded-[28px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-10 transition-colors ${
        theme === 'dark'
          ? 'from-white/[0.08] to-white/[0.04] border-white/10'
          : 'from-white/[0.15] to-white/[0.08] border-white/20'
      }`}>
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <h1 className={`text-[36px] font-bold mb-3 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Explore Ecosystems</h1>
            <p className={`text-[16px] max-w-3xl transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Discover a wide range of projects shaping the future of open source, each driving revolutionary change.
            </p>
          </div>
          <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15">
            <Globe className="w-8 h-8 text-white" />
          </div>
        </div>
      </div>

      {/* Search Bar Section */}
      <div className={`backdrop-blur-[40px] rounded-[20px] border shadow-[0_4px_16px_rgba(0,0,0,0.06)] p-5 transition-colors ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <div className="relative">
          <Search className={`absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 z-10 pointer-events-none transition-colors ${
            theme === 'dark' ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
          }`} />
          <input
            type="text"
            placeholder="Search ecosystems..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={`w-full pl-12 pr-4 py-3.5 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] shadow-[inset_0px_0px_4px_0px_rgba(0,0,0,0.12)] relative ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
            }`}
          />
        </div>
      </div>

      {/* Ecosystems Grid */}
      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5 animate-pulse">
          {Array.from({ length: 6 }).map((_, idx) => (
            <div
              key={idx}
              className={`backdrop-blur-[30px] rounded-[20px] border p-6 ${
                theme === 'dark'
                  ? 'bg-white/[0.08] border-white/10'
                  : 'bg-white/[0.15] border-white/25'
              }`}
            >
              <div className="flex items-start justify-between mb-5">
                <div
                  className={`w-14 h-14 rounded-[14px] ${
                    theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                  }`}
                />
              </div>
              <div
                className={`h-5 w-2/3 rounded ${
                  theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                }`}
              />
              <div className="flex items-center gap-6 mt-4 mb-4">
                <div className="flex-1">
                  <div
                    className={`h-3 w-16 rounded ${
                      theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                    }`}
                  />
                  <div
                    className={`h-6 w-10 rounded mt-2 ${
                      theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                    }`}
                  />
                </div>
                <div className="flex-1">
                  <div
                    className={`h-3 w-24 rounded ${
                      theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                    }`}
                  />
                  <div
                    className={`h-6 w-10 rounded mt-2 ${
                      theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                    }`}
                  />
                </div>
              </div>
              <div
                className={`h-3 w-full rounded ${
                  theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                }`}
              />
              <div
                className={`h-3 w-5/6 rounded mt-2 ${
                  theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                }`}
              />
              <div className="mt-4">
                <div
                  className={`h-4 w-28 rounded ${
                    theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                  }`}
                />
              </div>
            </div>
          ))}
        </div>
      ) : filteredEcosystems.length === 0 ? (
        <div className={`text-center py-12 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
          {searchQuery ? 'No ecosystems found matching your search.' : 'No ecosystems available yet.'}
          {!isLoading && ecosystems.length > 0 && (
            <div className="mt-2 text-xs opacity-70">
              (Filtered from {ecosystems.length} ecosystems)
            </div>
          )}
          {!isLoading && ecosystems.length === 0 && (
            <div className="mt-4 text-xs opacity-50">
              Debug: ecosystems array is empty. Check browser console for API errors.
            </div>
          )}
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
          {filteredEcosystems.map((ecosystem) => {
            console.log('Rendering ecosystem:', ecosystem);
            return (
          <div
            key={ecosystem.id}
            className={`backdrop-blur-[30px] rounded-[20px] border p-6 transition-all cursor-pointer group ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/10 hover:bg-white/[0.12] hover:shadow-[0_8px_24px_rgba(201,152,58,0.15)]'
                : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:shadow-[0_8px_24px_rgba(0,0,0,0.08)]'
            }`}
          >
            {/* Header with Icon */}
            <div className="flex items-start justify-between mb-5">
              <div className={`w-14 h-14 rounded-[14px] bg-gradient-to-br ${ecosystem.color} flex items-center justify-center shadow-lg border border-white/20`}>
                <span className="text-white text-[24px] font-bold">{ecosystem.letter}</span>
              </div>
            </div>

            {/* Title */}
            <h3 className={`text-[18px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>{ecosystem.name}</h3>

            {/* Stats */}
            <div className="flex items-center gap-6 mb-4">
              <div>
                <div className={`text-[11px] mb-0.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Projects</div>
                <div className={`text-[20px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{ecosystem.projects}</div>
              </div>
              <div>
                <div className={`text-[11px] mb-0.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Contributors</div>
                <div className={`text-[20px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{ecosystem.contributors}</div>
              </div>
            </div>

            {/* Description */}
            <p className={`text-[13px] mb-5 leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              {ecosystem.description}
            </p>

            {/* Website Link */}
            {ecosystem.website_url && (
              <div className="flex items-center gap-2 mt-4">
                <Globe className={`w-4 h-4 ${theme === 'dark' ? 'text-[#c9983a]' : 'text-[#8b6f3a]'}`} />
                <a
                  href={ecosystem.website_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className={`text-[12px] hover:underline transition-colors ${
                    theme === 'dark' ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
                  }`}
                >
                  Visit Website
                </a>
              </div>
            )}
          </div>
            );
          })}
        </div>

      {/* Request Ecosystem Section */}
      <div className={`backdrop-blur-[40px] bg-gradient-to-br rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-10 transition-all overflow-hidden relative ${
        theme === 'dark'
          ? 'from-white/[0.08] to-white/[0.04] border-white/10'
          : 'from-white/[0.15] to-white/[0.08] border-white/20'
      }`}>
        {/* Decorative gradient circles */}
        <div className="absolute -top-20 -right-20 w-60 h-60 bg-gradient-to-br from-[#c9983a]/20 to-transparent rounded-full blur-3xl"></div>
        <div className="absolute -bottom-20 -left-20 w-60 h-60 bg-gradient-to-br from-[#c9983a]/10 to-transparent rounded-full blur-3xl"></div>
        
        <div className="relative z-10">
          <div className="text-center max-w-2xl mx-auto">
            <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] shadow-[0_8px_24px_rgba(162,121,44,0.4)] mb-6 border border-white/15">
              <Sparkles className="w-10 h-10 text-white" />
            </div>
            
            <h3 className={`text-[28px] font-bold mb-4 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Missing Your Ecosystem?</h3>
            
            <p className={`text-[16px] mb-6 leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Don't see your ecosystem in the list? No worries! Request the admin to add it to our platform.
            </p>
            
            <button
              onClick={() => setShowRequestModal(true)}
              className="group px-8 py-4 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[15px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_10px_30px_rgba(162,121,44,0.5)] transition-all flex items-center gap-3 mx-auto border border-white/10 hover:scale-105"
            >
              <Plus className="w-5 h-5 group-hover:rotate-90 transition-transform" />
              Request Ecosystem Addition
              <ArrowUpRight className="w-4 h-4 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
            </button>
          </div>
        </div>
      </div>

      {/* Add Ecosystem Modal (Admin Only) */}
      {showAddModal && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-[10000]" onClick={() => setShowAddModal(false)}>
          <div 
            className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.12)] p-8 w-[500px] max-w-[90vw] transition-colors ${
              theme === 'dark'
                ? 'bg-white/[0.12] border-white/20'
                : 'bg-white/[0.15] border-white/25'
            }`}
            onClick={(e) => e.stopPropagation()}
          >
            <h3 className={`text-[24px] font-bold mb-6 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Add New Ecosystem</h3>
            
            <form onSubmit={handleSubmit}>
              <div className="space-y-4">
                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Ecosystem Name</label>
                  <input
                    type="text"
                    required
                    value={formData.name}
                    onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="e.g., Web3 Ecosystem"
                  />
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Description</label>
                  <textarea
                    rows={4}
                    required
                    value={formData.description}
                    onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] resize-none ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="Describe the ecosystem..."
                  />
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Status</label>
                  <select
                    value={formData.status}
                    onChange={(e) => setFormData({ ...formData, status: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                  >
                    <option value="active">Active</option>
                    <option value="inactive">Inactive</option>
                  </select>
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Website URL</label>
                  <input
                    type="url"
                    required
                    value={formData.websiteUrl}
                    onChange={(e) => setFormData({ ...formData, websiteUrl: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="https://example.com"
                  />
                </div>
              </div>

              <div className="flex items-center justify-end gap-3 mt-6">
                <button
                  type="button"
                  onClick={() => setShowAddModal(false)}
                  className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[20px] border font-medium text-[14px] transition-all ${
                    theme === 'dark'
                      ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4] hover:bg-white/[0.12]'
                      : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                  }`}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-5 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-medium text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all border border-white/10"
                >
                  Add Ecosystem
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Request Ecosystem Modal */}
      {showRequestModal && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-[10000]" onClick={() => setShowRequestModal(false)}>
          <div 
            className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.12)] p-8 w-[550px] max-w-[90vw] max-h-[90vh] overflow-y-auto transition-colors ${
              theme === 'dark'
                ? 'bg-white/[0.12] border-white/20'
                : 'bg-white/[0.15] border-white/25'
            }`}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-start justify-between mb-6">
              <div className="flex-1">
                <div className="flex items-center gap-3 mb-2">
                  <div className="w-10 h-10 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_6px_20px_rgba(162,121,44,0.3)] border border-white/10">
                    <Sparkles className="w-5 h-5 text-white" />
                  </div>
                  <h3 className={`text-[24px] font-bold transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>Request Ecosystem Addition</h3>
                </div>
                <p className={`text-[14px] transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Fill out the form below and we'll review your request</p>
              </div>
              <button
                onClick={() => setShowRequestModal(false)}
                className={`p-2 rounded-[10px] transition-all hover:scale-110 flex-shrink-0 ml-4 ${
                  theme === 'dark'
                    ? 'hover:bg-white/[0.1] text-[#e8c571] hover:text-[#f5d98a]'
                    : 'hover:bg-black/[0.05] text-[#8b6f3a] hover:text-[#c9983a]'
                }`}
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <form onSubmit={handleRequestSubmit}>
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                      theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>Your Name *</label>
                    <input
                      type="text"
                      required
                      value={requestData.userName}
                      onChange={(e) => setRequestData({ ...requestData, userName: e.target.value })}
                      className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                        theme === 'dark'
                          ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                          : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                      }`}
                      placeholder="John Doe"
                    />
                  </div>

                  <div>
                    <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                      theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>Your Email *</label>
                    <input
                      type="email"
                      required
                      value={requestData.userEmail}
                      onChange={(e) => setRequestData({ ...requestData, userEmail: e.target.value })}
                      className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                        theme === 'dark'
                          ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                          : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                      }`}
                      placeholder="john@example.com"
                    />
                  </div>
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Ecosystem Name *</label>
                  <input
                    type="text"
                    required
                    value={requestData.ecosystemName}
                    onChange={(e) => setRequestData({ ...requestData, ecosystemName: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="e.g., Web3 Ecosystem"
                  />
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Why do you want this ecosystem added? *</label>
                  <textarea
                    rows={4}
                    required
                    value={requestData.reason}
                    onChange={(e) => setRequestData({ ...requestData, reason: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] resize-none ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="Tell us why this ecosystem would be valuable to the community..."
                  />
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Additional Information (Optional)</label>
                  <textarea
                    rows={3}
                    value={requestData.additionalInfo}
                    onChange={(e) => setRequestData({ ...requestData, additionalInfo: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] resize-none ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="Any other details you'd like to share..."
                  />
                </div>
              </div>

              <div className="flex items-center justify-end gap-3 mt-6">
                <button
                  type="button"
                  onClick={() => setShowRequestModal(false)}
                  className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[20px] border font-medium text-[14px] transition-all ${
                    theme === 'dark'
                      ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4] hover:bg-white/[0.12]'
                      : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                  }`}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-5 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-medium text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all border border-white/10 flex items-center justify-center gap-2 hover:scale-[1.02]"
                >
                  <Send className="w-4 h-4 flex-shrink-0" />
                  <span>Submit Request</span>
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

function AdminPage() {
  const { theme } = useTheme();
  const [showAddModal, setShowAddModal] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    status: 'active',
    websiteUrl: ''
  });

  // Add/remove class to body when modal is open to blur sidebar
  useEffect(() => {
    if (showAddModal) {
      document.body.classList.add('modal-open');
    } else {
      document.body.classList.remove('modal-open');
    }
    return () => {
      document.body.classList.remove('modal-open');
    };
  }, [showAddModal]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    
    try {
      const { createEcosystem } = await import('../../shared/api/client');
      await createEcosystem({
        name: formData.name,
        description: formData.description || undefined,
        website_url: formData.websiteUrl || undefined,
        status: formData.status as 'active' | 'inactive',
      });
      
      // Success - close modal and reset form
      setShowAddModal(false);
      setFormData({
        name: '',
        description: '',
        status: 'active',
        websiteUrl: ''
      });
      
      // Refresh ecosystems list by triggering a custom event
      window.dispatchEvent(new CustomEvent('ecosystems-updated'));
      
      // Optionally refresh the page to show new ecosystem
      // window.location.reload();
    } catch (error) {
      console.error('Failed to create ecosystem:', error);
      alert(error instanceof Error ? error.message : 'Failed to create ecosystem. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Admin Header */}
      <div className={`backdrop-blur-[40px] bg-gradient-to-br rounded-[28px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-10 transition-all overflow-hidden relative ${
        theme === 'dark'
          ? 'from-white/[0.08] to-white/[0.04] border-white/10'
          : 'from-white/[0.15] to-white/[0.08] border-white/20'
      }`}>
        {/* Decorative gradient */}
        <div className="absolute -top-20 -right-20 w-80 h-80 bg-gradient-to-br from-[#c9983a]/20 to-transparent rounded-full blur-3xl"></div>
        
        <div className="relative z-10">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] shadow-[0_6px_20px_rgba(162,121,44,0.35)] border border-white/10">
                  <Shield className="w-6 h-6 text-white" />
                </div>
                <h1 className={`text-[36px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>Admin Panel</h1>
              </div>
              <p className={`text-[16px] max-w-3xl transition-colors ${
                theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>
                Manage ecosystems, review requests, and oversee platform operations.
              </p>
            </div>
            <div className="flex items-center gap-3">
              <div className={`px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${
                theme === 'dark'
                  ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]'
                  : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
              }`}>
                <span className="text-[13px] font-medium">Admin Access</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Ecosystem Management Section */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/10'
          : 'bg-white/[0.15] border-white/20'
      }`}>
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className={`text-[24px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Ecosystem Management</h2>
            <p className={`text-[14px] transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>Add, edit, or remove ecosystems from the platform</p>
          </div>
          <button
            onClick={() => setShowAddModal(true)}
            className="group px-6 py-3.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_10px_30px_rgba(162,121,44,0.5)] transition-all flex items-center gap-2.5 border border-white/10 hover:scale-105"
          >
            <Plus className="w-5 h-5 group-hover:rotate-90 transition-transform" />
            Add New Ecosystem
            <Sparkles className="w-4 h-4 group-hover:scale-110 transition-transform" />
          </button>
        </div>

        {/* Info Message */}
        <div className={`backdrop-blur-[30px] rounded-[16px] border p-5 flex items-start gap-4 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.06] border-white/10'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <div className="p-2 rounded-[10px] bg-gradient-to-br from-[#c9983a]/20 to-[#a67c2e]/10 border border-[#c9983a]/20">
            <Sparkles className="w-5 h-5 text-[#c9983a]" />
          </div>
          <div>
            <p className={`text-[14px] font-medium mb-1 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Ecosystem Management Tips</p>
            <p className={`text-[13px] leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Add ecosystems with accurate descriptions and valid website URLs. Review user requests regularly to maintain platform quality.
            </p>
          </div>
        </div>
      </div>

      {/* Add Ecosystem Modal */}
      {showAddModal && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-[10000] animate-in fade-in" onClick={() => setShowAddModal(false)} style={{ backdropFilter: 'blur(8px)' }}>
          <div 
            className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.12)] p-8 w-[550px] max-w-[90vw] transition-all animate-in zoom-in-95 ${
              theme === 'dark'
                ? 'bg-white/[0.12] border-white/20'
                : 'bg-white/[0.15] border-white/25'
            }`}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-start justify-between mb-6">
              <div>
                <h3 className={`text-[24px] font-bold mb-2 transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>Add New Ecosystem</h3>
                <p className={`text-[14px] transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Create a new ecosystem entry for the platform</p>
              </div>
              <div className="w-12 h-12 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_6px_20px_rgba(162,121,44,0.3)] border border-white/10">
                <Globe className="w-6 h-6 text-white" />
              </div>
            </div>
            
            <form onSubmit={handleSubmit}>
              <div className="space-y-4">
                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Ecosystem Name *</label>
                  <input
                    type="text"
                    required
                    value={formData.name}
                    onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="e.g., Web3 Ecosystem"
                  />
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Description *</label>
                  <textarea
                    rows={4}
                    required
                    value={formData.description}
                    onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] resize-none ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="Describe the ecosystem..."
                  />
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Status *</label>
                  <select
                    value={formData.status}
                    onChange={(e) => setFormData({ ...formData, status: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                  >
                    <option value="active">Active</option>
                    <option value="inactive">Inactive</option>
                  </select>
                </div>

                <div>
                  <label className={`block text-[13px] font-medium mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>Website URL *</label>
                  <input
                    type="url"
                    required
                    value={formData.websiteUrl}
                    onChange={(e) => setFormData({ ...formData, websiteUrl: e.target.value })}
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
                        : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
                    }`}
                    placeholder="https://example.com"
                  />
                </div>
              </div>

              <div className="flex items-center justify-end gap-3 mt-6">
                <button
                  type="button"
                  onClick={() => setShowAddModal(false)}
                  className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[20px] border font-medium text-[14px] transition-all ${
                    theme === 'dark'
                      ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4] hover:bg-white/[0.12]'
                      : 'bg-white/[0.15] border-white/25 text-[#7a6b5a] hover:bg-white/[0.2]'
                  }`}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="px-5 py-2.5 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-medium text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_8px_24px_rgba(162,121,44,0.5)] transition-all border border-white/10 flex items-center gap-2"
                >
                  <Plus className="w-4 h-4" />
                  Add Ecosystem
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}