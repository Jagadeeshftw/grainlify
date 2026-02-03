import { ReactNode } from 'react';
import { GitBranch, Users, Circle } from 'lucide-react';
import { useTheme } from '../../contexts/ThemeContext';
import { LanguageIcon } from '../LanguageIcon';

export interface IssueCardProps {
  id: string;
  number?: string;
  title: string;
  repository?: string;
  applicants?: number;
  author?: {
    name: string;
    avatar: string;
  };
  timeAgo?: string;
  tags?: string[];
  isSelected?: boolean;
  onClick?: () => void;
  icon?: ReactNode;
  showTags?: boolean;
  // New props for recommended issues format
  description?: string;
  language?: string;
  daysLeft?: string;
  variant?: 'default' | 'recommended';
  primaryTag?: string; // For the main tag (e.g., "good first issue", "bug")
}

export function IssueCard({
  id,
  number,
  title,
  repository,
  applicants,
  author,
  timeAgo,
  tags = [],
  isSelected = false,
  onClick,
  icon,
  showTags = false,
  description,
  language,
  daysLeft,
  variant = 'default',
  primaryTag,
}: IssueCardProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  // Recommended Issue Variant
  if (variant === 'recommended') {
    return (
      <div 
        onClick={onClick}
        className={`backdrop-blur-[30px] rounded-[16px] border p-5 transition-all cursor-pointer ${
          isDark
            ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12]'
            : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2]'
        }`}
      >
        <div className="flex items-start justify-between mb-2">
          <h4 className={`text-[16px] font-semibold leading-6 min-h-[3rem] line-clamp-2 transition-colors ${
            isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>{title}</h4>
          {primaryTag && (
            <span className={`px-2.5 py-1 rounded-[8px] text-[11px] font-semibold whitespace-nowrap ml-2 ${
              primaryTag === 'good first issue'
                ? isDark
                  ? 'bg-green-500/30 border border-green-500/50 text-green-300'
                  : 'bg-green-500/20 border border-green-600/30 text-green-800'
                : primaryTag === 'bug'
                  ? isDark
                    ? 'bg-red-500/30 border border-red-500/50 text-red-300'
                    : 'bg-red-500/20 border border-red-600/30 text-red-800'
                  : isDark
                    ? 'bg-[#c9983a]/30 border border-[#c9983a]/50 text-[#e8c571]'
                    : 'bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#8b6f3a]'
            }`}>
              {primaryTag}
            </span>
          )}
        </div>
        {description && (
          <p className={`text-[13px] mb-3 line-clamp-2 transition-colors ${
            isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>
            {description}
          </p>
        )}
        <div className={`flex items-center space-x-3 text-[12px] transition-colors ${
          isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}>
          {language && (
            <span className="flex items-center space-x-1.5">
              <LanguageIcon language={language} className="w-3.5 h-3.5" />
              <span>{language}</span>
            </span>
          )}
          {daysLeft && <span>{daysLeft}</span>}
        </div>
      </div>
    );
  }

  // Default Issue Card Variant
  return (
    <button
      onClick={onClick}
      className={`w-full p-3 rounded-[16px] backdrop-blur-[40px] border transition-all text-left ${
        isSelected
          ? isDark
            ? 'border-[#c9983a] bg-[#c9983a]/10'
            : 'border-2 border-[#c9983a] bg-gradient-to-br from-[#c9983a]/20 to-[#d4af37]/15 shadow-sm hover:from-[#c9983a]/25 hover:to-[#d4af37]/20'
          : isDark
            ? 'bg-white/[0.12] border-white/20 hover:bg-white/[0.15]'
            : 'bg-white/[0.12] border-white/20 hover:bg-white/[0.15]'
      }`}
    >
      {/* Issue Header */}
      <div className="flex items-start justify-between mb-2">
        <div className="flex items-center gap-2">
          {/* Circular Icon Container */}
          <div className={`w-8 h-8 rounded-full flex items-center justify-center shadow-md transition-colors ${
            isDark 
              ? 'bg-[#c9983a]' 
              : 'bg-[#c9983a]'
          }`}>
            <Circle className="w-4 h-4 text-white fill-white" strokeWidth={0} />
          </div>
          {/* Issue Number Badge */}
          {number && (
            <div className={`px-2.5 py-1 rounded-[8px] shadow-md transition-colors ${
              isDark 
                ? 'bg-[#c9983a]' 
                : 'bg-[#c9983a]'
            }`}>
              <span className="text-[12px] font-bold text-white">{number}</span>
            </div>
          )}
        </div>
      </div>

      {/* Issue Title */}
      <h3 className={`text-[14px] font-bold mb-2 line-clamp-2 transition-colors ${
        isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}>
        {title}
      </h3>

      {/* Repository and Applicants */}
      {repository && applicants !== undefined && (
        <div className="flex items-center gap-3 mb-2">
          <span className={`text-[11px] transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
            {repository}
          </span>
          <div className="flex items-center gap-1">
            <Users className={`w-3 h-3 transition-colors ${isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'}`} />
            <span className={`text-[11px] font-semibold transition-colors ${isDark ? 'text-[#c9983a]' : 'text-[#8b6f3a]'}`}>
              {applicants} applicant{applicants !== 1 ? 's' : ''}
            </span>
          </div>
        </div>
      )}

      {/* Author */}
      {author && timeAgo && (
        <div className="flex items-center gap-2">
          <img 
            src={author.avatar} 
            alt={author.name}
            className="w-5 h-5 rounded-full border border-[#c9983a]/30"
          />
          <span className={`text-[11px] font-semibold transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
            {author.name}
          </span>
          <span className={`text-[11px] transition-colors ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
            {timeAgo}
          </span>
        </div>
      )}

      {/* Optional Tags */}
      {showTags && tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mt-3">
          {tags.map((tag, idx) => (
            <span
              key={idx}
              className={`px-2 py-1 rounded-[6px] text-[10px] font-bold backdrop-blur-[20px] border border-white/25 transition-colors ${
                isDark ? 'bg-white/[0.08] text-[#d4d4d4]' : 'bg-white/[0.08] text-[#4a3f2f]'
              }`}
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </button>
  );
}