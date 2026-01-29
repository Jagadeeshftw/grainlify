import { Github } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function BlogHowItWorks() {
  const { theme } = useTheme();

  const contributorSteps = [
    'Connect your GitHub account',
    'Browse projects that match your skills',
    'Start contributing to issues and features',
    'Earn rewards and build your reputation',
    'Climb the leaderboard and unlock opportunities',
  ];

  const maintainerSteps = [
    'Submit your project to OnlyGrain',
    'Set up bounties and contribution guidelines',
    'Get matched with skilled developers',
    'Review contributions and approve rewards',
    'Scale your project with community support',
  ];

  return (
    <div className="mb-6 md:mb-8 p-4 md:p-5 lg:p-6 backdrop-blur-[30px] bg-gradient-to-br from-[#c9983a]/10 to-[#d4af37]/5 rounded-[16px] md:rounded-[20px] border-2 border-[#c9983a]/30">
      <h3 className={`text-lg md:text-2xl lg:text-[28px] font-bold mb-3 md:mb-4 flex items-center gap-2 md:gap-3 transition-colors ${
        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}>
        <Github className="w-5 h-5 md:w-6 md:h-6 lg:w-7 lg:h-7 text-[#c9983a] flex-shrink-0" />
        How It Works
      </h3>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 md:gap-4 lg:gap-6">
        {/* Contributors */}
        <div className="backdrop-blur-[20px] bg-white/[0.25] rounded-[12px] md:rounded-[16px] border border-white/30 p-3 md:p-4 lg:p-5">
          <h4 className={`text-base md:text-lg lg:text-[18px] font-bold mb-2 md:mb-3 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>For Contributors</h4>
          <ol className={`space-y-1.5 md:space-y-2 text-xs md:text-sm lg:text-[14px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
          }`}>
            {contributorSteps.map((step, index) => (
              <li key={index} className="flex gap-1.5 md:gap-2">
                <span className="font-bold text-[#c9983a] flex-shrink-0">{index + 1}.</span>
                <span className="break-words">{step}</span>
              </li>
            ))}
          </ol>
        </div>

        {/* Maintainers */}
        <div className="backdrop-blur-[20px] bg-white/[0.25] rounded-[12px] md:rounded-[16px] border border-white/30 p-3 md:p-4 lg:p-5">
          <h4 className={`text-base md:text-lg lg:text-[18px] font-bold mb-2 md:mb-3 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>For Project Maintainers</h4>
          <ol className={`space-y-1.5 md:space-y-2 text-xs md:text-sm lg:text-[14px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
          }`}>
            {maintainerSteps.map((step, index) => (
              <li key={index} className="flex gap-1.5 md:gap-2">
                <span className="font-bold text-[#c9983a] flex-shrink-0">{index + 1}.</span>
                <span className="break-words">{step}</span>
              </li>
            ))}
          </ol>
        </div>
      </div>
    </div>
  );
}
