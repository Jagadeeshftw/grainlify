import { Bell } from "lucide-react";
import { useTheme } from "../contexts/ThemeContext";
import { useState, useEffect } from "react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "../../app/components/ui/dropdown-menu";

interface NotificationsDropdownProp {
     showMobileNav: boolean; 
     closeMobileNav:() => void; 
}
export function NotificationsDropdown({showMobileNav}:NotificationsDropdownProp) {
  const { theme } = useTheme();
  const darkTheme = theme === "dark";
  const [notificationCount, setNotificationCount] = useState(0);

  // Use real count when notifications API is available; until then show 0 (badge hidden)
  useEffect(() => {
    const fetchNotificationCount = async () => {
      try {
        // When backend provides GET /notifications/count or similar, wire it here:
        // const data = await getNotificationCount();
        // setNotificationCount(data.count ?? 0);
        setNotificationCount(0);
      } catch (error) {
        console.error("Failed to fetch notification count:", error);
        setNotificationCount(0);
      }
    };

    fetchNotificationCount();
  }, []);

  // Format count for display (99+ for counts over 99)
  const formatCount = (count: number): string => {
    return count > 99 ? "99+" : count.toString();
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className={`h-[46px] w-[46px] rounded-full relative items-center justify-center backdrop-blur-[40px] transition-all hover:scale-105 shadow-[0px_6px_6.5px_-1px_rgba(0,0,0,0.36),0px_0px_4.2px_0px_rgba(0,0,0,0.69)] ${
            darkTheme ? "bg-[#2d2820] " : "bg-[#d4c5b0] "
          }
          ${showMobileNav ? "flex w-[80%] max-w-[800px] rounded-sm" : "hidden lg:flex"}`}
        >
          <div
            className={`absolute inset-0 pointer-events-none ${showMobileNav? 'rounded-sm': 'rounded-full'} ${
              darkTheme
                ? "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.5),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.11)]"
                : "shadow-[inset_1px_-1px_1px_0px_rgba(0,0,0,0.15),inset_-2px_2px_1px_-1px_rgba(255,255,255,0.35)]"
            }`}
          />
          <Bell
            className={`w-4 h-4 relative z-10 transition-colors ${
              darkTheme
                ? "text-[rgba(255,255,255,0.69)]"
                : "text-[rgba(45,40,32,0.75)]"
            }`}
          />
  {
          showMobileNav && <span className={`ml-2 ${darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'}`}>
          Notification
          </span>
               }
          {/* Notification Count Badge - Only show when count > 0; high-contrast for visibility */}
          {notificationCount > 0 && (
            <div
              className={`absolute -top-0.5 -right-0.5 lg:-top-1 lg:-right-1 min-w-[18px] h-[18px] px-1 rounded-full z-20 border-2 flex items-center justify-center ${
                darkTheme
                  ? "bg-[#2d2820] border-[#c9983a] text-[#fef5e7] shadow-[0_2px_8px_rgba(0,0,0,0.4)]"
                  : "bg-[#2d2820] border-white/90 text-white shadow-[0_2px_8px_rgba(0,0,0,0.25)]"
              }`}
            >
              <span className="text-[10px] font-bold leading-none tabular-nums">
                {formatCount(notificationCount)}
              </span>
            </div>
          )}
        </button>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        align="end"
        sideOffset={8}
        className={`w-80 rounded-[18px] backdrop-blur-[40px] border shadow-[0_8px_32px_rgba(0,0,0,0.12),0_0_20px_rgba(201,152,58,0.15)] overflow-hidden p-0 ${
          darkTheme
            ? "bg-white/[0.08] border-white/15"
            : "bg-white/[0.15] border-white/25"
        }`}
      >
        {/* Header */}
        <DropdownMenuLabel
          className={`px-4 py-4 border-b ${
            darkTheme ? "border-white/10" : "border-white/20"
          }`}
        >
          <div className="flex items-center space-x-3">
            <div
              className={`w-10 h-10 rounded-full flex items-center justify-center ${
                darkTheme ? "bg-white/[0.12]" : "bg-white/[0.2]"
              }`}
            >
              <Bell
                className={`w-5 h-5 ${darkTheme ? "text-[#c9983a]" : "text-[#c9983a]"}`}
              />
            </div>
            <div className="flex-1">
              <p
                className={`font-semibold text-sm ${
                  darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"
                }`}
              >
                Notifications
              </p>
            </div>
          </div>
        </DropdownMenuLabel>

        {/* Empty State */}
        <div
          className={`px-4 py-12 flex flex-col items-center justify-center ${
            darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"
          }`}
        >
          <div
            className={`w-16 h-16 rounded-full flex items-center justify-center mb-4 ${
              darkTheme ? "bg-white/[0.08]" : "bg-white/[0.15]"
            }`}
          >
            <Bell
              className={`w-8 h-8 ${darkTheme ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}
            />
          </div>
          <p
            className={`text-sm font-medium mb-1 ${
              darkTheme ? "text-[#e8dfd0]" : "text-[#2d2820]"
            }`}
          >
            No notifications yet
          </p>
          <p className="text-xs text-center max-w-[200px]">
            You'll see updates about your contributions, rewards, and project
            activity here.
          </p>
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
