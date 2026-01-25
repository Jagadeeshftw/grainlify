import { useState } from 'react';
import { ContributionsTab } from '../components/ContributionsTab';
import { ProjectsTab } from '../components/ProjectsTab';
import { RewardsTab } from '../components/RewardsTab';
import { useTheme } from '../../../shared/contexts/ThemeContext';

export function ContributorsPage() {
  const { theme } = useTheme();
  const [activeTab, setActiveTab] = useState<'contributions' | 'projects' | 'rewards'>('contributions');

  return (
    <div className="space-y-6">
      {/* Tabs */}
      <div className="flex items-center justify-between">
        <div className="inline-flex items-center backdrop-blur-[30px] bg-white/[0.12] rounded-[16px] border border-white/25 p-1.5 shadow-[0_4px_12px_rgba(0,0,0,0.06)]">
          <button
            onClick={() => setActiveTab('contributions')}
            className={`px-6 py-2.5 rounded-[12px] font-semibold text-[14px] transition-all ${
              activeTab === 'contributions'
                ? 'bg-[#c9983a] text-white shadow-[0_4px_12px_rgba(201,152,58,0.4)]'
                : theme === 'dark'
                  ? 'bg-transparent text-[#b8a898] hover:text-[#d4c5b0]'
                  : 'bg-transparent text-[#7a6b5a] hover:text-[#2d2820]'
            }`}
          >
            Contributions
          </button>
          <button
            onClick={() => setActiveTab('projects')}
            className={`px-6 py-2.5 rounded-[12px] font-semibold text-[14px] transition-all ${
              activeTab === 'projects'
                ? 'bg-[#c9983a] text-white shadow-[0_4px_12px_rgba(201,152,58,0.4)]'
                : theme === 'dark'
                  ? 'bg-transparent text-[#b8a898] hover:text-[#d4c5b0]'
                  : 'bg-transparent text-[#7a6b5a] hover:text-[#2d2820]'
            }`}
          >
            Projects
          </button>
          <button
            onClick={() => setActiveTab('rewards')}
            className={`px-6 py-2.5 rounded-[12px] font-semibold text-[14px] transition-all ${
              activeTab === 'rewards'
                ? 'bg-[#c9983a] text-white shadow-[0_4px_12px_rgba(201,152,58,0.4)]'
                : theme === 'dark'
                  ? 'bg-transparent text-[#b8a898] hover:text-[#d4c5b0]'
                  : 'bg-transparent text-[#7a6b5a] hover:text-[#2d2820]'
            }`}
          >
            Rewards
          </button>
        </div>

        {/* Action Buttons - Only show on Rewards tab */}
        {activeTab === 'rewards' && (
          <div className="flex items-center gap-3">
            <button className="px-4 py-3 rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 text-[13px] font-medium text-[#6b5d4d] hover:bg-white/[0.2] transition-all whitespace-nowrap">
              See transactions
            </button>
            <button className="px-4 py-3 rounded-[12px] bg-[#c9983a] text-white text-[13px] font-semibold shadow-[0_4px_12px_rgba(201,152,58,0.3)] hover:shadow-[0_6px_16px_rgba(201,152,58,0.4)] transition-all whitespace-nowrap">
              Request payment
            </button>
          </div>
        )}
      </div>

      {/* Tab Content */}
      {activeTab === 'contributions' && <ContributionsTab />}
      {activeTab === 'projects' && <ProjectsTab />}
      {activeTab === 'rewards' && <RewardsTab />}
    </div>
  );
}