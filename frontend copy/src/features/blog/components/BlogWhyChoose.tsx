import { Zap, Globe, Coins, Users } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function BlogWhyChoose() {
  const { theme } = useTheme();

  const cards = [
    {
      icon: <Zap className="w-6 h-6 text-white" />,
      title: 'Lightning Fast Matching',
      description: 'Our AI-powered algorithm instantly connects you with the most relevant opportunities based on your profile and preferences.',
    },
    {
      icon: <Globe className="w-6 h-6 text-white" />,
      title: 'All Chains, One Platform',
      description: 'Access projects from every major blockchain ecosystem without switching platforms or creating multiple accounts.',
    },
    {
      icon: <Coins className="w-6 h-6 text-white" />,
      title: 'Fair Compensation',
      description: 'Transparent reward systems ensure contributors are fairly compensated for their work with competitive bounties and grants.',
    },
    {
      icon: <Users className="w-6 h-6 text-white" />,
      title: 'Vibrant Community',
      description: 'Join thousands of developers and projects building the future of Web3 together in a supportive ecosystem.',
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
      {cards.map((card, index) => (
        <div key={index} className="backdrop-blur-[20px] bg-white/[0.15] rounded-[16px] border border-white/25 p-5">
          <div className="w-12 h-12 mb-3 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-md">
            {card.icon}
          </div>
          <h4 className={`text-[18px] font-bold mb-2 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>{card.title}</h4>
          <p className={`text-[14px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#6b5d4d]'
          }`}>
            {card.description}
          </p>
        </div>
      ))}
    </div>
  );
}
