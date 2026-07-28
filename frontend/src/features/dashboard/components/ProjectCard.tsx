import { Star, GitFork, Package } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { useState } from 'react';

export interface Project {
  id: number | string;
  name: string;
  icon: string;
  stars: string;
  forks: string;
  contributors: number;
  openIssues: number;
  prs: number;
  description: string;
  tags: string[];
  color: string;
}

interface ProjectCardProps {
  project: Project;
  onClick?: (id: string) => void;
}

export function ProjectCard({ project, onClick }: ProjectCardProps) {
  const { theme } = useTheme();
  const [avatarError, setAvatarError] = useState(false);

  // Check if icon is a URL (GitHub avatar) or emoji/text
  const isAvatarUrl = project.icon.startsWith('http');

  return (
    <div
      className={`backdrop-blur-[25px] rounded-[18px] border p-5 transition-all cursor-pointer shadow-elevation-1 hover:shadow-elevation-2 hover:scale-[1.02] ${
        theme === 'dark'
          ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12] hover:border-[#e8c571]/30'
          : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:border-[#c9983a]/30'
      }`}
      onClick={() => onClick?.(project.id.toString())}
    >
      <div className="flex items-start justify-between mb-4">
        {isAvatarUrl && !avatarError ? (
          <img
            src={project.icon}
            alt={project.name}
            className="w-11 h-11 rounded-[12px] border border-white/20 flex-shrink-0"
            onError={() => setAvatarError(true)}
          />
        ) : (
          <div className={`w-11 h-11 rounded-[12px] bg-gradient-to-br ${project.color} flex items-center justify-center shadow-md ${
            isAvatarUrl ? '' : 'text-xl'
          }`}>
            {isAvatarUrl ? (
              <Package className="w-6 h-6 text-white" />
            ) : (
              project.icon
            )}
          </div>
        )}
      </div>

      <h4 className={`text-[16px] font-bold mb-2 transition-colors ${
        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
      }`}>{project.name}</h4>
      <p className={`text-[12px] mb-3 line-clamp-2 transition-colors ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
      }`}>{project.description}</p>

      <div className={`flex items-center gap-2 text-[12px] mb-3 transition-colors ${
        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
      }`}>
        <div className="flex items-center gap-1">
          <Star className="w-3 h-3 text-[#c9983a]" />
          <span>{project.stars}</span>
        </div>
        <div className="flex items-center gap-1">
          <GitFork className="w-3 h-3 text-[#c9983a]" />
          <span>{project.forks}</span>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-1.5 mb-3 pb-3 border-b border-white/10">
        <div className="text-center">
          <div className={`text-[18px] font-bold transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>{project.contributors}</div>
          <div className={`text-[10px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>Contributors</div>
        </div>
        <div className="text-center">
          <div className={`text-[18px] font-bold transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>{project.openIssues}</div>
          <div className={`text-[10px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>Issues</div>
        </div>
        <div className="text-center">
          <div className={`text-[18px] font-bold transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>{project.prs}</div>
          <div className={`text-[10px] transition-colors ${
            theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>PRs</div>
        </div>
      </div>

      <div className="flex flex-wrap gap-1.5">
        {project.tags.map((tag, idx) => (
          <span
            key={idx}
            className={`px-2 py-1 rounded-[8px] text-[11px] font-semibold shadow-[0_2px_8px_rgba(201,152,58,0.1)] ${
              theme === 'dark'
                ? 'bg-[#c9983a]/20 border border-[#c9983a]/40 text-[#f5c563]'
                : 'bg-[#c9983a]/20 border border-[#c9983a]/35 text-[#8b6f3a]'
            }`}
          >
            {tag}
          </span>
        ))}
      </div>
    </div>
  );
}
