import { useNavigate } from 'react-router-dom';
import { User, Settings, LogOut, ChevronDown, LogIn } from 'lucide-react';
import { useAuth } from '../contexts/AuthContext';
import { useTheme } from '../contexts/ThemeContext';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../../app/components/ui/dropdown-menu';

interface UserProfileDropdownProps {
  onPageChange?: (page: string) => void;
  showMobileNav:boolean
}

export function UserProfileDropdown({ onPageChange,showMobileNav }: UserProfileDropdownProps) {
  const { user, userRole, logout } = useAuth();
  const { theme } = useTheme();
  const navigate = useNavigate();
  const darkTheme = theme === 'dark';

  const handleProfileClick = () => {
    if (onPageChange) {
      onPageChange('profile');
    } else {
      navigate('/dashboard?page=profile');
    }
  };

  const handleSettingsClick = () => {
    if (onPageChange) {
      onPageChange('settings');
    } else {
      navigate('/dashboard?page=settings');
    }
  };

  const handleSignIn = () => {
    navigate('/signin');
  };

  const handleLogout = () => {
    logout();
    navigate('/');
  };

  // Show login button when not authenticated
  if (!user) {
    return (
      <button
        onClick={handleSignIn}
        className={`h-[46px] px-6 rounded-[999px] overflow-clip relative  items-center gap-2 bg-gradient-to-br transition-all hover:scale-105 shadow-[0px_6px_6.5px_-1px_rgba(0,0,0,0.36),0px_0px_4.2px_0px_rgba(0,0,0,0.69)] mr-[3px] ${
          darkTheme ? 'from-[#c9983a] to-[#a67c2e]' : 'from-[#e8c571] to-[#c9983a]'
        }
        ${showMobileNav? 'flex':'hidden lg:flex'} `}
      >
        <div className="absolute inset-0 pointer-events-none shadow-[0_0_20px_rgba(201,152,58,0.5),inset_0_1px_1px_rgba(255,255,255,0.25)]" />
        <LogIn className="w-4 h-4 relative z-10 text-white" />
        <span className="block text-[13px] font-medium relative z-10 text-white tracking-wide text-shadow-[0px_1px_2px_rgba(0,0,0,0.3)]">
          Sign In
        </span>
      </button>
    );
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className={`h-[46px] px-4 rounded-[999px] overflow-clip relative flex items-center gap-2 backdrop-blur-[40px] transition-all hover:scale-105 shadow-[0px_6px_6.5px_-1px_rgba(0,0,0,0.36),0px_0px_4.2px_0px_rgba(0,0,0,0.69)] mr-[3px] ${
            darkTheme ? 'bg-[#2d2820]' : 'bg-[#d4c5b0]'
          }
          ${showMobileNav ? " flex " : " hidden lg:flex "}
          `}
        >
          <div className={`absolute inset-0 pointer-events-none rounded-full ${
            darkTheme
              ? 'shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.5),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.11)]'
              : 'shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.15),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.35)]'
          }`} />
          <img
            src={user.github.avatar_url}
            alt={user.github.login}
            className="w-7 h-7 rounded-full border-2 border-[#c9983a] relative z-10 flex-shrink-0"
          />
          <div className="flex flex-col items-start relative z-10 min-w-0">
            <span className={`text-[13px] font-medium leading-tight truncate max-w-[120px] ${ 
              darkTheme ? 'text-[rgba(255,255,255,0.69)]' : 'text-[rgba(45,40,32,0.75)]'
            }`}>
              {user.github.login}
            </span>
            <span className="text-[11px] text-[#c9983a] capitalize leading-tight font-medium">
              {userRole}
            </span>
          </div>
          <ChevronDown className={`w-3.5 h-3.5 transition-transform relative z-10 flex-shrink-0 ${ 
            darkTheme ? 'text-[rgba(255,255,255,0.69)]' : 'text-[rgba(45,40,32,0.75)]'
          }`} />
        </button>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        align="end"
        sideOffset={8}
        className={`w-64 rounded-[18px] backdrop-blur-[40px] border shadow-[0_8px_32px_rgba(0,0,0,0.12),0_0_20px_rgba(201,152,58,0.15)] overflow-hidden p-0 ${
          darkTheme
            ? 'bg-white/[0.08] border-white/15'
            : 'bg-white/[0.15] border-white/25'
        }`}
      >
        {/* User Info Section */}
        <DropdownMenuLabel className={`px-4 py-4 border-b ${
          darkTheme ? 'border-white/10' : 'border-white/20'
        }`}>
          <div className="flex items-center space-x-3">
            <img
              src={user.github.avatar_url}
              alt={user.github.login}
              className="w-12 h-12 rounded-full border-2 border-[#c9983a] shadow-[0_0_12px_rgba(201,152,58,0.4)]"
            />
            <div className="flex-1">
              <p className={`font-semibold text-sm ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}>
                {user.github.login}
              </p>
              <p className="text-xs text-[#c9983a] capitalize font-medium">
                {userRole}
              </p>
              <p className={`text-xs truncate ${
                darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>
                ID: {user.id.substring(0, 8)}...
              </p>
            </div>
          </div>
        </DropdownMenuLabel>

        {/* Menu Items */}
        <DropdownMenuItem
          onClick={handleProfileClick}
          className={`flex items-center space-x-3 px-4 py-3 cursor-pointer ${
            darkTheme
              ? 'hover:bg-white/[0.12] text-[#d4c5b0] focus:bg-white/[0.12] focus:text-[#e8dfd0]'
              : 'hover:bg-white/[0.2] text-[#2d2820] focus:bg-white/[0.2]'
          }`}
        >
          <User className="w-4 h-4 text-[#c9983a]" />
          <span className="text-sm font-medium">Public Profile</span>
        </DropdownMenuItem>

        <DropdownMenuItem
          onClick={handleSettingsClick}
          className={`flex items-center space-x-3 px-4 py-3 cursor-pointer ${
            darkTheme
              ? 'hover:bg-white/[0.12] text-[#d4c5b0] focus:bg-white/[0.12] focus:text-[#e8dfd0]'
              : 'hover:bg-white/[0.2] text-[#2d2820] focus:bg-white/[0.2]'
          }`}
        >
          <Settings className="w-4 h-4 text-[#c9983a]" />
          <span className="text-sm font-medium">Settings</span>
        </DropdownMenuItem>

        <DropdownMenuSeparator className={darkTheme ? 'bg-white/10' : 'bg-white/20'} />

        <DropdownMenuItem
          onClick={handleLogout}
          variant="destructive"
          className={`flex items-center space-x-3 px-4 py-3 cursor-pointer ${
            darkTheme
              ? 'hover:bg-red-500/10 text-red-400 focus:bg-red-500/10'
              : 'hover:bg-red-50 text-red-600 focus:bg-red-50'
          }`}
        >
          <LogOut className="w-4 h-4" />
          <span className="text-sm font-medium">Logout</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}