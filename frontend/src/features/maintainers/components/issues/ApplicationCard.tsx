import { User, ExternalLink, Award, GitPullRequest, Trophy, Users, Star, Check } from 'lucide-react';
import { Applicant } from '../../types';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

interface ApplicationCardProps {
  applicant: Applicant;
  status: 'assigned' | 'pending';
  onProfileClick: () => void;
}

export function ApplicationCard({ applicant, status, onProfileClick }: ApplicationCardProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  return (
    <div className={`backdrop-blur-[25px] rounded-[16px] border p-6 shadow-elevation-1 transition-all ${
      isDark
        ? 'bg-white/[0.08] border-white/15 text-[#e8dfd0]'
        : 'bg-white/[0.15] border-white/25 text-[#2d2820]'
    }`}>
      {/* Clickable User Header */}
      <button 
        onClick={onProfileClick}
        className={`w-full flex items-center gap-3 mb-5 -m-2 p-2 rounded-[12px] transition-all group/user focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
          isDark ? 'hover:bg-white/[0.08]' : 'hover:bg-black/[0.05]'
        }`}
      >
        <div className="w-12 h-12 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center shadow-[0_4px_12px_rgba(201,152,58,0.3)]">
          <User className="w-6 h-6 text-white" />
        </div>
        <div className="text-left">
          <h4 className={`text-[15px] font-bold transition-colors group-hover/user:text-[#c9983a] ${
            isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>
            {applicant.name}
          </h4>
          <p className={`text-[12px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>Applied - {applicant.appliedDate}</p>
        </div>
        <ExternalLink className={`w-4 h-4 ml-auto opacity-0 group-hover/user:opacity-100 transition-opacity ${
          isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`} />
      </button>

      {/* Badge */}
      {applicant.badge && (
        <div className={`inline-flex items-center gap-2 px-4 py-2 rounded-[10px] border mb-5 ${
          isDark
            ? 'bg-[#c9983a]/20 border-[#c9983a]/40'
            : 'bg-[#c9983a]/20 border-[#c9983a]/30'
        }`}>
          <Award className="w-4 h-4 text-[#c9983a]" />
          <span className={`text-[13px] font-bold ${
            isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
          }`}>{applicant.badge}</span>
        </div>
      )}

      {/* Profile Stats */}
      {applicant.profileStats && (
        <div className="grid grid-cols-2 gap-3 mb-5">
          <div className={`backdrop-blur-[20px] rounded-[12px] border p-3 ${
            isDark
              ? 'bg-white/[0.04] border-[#c9983a]/30'
              : 'bg-white/[0.12] border-[#c9983a]/20'
          }`}>
            <div className="flex items-center gap-2 mb-1">
              <GitPullRequest className="w-4 h-4 text-[#c9983a]" />
              <span className={`text-[20px] font-bold ${
                isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>{applicant.profileStats.contributions}</span>
            </div>
            <p className={`text-[11px] font-semibold uppercase tracking-wide ${
              isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>Contributions</p>
          </div>
          <div className={`backdrop-blur-[20px] rounded-[12px] border p-3 ${
            isDark
              ? 'bg-white/[0.04] border-[#c9983a]/30'
              : 'bg-white/[0.12] border-[#c9983a]/20'
          }`}>
            <div className="flex items-center gap-2 mb-1">
              <Trophy className="w-4 h-4 text-[#c9983a]" />
              <span className={`text-[20px] font-bold ${
                isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>{applicant.profileStats.rewards}</span>
            </div>
            <p className={`text-[11px] font-semibold uppercase tracking-wide ${
              isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>Rewards</p>
          </div>
        </div>
      )}

      {/* Additional Profile Info */}
      {applicant.profileStats && (
        <div className="space-y-2 mb-5">
          <div className="flex items-center gap-2">
            <Users className={`w-4 h-4 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`} />
            <span className={`text-[13px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              Contributor on <span className={`font-bold ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>{applicant.profileStats.contributorProjects}</span> projects
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Star className={`w-4 h-4 ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`} />
            <span className={`text-[13px] ${isDark ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              Lead <span className={`font-bold ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>{applicant.profileStats.leadProjects}</span> projects
            </span>
          </div>
        </div>
      )}

      {/* Message */}
      {applicant.message && (
        <div className={`p-4 rounded-[12px] border mb-5 ${
          isDark
            ? 'bg-white/[0.04] border-white/10'
            : 'bg-white/20 border-white/30'
        }`}>
          <p className={`text-[13px] leading-relaxed ${
            isDark ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
          }`}>
            {applicant.message}
          </p>
        </div>
      )}

      {/* Status & Action Button */}
      <div className="flex items-center justify-between">
        {status === 'assigned' ? (
          <>
            <div className="flex items-center gap-2">
              <div className="w-5 h-5 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center">
                <Check className="w-3 h-3 text-white" strokeWidth={3} />
              </div>
              <span className="text-[13px] font-bold text-[#c9983a]">Assigned</span>
            </div>
            <button className={`px-4 py-2 rounded-[8px] border text-[13px] font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
              isDark
                ? 'bg-white/[0.08] hover:bg-white/[0.15] border-white/15 text-[#e8dfd0] hover:text-[#c9983a] hover:border-[#c9983a]/40'
                : 'bg-white/30 hover:bg-white/50 border-white/40 text-[#2d2820] hover:text-[#c9983a] hover:border-[#c9983a]/40'
            }`}>
              Unassign
            </button>
          </>
        ) : (
          <>
            <button className={`flex-1 px-4 py-2 rounded-[8px] border text-[13px] font-semibold transition-all mr-2 focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
              isDark
                ? 'bg-white/[0.08] hover:bg-white/[0.15] border-white/15 text-[#e8dfd0] hover:text-[#c9983a] hover:border-[#c9983a]/40'
                : 'bg-white/30 hover:bg-white/50 border-white/40 text-[#2d2820] hover:text-[#c9983a] hover:border-[#c9983a]/40'
            }`}>
              Reject
            </button>
            <button className={`flex-1 px-4 py-2 rounded-[8px] border text-[13px] font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
              isDark
                ? 'bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border-[#c9983a]/50 text-[#e8dfd0] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 hover:shadow-elevation-2'
                : 'bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border-[#c9983a]/40 text-[#2d2820] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 hover:shadow-elevation-2'
            }`}>
              Assign
            </button>
          </>
        )}
      </div>
    </div>
  );
}
