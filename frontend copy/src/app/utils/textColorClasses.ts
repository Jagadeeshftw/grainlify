// Utility for consistent text color classes across light and dark modes
export const getTextColorClasses = (theme: 'light' | 'dark', type: 'primary' | 'secondary' | 'tertiary' = 'primary') => {
  const colorMap = {
    primary: theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]',
    secondary: theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]',
    tertiary: theme === 'dark' ? 'text-[#b8a898]' : 'text-[#8b7a6a]',
  };
  
  return colorMap[type];
};

export const getBackgroundColorClasses = (theme: 'light' | 'dark', type: 'card' | 'input' | 'modal' = 'card') => {
  const colorMap = {
    card: theme === 'dark' 
      ? 'bg-white/[0.08] border-white/10' 
      : 'bg-white/[0.15] border-white/25',
    input: theme === 'dark'
      ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4]'
      : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]',
    modal: theme === 'dark'
      ? 'bg-[#1a1410]/98 border-white/10'
      : 'bg-[#e5ddd1]/95 border-white/30',
  };
  
  return colorMap[type];
};
