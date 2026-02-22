import { useState, useEffect } from 'react';
import { Info } from 'lucide-react';
import { SkeletonLoader } from '../shared/SkeletonLoader';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { getProjectsContributed } from '../../../../shared/api/client';
import { useBillingProfiles } from '../../contexts/BillingProfilesContext';
import { LanguageIcon } from '../../../../shared/components/LanguageIcon';

export function PayoutTab() {
  const { theme } = useTheme();
  const { profiles } = useBillingProfiles();
  const [isLoading, setIsLoading] = useState(true);
  const [projects, setProjects] = useState<Array<{
    id: string;
    github_full_name: string;
    status: string;
    ecosystem_name?: string;
    language?: string;
    owner_avatar_url?: string;
  }>>([]);
  const [projectMappings, setProjectMappings] = useState<Record<string, number | null>>({});
const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    const fetchProjects = async () => {
      try {
        setIsLoading(true);
        const response = await getProjectsContributed();
        setProjects(response || []);
      } catch (error) {
        console.error('Failed to fetch projects:', error);
        setErrorMessage("Failed to load projects. Please try again later.");
        setProjects([]);
      } finally {
        setIsLoading(false);
      }
    };

    fetchProjects();
  }, []);

  const handleProfileChange = (projectId: string, profileId: string) => {
    setProjectMappings((prev) => ({
      ...prev,
      [projectId]: profileId === '' ? null : parseInt(profileId, 10),
    }));
  };

  const handleSave = () => {
    // TODO: Implement save to backend
    console.log('Saving payout preferences:', projectMappings);
  };

  const getProjectInitial = (fullName: string) => {
    const parts = fullName.split('/');
    return parts[parts.length - 1][0].toUpperCase();
  };

  const getProjectDisplayName = (fullName: string) => {
    const parts = fullName.split('/');
    return parts[parts.length - 1];
  };

  // Only show verified billing profiles
  const availableProfiles = profiles.filter(p => p.status === 'verified');

  return (
    <div className="space-y-6">
      {errorMessage && (
              <div className="bg-red-500/10 border border-red-500/50 text-red-500 p-3 rounded-lg mb-4">
                      {errorMessage}
                            </div>
                                )}
      
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        {isLoading ? (
          <>
            <SkeletonLoader className="h-7 w-48 mb-2" />
            <SkeletonLoader className="h-4 w-full mb-1" />
            <SkeletonLoader className="h-4 w-3/4 mb-8" />
          </>
        ) : (
          <>
            <h2 className={`text-[28px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Payout preferences</h2>
            <p className={`text-[14px] mb-8 transition-colors ${
              theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
              Connect your billing profile to the projects you contribute to for receiving rewards.
            </p>
          </>
        )}

        {isLoading ? (
          // Loading State with granular skeletons matching discover/explore style
          <div className="space-y-4 mb-6">
            {/* Header Row Skeleton */}
            <div className="grid grid-cols-2 gap-4 pb-3 border-b border-white/10">
              <SkeletonLoader className="h-4 w-20" />
              <SkeletonLoader className="h-4 w-32" />
            </div>

            {/* Project Rows Skeleton - Granular like discover/explore */}
            {Array.from({ length: 4 }).map((_, idx) => (
              <div key={idx} className="grid grid-cols-2 gap-4 items-center py-4 border-b border-white/5">
                {/* Project Column Skeleton */}
                <div className="flex items-center gap-3">
                  <SkeletonLoader variant="circle" className="w-10 h-10 flex-shrink-0" />
                  <div className="flex flex-col gap-2">
                    <SkeletonLoader className="h-4 w-32" />
                    <SkeletonLoader className="h-3 w-24" />
                  </div>
                </div>
                {/* Billing Profile Column Skeleton */}
                <div className="flex items-center">
                  <SkeletonLoader className="h-10 w-[300px] rounded-[12px]" />
                </div>
              </div>
            ))}
          </div>
        ) : projects.length === 0 ? (
          <div className="text-center py-12">
            <div className={`w-16 h-16 rounded-full mx-auto mb-4 flex items-center justify-center ${
              theme === 'dark' ? 'bg-white/[0.08]' : 'bg-white/[0.15]'
            }`}>
              <Info className={`w-8 h-8 ${
                theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`} />
            </div>
            <p className={`text-[16px] font-semibold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>
              No projects found
            </p>
            <p className={`text-[14px] transition-colors ${
              theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
              Start contributing to projects to see them here
            </p>
          </div>
        ) : (
          <>
            {/* Project List */}
            <div className="space-y-4 mb-6">
              {/* Header Row */}
              <div className="grid grid-cols-2 gap-4 pb-3 border-b border-white/10">
                <div className={`text-[13px] font-semibold transition-colors ${
                  theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                }`}>Project</div>
                <div className={`text-[13px] font-semibold transition-colors ${
                  theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                }`}>Billing profile</div>
              </div>

              {/* Project Rows */}
              {projects.map((project) => (
                <div key={project.id} className="grid grid-cols-2 gap-4 items-center py-4 border-b border-white/5">
                  <div className="flex items-center gap-3">
                    {project.owner_avatar_url ? (
                      <img
                        src={project.owner_avatar_url}
                        alt={getProjectDisplayName(project.github_full_name)}
                        className="w-10 h-10 rounded-full object-cover border border-[#c9983a]/30"
                        onError={(e) => {
                          // Fallback to language icon or initial if image fails to load
                          const target = e.target as HTMLImageElement;
                          target.style.display = 'none';
                          const fallback = target.nextElementSibling as HTMLElement;
                          if (fallback) fallback.style.display = 'flex';
                        }}
                      />
                    ) : null}
                    <div className={`w-10 h-10 rounded-full flex items-center justify-center ${project.owner_avatar_url ? 'hidden' : ''} ${
                      project.language ? '' : 'bg-gradient-to-br from-[#c9983a]/20 to-[#a67c2e]/20 border border-[#c9983a]/30'
                    }`}>
                      {project.language ? (
                        <LanguageIcon language={project.language} size={40} />
                      ) : (
                        <span className={`text-[14px] font-bold transition-colors ${
                          theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#2d2820]'
                        }`}>{getProjectInitial(project.github_full_name)}</span>
                      )}
                    </div>
                    <div className="flex flex-col">
                      <span className={`text-[15px] font-medium transition-colors ${
                        theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#2d2820]'
                      }`}>{getProjectDisplayName(project.github_full_name)}</span>
                      {project.ecosystem_name && (
                        <span className={`text-[12px] transition-colors ${
                          theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
                        }`}>{project.ecosystem_name}</span>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center">
                    {availableProfiles.length === 0 ? (
                      <div className={`px-4 py-2.5 rounded-[12px] text-[14px] transition-colors ${
                        theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
                      }`}>
                        No verified billing profiles
                      </div>
                    ) : (
                      <select
                        value={projectMappings[project.id]?.toString() || ''}
                        onChange={(e) => handleProfileChange(project.id, e.target.value)}
                        className={`w-full max-w-[300px] px-4 py-2.5 rounded-[12px] backdrop-blur-[30px] border text-[14px] focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/30 transition-all ${
                          theme === 'dark'
                            ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5]'
                            : 'bg-white/[0.15] border-white/25 text-[#2d2820]'
                        }`}
                      >
                        <option value="">Select billing profile</option>
                        {availableProfiles.map((profile) => (
                          <option key={profile.id} value={profile.id.toString()}>
                            {profile.name}
                          </option>
                        ))}
                      </select>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {/* Info Message */}
            <div className={`flex items-start gap-2 p-4 rounded-[14px] backdrop-blur-[30px] border transition-colors ${
              theme === 'dark'
                ? 'bg-[#3d342c]/[0.3] border-white/10'
                : 'bg-white/[0.08] border-white/15'
            }`}>
              <Info className={`w-5 h-5 flex-shrink-0 mt-0.5 transition-colors ${
                theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
              }`} />
              <p className={`text-[13px] transition-colors ${
                theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>
                {availableProfiles.length === 0
                  ? 'Create and verify a billing profile in the Billing Profiles tab to connect it to projects.'
                  : 'Only projects for which you have already received rewards will appear here.'}
              </p>
            </div>
          </>
        )}
      </div>

      {/* Save Button */}
      {!isLoading && projects.length > 0 && (
        <div className="flex justify-end">
          <button
            onClick={handleSave}
            className="px-8 py-3 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[15px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10"
          >
            Save
          </button>
        </div>
      )}
    </div>
  );
}
