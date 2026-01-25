import { useEffect, useMemo, useState } from 'react';
import { ArrowLeft } from 'lucide-react';

import { useTheme } from '../../../shared/contexts/ThemeContext';
import { getMyProjects, getPublicProject } from '../../../shared/api/client';
import { IssuesTab } from '../../maintainers/components/issues/IssuesTab';

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
  const [myProjects, setMyProjects] = useState<Array<{ id: string; github_full_name: string; status: string }>>([]);

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

      <IssuesTab
        onNavigate={() => {}}
        selectedProjects={myProjects}
        initialSelectedIssueId={issueId}
        initialSelectedProjectId={projectId}
      />
    </div>
  );
}


