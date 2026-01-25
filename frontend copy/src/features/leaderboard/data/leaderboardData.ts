import { LeaderData, ProjectData } from '../types';

export const leaderboardData: LeaderData[] = [
  { rank: 1, username: 'octocat', avatar: '🐙', score: 1550, trend: 'same', trendValue: 0, contributions: 245, ecosystems: ['Web3', 'AI'] },
  { rank: 2, username: 'developer', avatar: '👨‍💻', score: 1340, trend: 'down', trendValue: -1, contributions: 198, ecosystems: ['Blockchain'] },
  { rank: 3, username: 'contributor1', avatar: '🌟', score: 1150, trend: 'up', trendValue: 2, contributions: 176, ecosystems: ['Developer Tools'] },
  { rank: 4, username: 'reviewer', avatar: '👀', score: 1030, trend: 'down', trendValue: -2, contributions: 152, ecosystems: ['Cloud-Native'] },
  { rank: 5, username: 'maintainer', avatar: '🛠️', score: 910, trend: 'up', trendValue: 1, contributions: 134, ecosystems: ['Security'] },
  { rank: 6, username: 'codemaster', avatar: '💻', score: 875, trend: 'same', trendValue: 0, contributions: 128, ecosystems: ['Mobile'] },
  { rank: 7, username: 'opensourcefan', avatar: '❤️', score: 820, trend: 'up', trendValue: 3, contributions: 115, ecosystems: ['Data Science'] },
  { rank: 8, username: 'gitexpert', avatar: '🚀', score: 795, trend: 'down', trendValue: -1, contributions: 108, ecosystems: ['Gaming'] },
  { rank: 9, username: 'techguru', avatar: '🎯', score: 760, trend: 'up', trendValue: 2, contributions: 98, ecosystems: ['Web3'] },
  { rank: 10, username: 'devninja', avatar: '🥷', score: 720, trend: 'same', trendValue: 0, contributions: 92, ecosystems: ['AI'] },
];

export const projectsData: ProjectData[] = [
  { rank: 1, name: 'DeFi Protocol', logo: '🌐', score: 8950, trend: 'up', trendValue: 2, contributors: 342, ecosystems: ['Web3', 'Blockchain'], activity: 'Very High' },
  { rank: 2, name: 'AI Framework', logo: '🤖', score: 7840, trend: 'same', trendValue: 0, contributors: 289, ecosystems: ['AI'], activity: 'High' },
  { rank: 3, name: 'Cloud Tools', logo: '☁️', score: 6720, trend: 'up', trendValue: 1, contributors: 256, ecosystems: ['Cloud-Native'], activity: 'High' },
  { rank: 4, name: 'Security Suite', logo: '🔒', score: 5980, trend: 'down', trendValue: -1, contributors: 198, ecosystems: ['Security'], activity: 'Medium' },
  { rank: 5, name: 'Mobile SDK', logo: '📱', score: 5430, trend: 'up', trendValue: 3, contributors: 176, ecosystems: ['Mobile'], activity: 'Medium' },
  { rank: 6, name: 'Data Pipeline', logo: '📊', score: 4920, trend: 'same', trendValue: 0, contributors: 154, ecosystems: ['Data Science'], activity: 'Medium' },
  { rank: 7, name: 'Game Engine', logo: '🎮', score: 4560, trend: 'up', trendValue: 2, contributors: 142, ecosystems: ['Gaming'], activity: 'Low' },
  { rank: 8, name: 'Dev Toolkit', logo: '🔧', score: 4210, trend: 'down', trendValue: -2, contributors: 128, ecosystems: ['Developer Tools'], activity: 'Low' },
  { rank: 9, name: 'Web3 Wallet', logo: '💼', score: 3980, trend: 'up', trendValue: 1, contributors: 115, ecosystems: ['Web3'], activity: 'Low' },
  { rank: 10, name: 'Blockchain Explorer', logo: '🔍', score: 3650, trend: 'same', trendValue: 0, contributors: 98, ecosystems: ['Blockchain'], activity: 'Low' },
];

export const getAvatarGradient = (index: number): string => {
  const gradients = [
    'from-[#c9983a] to-[#a67c2e]',
    'from-[#d4af37] to-[#b8932e]',
    'from-[#b89968] to-[#9a7d4f]',
    'from-[#c9983a]/90 to-[#8b6f3a]',
    'from-[#d4c4b0] to-[#a89780]',
    'from-[#c4b5a0] to-[#9a8270]',
    'from-[#b8a590] to-[#8b7355]',
    'from-[#c9983a]/80 to-[#a67c2e]/80',
    'from-[#d4af37]/90 to-[#c9983a]/70',
    'from-[#b89968]/85 to-[#8b6f3a]/75',
  ];
  return gradients[index % gradients.length];
};
