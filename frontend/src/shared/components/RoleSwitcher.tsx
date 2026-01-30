import { Shield, Users, Code, Lock } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';
import { useEffect , useState} from 'react';

interface RoleSwitcherProps {
  currentRole: 'contributor' | 'maintainer' | 'admin';
  onRoleChange: (role: 'contributor' | 'maintainer' | 'admin') => void;
  showMobileNav: boolean;
  isSmallDevice: boolean;
  closeMobileNav:()=>void
}

export function RoleSwitcher({ currentRole, onRoleChange , showMobileNav , isSmallDevice, closeMobileNav}: RoleSwitcherProps) {
  const { theme } = useTheme();

  const roles = [
    {
      id: 'contributor' as const,
      label: 'CONTRIBUTOR',
      icon: Code,
    },
    {
      id: 'maintainer' as const,
      label: 'MAINTAINER',
      icon: Users,
    },
    {
      id: 'admin' as const,
      label: 'ADMIN',
      icon: Shield,
    },
  ];

  return (
    <div 
      className={`w-[80%] lg:w-auto max-w-[800px] lg:max-w-none flex-col lg:flex-row gap-[8px] lg:gap-[2px] lg:h-[44px] items-start p-[2px] rounded-[10px] lg:rounded-[999px] shadow-[0px_6px_6.5px_-1px_rgba(0,0,0,0.36),0px_0px_4.2px_0px_rgba(0,0,0,0.69)] ${
        theme === 'dark'
          ? 'bg-[#1a1612]'
          : 'bg-[#8b7d6b]'
      }
      ${showMobileNav? 'inline-flex' : 'hidden lg:inline-flex'}
      `}
    >
      {roles.map((role, index) => {
        const Icon = role.icon;
        const isActive = currentRole === role.id;
        const isFirst = index === 0;
        const isLast = index === roles.length - 1;
        
        return (
          <button
            key={role.id}
            onClick={() => {onRoleChange(role.id); closeMobileNav();}}
            className={`h-[40px] relative shrink-0 w-full lg:w-[130px] px-3 ${
              isFirst ? 'rounded-bl-[10px] lg:rounded-bl-[20px] rounded-br-[4px] rounded-tl-[10px] lg:rounded-tl-[20px] rounded-tr-[4px] ' :
              isLast ? 'rounded-bl-[4px] rounded-br-[10px] lg:rounded-br-[20px] rounded-tl-[4px] rounded-tr-[10px] lg:rounded-tr-[20px]' :
              'rounded-[4px]'
            } ${
              isActive
                ? theme === 'dark'
                  ? 'bg-gradient-to-br from-[#c9983a] to-[#a67c2e]'
                  : 'bg-gradient-to-br from-[#e8c571] to-[#c9983a]'
                : theme === 'dark'
                ? 'bg-[#2d2820]'
                : 'bg-[#d4c5b0]'
            }`}
          >
            {/* Button Content */}
            <div className={`absolute flex items-center justify-center gap-2 left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 whitespace-nowrap ${
              isActive
                ? 'text-white'
                : theme === 'dark'
                ? 'text-[rgba(255,255,255,0.69)]'
                : 'text-[rgba(45,40,32,0.75)]'
            }`}>
              <Icon className={`w-3.5 h-3.5 sm:w-4 sm:h-4 ${
                isActive ? '' : 'opacity-80'
              }`} />
              
              <span className={`text-[11px] font-medium leading-[0] tracking-wide ${
                isActive ? 'text-shadow-[0px_1px_2px_rgba(0,0,0,0.3)]' : 'text-shadow-[0px_1px_0px_rgba(0,0,0,0.19)]'
              }`}>
                {role.label}
              </span>
            </div>

            {/* Lock Badge for Admin - Restricted Access Indicator */}
            {role.id === 'admin' && (
              <div className="absolute -top-2 -right-2 w-5 h-5 bg-gradient-to-br from-[#e8c571] to-[#c9983a] rounded-full shadow-[0_2px_8px_rgba(201,152,58,0.9),0_0_12px_rgba(201,152,58,0.7)] z-20 flex items-center justify-center border-[2px] border-white">
                <Lock className="w-2.5 h-2.5 text-white drop-shadow-[0_1px_3px_rgba(0,0,0,0.6)]" strokeWidth={3} />
              </div>
            )}
          </button>
        );
      })}
    </div>
  );
}