import { Circle, FileText, User, Rocket, Users, User as UserIcon } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { Issue } from '../../types';

interface IssueCardProps {
  issue: Issue;
  index: number;
  onClick: () => void;
}

export function IssueCard({ issue, index, onClick }: IssueCardProps) {
  const { theme } = useTheme();
  const getIcon = () => {
    const iconColor = theme === 'dark' ? '#b8a898' : '#7a6b5a';
    switch (issue.icon) {
      case 'rocket':
        return <Rocket className="w-4 h-4" style={{ color: iconColor }} />;
      case 'users':
        return <Users className="w-4 h-4" style={{ color: iconColor }} />;
      case 'user':
        return <UserIcon className="w-4 h-4" style={{ color: iconColor }} />;
      default:
        return null;
    }
  };

  return (
    <div
      className={`backdrop-blur-[25px] rounded-[16px] border p-4 hover:border-[#c9983a]/30 hover:shadow-[0_8px_24px_rgba(0,0,0,0.12)] transition-all duration-300 cursor-pointer group/issue ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/10 hover:bg-white/[0.12]'
          : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.22]'
      }`}
      style={{ animationDelay: `${index * 60}ms` }}
      onClick={onClick}
    >
      {/* Issue Header */}
      <div className="flex items-start gap-3 mb-3">
        <div className="flex items-center gap-2">
          <div className="w-5 h-5 rounded-full bg-gradient-to-br from-[#c9983a]/50 to-[#d4af37]/40 flex items-center justify-center border border-[#c9983a]/40">
            <Circle className="w-2.5 h-2.5 text-white fill-white" strokeWidth={0} />
          </div>
          <span className="text-[13px] font-bold text-[#c9983a]">#{issue.id}</span>
        </div>
        
        {issue.icon && (
          <div className="ml-auto">
            {getIcon()}
          </div>
        )}
      </div>

      {/* Issue Title */}
      <h3 className={`text-[14px] font-semibold mb-2 line-clamp-2 transition-colors ${
        theme === 'dark'
          ? 'text-[#e8dfd0] group-hover/issue:text-[#f5ede0]'
          : 'text-[#2d2820] group-hover/issue:text-[#4a3f2f]'
      }`}>
        {issue.title}
      </h3>

      {/* Repository and Stats in same row */}
      <div className="flex items-center gap-4 mb-3">
        <p className={`text-[12px] font-medium transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>{issue.repo}</p>
        <div className="flex items-center gap-1.5">
          <FileText className={`w-3.5 h-3.5 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`} />
          <span className={`text-[12px] font-semibold transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}>{issue.comments}</span>
        </div>
        <div className="flex items-center gap-1.5">
          <User className={`w-3.5 h-3.5 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`} />
          <span className={`text-[12px] font-semibold transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}>
            {issue.applicants} {issue.applicants === 1 ? 'applicant' : 'applicants'}
          </span>
        </div>
      </div>

      {/* Tags */}
      {issue.tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mb-3">
          {issue.tags.map((tag, tagIdx) => (
            <span
              key={tagIdx}
              className="inline-flex px-2 py-1 rounded-[6px] bg-[#7a6b5a]/20 text-[10px] font-semibold text-[#7a6b5a] border border-[#7a6b5a]/30"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* User & Time */}
      <div className={`flex items-center gap-2 pt-3 border-t transition-colors ${
        theme === 'dark' ? 'border-white/10' : 'border-white/20'
      }`}>
        <img
          src={`https://github.com/${issue.user}.png?size=24`}
          alt={issue.user}
          className="w-6 h-6 rounded-full border border-[#c9983a]/40"
          onError={(e) => {
            // Fallback to initials if image fails to load
            const target = e.target as HTMLImageElement;
            target.style.display = 'none';
            const parent = target.parentElement;
            if (parent) {
              const fallback = document.createElement('div');
              fallback.className = 'w-6 h-6 rounded-full bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border border-[#c9983a]/40 flex items-center justify-center';
              const span = document.createElement('span');
              span.className = 'text-[10px] font-bold text-[#c9983a]';
              span.textContent = issue.user.substring(0, 2).toUpperCase();
              fallback.appendChild(span);
              parent.insertBefore(fallback, target);
            }
          }}
        />
        <span className={`text-[11px] font-semibold transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>{issue.user}</span>
        <span className={`text-[11px] ml-auto transition-colors ${
          theme === 'dark' ? 'text-[#9a8b7a]' : 'text-[#9a8b7a]'
        }`}>{issue.timeAgo}</span>
      </div>
    </div>
  );
}