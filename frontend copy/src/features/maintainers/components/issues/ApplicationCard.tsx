import { User, ExternalLink, Award, GitPullRequest, Trophy, Users, Star, Check } from 'lucide-react';
import { Applicant } from '../../types';

interface ApplicationCardProps {
  applicant: Applicant;
  status: 'assigned' | 'pending';
  onProfileClick: () => void;
}

export function ApplicationCard({ applicant, status, onProfileClick }: ApplicationCardProps) {
  return (
    <div className="backdrop-blur-[25px] bg-white/[0.15] rounded-[16px] border border-white/25 p-6">
      {/* Clickable User Header */}
      <button 
        onClick={onProfileClick}
        className="w-full flex items-center gap-3 mb-5 hover:bg-white/10 -m-2 p-2 rounded-[12px] transition-all group/user"
      >
        <div className="w-12 h-12 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center shadow-[0_4px_12px_rgba(201,152,58,0.3)]">
          <User className="w-6 h-6 text-white" />
        </div>
        <div className="text-left">
          <h4 className="text-[15px] font-bold text-[#2d2820] group-hover/user:text-[#c9983a] transition-colors">
            {applicant.name}
          </h4>
          <p className="text-[12px] text-[#7a6b5a]">Applied - {applicant.appliedDate}</p>
        </div>
        <ExternalLink className="w-4 h-4 text-[#7a6b5a] ml-auto opacity-0 group-hover/user:opacity-100 transition-opacity" />
      </button>

      {/* Badge */}
      {applicant.badge && (
        <div className="inline-flex items-center gap-2 px-4 py-2 rounded-[10px] bg-gradient-to-r from-[#c9983a]/20 to-[#d4af37]/15 border border-[#c9983a]/30 mb-5">
          <Award className="w-4 h-4 text-[#c9983a]" />
          <span className="text-[13px] font-bold text-[#2d2820]">{applicant.badge}</span>
        </div>
      )}

      {/* Profile Stats */}
      {applicant.profileStats && (
        <div className="grid grid-cols-2 gap-3 mb-5">
          <div className="backdrop-blur-[20px] bg-white/[0.12] rounded-[12px] border border-[#c9983a]/20 p-3">
            <div className="flex items-center gap-2 mb-1">
              <GitPullRequest className="w-4 h-4 text-[#c9983a]" />
              <span className="text-[20px] font-bold text-[#2d2820]">{applicant.profileStats.contributions}</span>
            </div>
            <p className="text-[11px] font-semibold text-[#7a6b5a] uppercase tracking-wide">Contributions</p>
          </div>
          <div className="backdrop-blur-[20px] bg-white/[0.12] rounded-[12px] border border-[#c9983a]/20 p-3">
            <div className="flex items-center gap-2 mb-1">
              <Trophy className="w-4 h-4 text-[#c9983a]" />
              <span className="text-[20px] font-bold text-[#2d2820]">{applicant.profileStats.rewards}</span>
            </div>
            <p className="text-[11px] font-semibold text-[#7a6b5a] uppercase tracking-wide">Rewards</p>
          </div>
        </div>
      )}

      {/* Additional Profile Info */}
      {applicant.profileStats && (
        <div className="space-y-2 mb-5">
          <div className="flex items-center gap-2">
            <Users className="w-4 h-4 text-[#7a6b5a]" />
            <span className="text-[13px] text-[#7a6b5a]">
              Contributor on <span className="font-bold text-[#2d2820]">{applicant.profileStats.contributorProjects}</span> projects
            </span>
          </div>
          <div className="flex items-center gap-2">
            <Star className="w-4 h-4 text-[#7a6b5a]" />
            <span className="text-[13px] text-[#7a6b5a]">
              Lead <span className="font-bold text-[#2d2820]">{applicant.profileStats.leadProjects}</span> projects
            </span>
          </div>
        </div>
      )}

      {/* Message */}
      {applicant.message && (
        <div className="p-4 rounded-[12px] bg-white/20 border border-white/30 mb-5">
          <p className="text-[13px] text-[#2d2820] leading-relaxed">
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
            <button className="px-4 py-2 rounded-[8px] bg-white/30 hover:bg-white/50 border border-white/40 hover:border-[#c9983a]/40 text-[13px] font-semibold text-[#2d2820] hover:text-[#c9983a] transition-all">
              Unassign
            </button>
          </>
        ) : (
          <>
            <button className="flex-1 px-4 py-2 rounded-[8px] bg-white/30 hover:bg-white/50 border border-white/40 hover:border-[#c9983a]/40 text-[13px] font-semibold text-[#2d2820] hover:text-[#c9983a] transition-all mr-2">
              Reject
            </button>
            <button className="flex-1 px-4 py-2 rounded-[8px] bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border border-[#c9983a]/40 text-[13px] font-semibold text-[#2d2820] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 hover:shadow-[0_4px_16px_rgba(201,152,58,0.3)] transition-all">
              Assign
            </button>
          </>
        )}
      </div>
    </div>
  );
}
