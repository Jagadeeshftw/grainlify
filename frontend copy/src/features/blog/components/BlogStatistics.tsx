import { Users, Code, Globe } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { useLandingStats } from '../../../shared/hooks/useLandingStats';

export function BlogStatistics() {
  const { theme } = useTheme();
  const { display } = useLandingStats();

  const stats = [
    { icon: <Users className="w-6 h-6 text-white" />, value: display.contributors, label: 'Active Contributors' },
    { icon: <Code className="w-6 h-6 text-white" />, value: display.activeProjects, label: 'Active Projects' },
    { icon: <Globe className="w-6 h-6 text-white" />, value: '20+', label: 'Blockchain Ecosystems' },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-6">
      {stats.map((stat, index) => (
        <div key={index} className="text-center p-4 backdrop-blur-[20px] bg-white/[0.12] rounded-[16px] border border-white/20">
          <div className="w-12 h-12 mx-auto mb-3 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-md">
            {stat.icon}
          </div>
          <div className={`text-[24px] font-bold mb-1 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>{stat.value}</div>
          <div className={`text-[12px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>{stat.label}</div>
        </div>
      ))}
    </div>
  );
}
