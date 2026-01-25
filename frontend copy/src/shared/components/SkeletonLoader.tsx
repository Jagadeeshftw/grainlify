import { useTheme } from '../contexts/ThemeContext';

interface SkeletonLoaderProps {
  className?: string;
  variant?: 'default' | 'circle' | 'text';
  width?: string;
  height?: string;
}

export function SkeletonLoader({ className, variant = 'default', width, height }: SkeletonLoaderProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const baseClasses = `relative overflow-hidden ${
    variant === 'circle' 
      ? 'rounded-full' 
      : variant === 'text'
      ? 'rounded-[100px]'
      : 'rounded-[12px]'
  }`;

  const bgColor = isDark 
    ? 'bg-white/[0.08]' 
    : 'bg-white/[0.12]';

  const shimmerGradient = isDark
    ? 'from-transparent via-white/[0.15] to-transparent'
    : 'from-transparent via-white/[0.25] to-transparent';

  const style: React.CSSProperties = {};
  if (width) style.width = width;
  if (height) style.height = height;

  return (
    <div 
      className={`${baseClasses} ${bgColor} ${className || ''}`}
      style={style}
    >
      <div 
        className={`absolute inset-0 -translate-x-full animate-shimmer bg-gradient-to-r ${shimmerGradient}`}
      />
    </div>
  );
}
