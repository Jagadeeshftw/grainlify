import { useEffect, useMemo, useState } from 'react';
import { ArrowLeft } from 'lucide-react';

import { useTheme } from '../../../shared/contexts/ThemeContext';
import { getMyProjects, getPublicProject } from '../../../shared/api/client';
import { IssuesTab } from '../../maintainers/components/issues/IssuesTab';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';
import { IssueCardSkeleton } from '../../../shared/components/IssueCardSkeleton';

interface ProjectForIssues {
  id: string;
  github_full_name: string;
  status: string;
}

interface IssueDetailPageProps {
  issueId?: string;
  projectId?: string;
  onClose: () => void;
}

export function IssueDetailPage({ issueId, projectId, onClose }: IssueDetailPageProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const [project, setProject] = useState<null | Awaited<ReturnType<typeof getPublicProject>>>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [myProjects, setMyProjects] = useState<ProjectForIssues[]>([]);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setIsLoading(true);
      try {
        const [p, mine] = await Promise.all([
          projectId ? getPublicProject(projectId) : Promise.resolve(null as any),
          getMyProjects(),
        ]);
        if (cancelled) return;
        if (projectId) setProject(p);
        setMyProjects(
          (Array.isArray(mine) ? mine : []).map((x) => ({
            id: x.id,
            github_full_name: x.github_full_name,
            status: x.status,
          }))
        );
      } finally {
        if (cancelled) return;
        setIsLoading(false);
      }
    };

    load();
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  // Include the current project so contributors (who have no "my projects") still see the issue
  const selectedProjects = useMemo((): ProjectForIssues[] => {
    const current: ProjectForIssues | null = project
      ? { id: project.id, github_full_name: project.github_full_name, status: 'verified' }
      : null;
    const others = myProjects.filter((m) => !current || m.id !== current.id);
    return current ? [current, ...others] : others;
  }, [project, myProjects]);

  const repoName = useMemo(() => {
    const full = project?.github_full_name || '';
    const parts = full.split('/');
    return parts[1] || full || 'Project';
  }, [project?.github_full_name]);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <button
          onClick={onClose}
          className={`flex items-center gap-2 px-4 py-2.5 rounded-[16px] backdrop-blur-[40px] border transition-all ${
            isDark ? 'bg-white/[0.12] border-white/20 text-[#f5f5f5]' : 'bg-white/[0.35] border-black/10 text-[#2d2820]'
          }`}
        >
          <ArrowLeft className="w-4 h-4" />
          <span className="text-[13px] font-semibold">Back</span>
        </button>
      </div>

      {isLoading ? (
        <div className="flex gap-6 h-[calc(100vh-220px)]">
          <div className="w-[450px] flex-shrink-0 flex flex-col h-full space-y-4">
            <SkeletonLoader className="h-12 w-full rounded-[16px]" />
            <div className="space-y-3 flex-1 overflow-hidden">
              {[...Array(6)].map((_, idx) => (
                <IssueCardSkeleton key={idx} />
              ))}
            </div>
          </div>
          <div
            className={`flex-1 rounded-[24px] border overflow-hidden ${
              isDark ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
            }`}
          >
            <div className="p-8 space-y-4">
              <SkeletonLoader className="h-8 w-3/4 rounded-[10px]" />
              <SkeletonLoader className="h-4 w-full rounded-[10px]" />
              <SkeletonLoader className="h-4 w-full rounded-[10px]" />
              <SkeletonLoader className="h-24 w-full rounded-[12px]" />
            </div>
          </div>
        </div>
      ) : (
        <IssuesTab
          onNavigate={() => {}}
          selectedProjects={selectedProjects}
          initialSelectedIssueId={issueId}
          initialSelectedProjectId={projectId}
          viewMode="contributor"
        />
      )}
    </div>
  );
}


