import React, { createContext, useContext, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import { ExternalLink, Copy, Circle, ArrowLeft, GitPullRequest } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { getPublicProject, getPublicProjectIssues, getPublicProjectPRs } from '../../../shared/api/client';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';
import ReactMarkdown from 'react-markdown';
import { LanguageIcon } from '../../../shared/components/LanguageIcon';

const InPreContext = createContext(false);

interface ProjectDetailPageProps {
  onBack?: () => void;
  onIssueClick?: (issueId: string, projectId: string) => void;
  projectId?: string;
  onClose?: () => void;
}

function OverviewMarkdown({ readme, theme }: { readme: string; theme: string }) {
  const inPre = useContext(InPreContext);
  const dark = theme === 'dark';
  const textColor = dark ? 'text-[#d4d4d4]' : 'text-[#4a3f2f]';
  const headingColor = dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]';

  return (
    <ReactMarkdown
      components={{
        h1: ({ ...props }) => (
          <h1 className={`text-[24px] font-bold mb-4 mt-6 first:mt-0 ${headingColor}`} {...props} />
        ),
        h2: ({ ...props }) => (
          <h2 className={`text-[20px] font-bold mb-3 mt-5 ${headingColor}`} {...props} />
        ),
        h3: ({ ...props }) => (
          <h3 className={`text-[18px] font-semibold mb-2 mt-4 ${headingColor}`} {...props} />
        ),
        h4: ({ ...props }) => (
          <h4 className={`text-[16px] font-semibold mb-2 mt-3 ${headingColor}`} {...props} />
        ),
        p: ({ ...props }) => (
          <p className={`mb-4 leading-relaxed ${textColor}`} {...props} />
        ),
        a: ({ ...props }) => (
          <a
            className={`font-semibold hover:underline ${dark ? 'text-[#f5c563] hover:text-[#ffd700]' : 'text-[#b8872f] hover:text-[#8b6f3a]'}`}
            target="_blank"
            rel="noopener noreferrer"
            {...props}
          />
        ),
        code: ({ ...props }) => {
          if (inPre) {
            return (
              <code
                className={`text-[13px] font-mono ${textColor}`}
                {...props}
              />
            );
          }
          return (
            <code
              className={`inline px-1.5 py-0.5 rounded text-[13px] font-mono ${dark ? 'bg-white/[0.15] text-[#f5c563]' : 'bg-[#e8e0d0] text-[#6b5d4d]'}`}
              {...props}
            />
          );
        },
        pre: ({ children, ...props }) => (
          <InPreContext.Provider value={true}>
            <pre
              className={`mb-4 overflow-x-auto rounded-[12px] p-4 font-mono text-[13px] ${dark ? 'bg-white/[0.12] border border-white/20 text-[#e8dfd0]' : 'bg-white/[0.20] border border-white/30 text-[#2d2820]'}`}
              {...props}
            >
              {children}
            </pre>
          </InPreContext.Provider>
        ),
        ul: ({ ...props }) => (
          <ul className={`list-disc pl-6 mb-4 space-y-1.5 ${textColor}`} {...props} />
        ),
        ol: ({ ...props }) => (
          <ol className={`list-decimal pl-6 mb-4 space-y-1.5 ${textColor}`} {...props} />
        ),
        li: ({ ...props }) => (
          <li className={`leading-relaxed ${textColor}`} {...props} />
        ),
        blockquote: ({ ...props }) => (
          <blockquote
            className={`border-l-4 pl-4 italic my-4 ${dark ? 'border-[#c9983a]/60 text-[#d4d4d4] bg-white/[0.05]' : 'border-[#c9983a]/70 text-[#4a3f2f] bg-white/[0.10]'}`}
            {...props}
          />
        ),
        img: ({ ...props }) => (
          <img className="rounded-[12px] max-w-full h-auto my-4" alt="" {...props} />
        ),
        strong: ({ ...props }) => (
          <strong className={`font-bold ${headingColor}`} {...props} />
        ),
      }}
    >
      {readme}
    </ReactMarkdown>
  );
}

export function ProjectDetailPage({ onBack, onIssueClick, projectId: propProjectId, onClose }: ProjectDetailPageProps) {
  const { theme } = useTheme();
  const { projectId: paramProjectId } = useParams<{ projectId: string }>();
  const projectId = propProjectId || paramProjectId;
  const [activeIssueTab, setActiveIssueTab] = useState('all');
  const [copiedLink, setCopiedLink] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [project, setProject] = useState<null | Awaited<ReturnType<typeof getPublicProject>>>(null);
  const [issues, setIssues] = useState<Array<{
    github_issue_id: number;
    number: number;
    state: string;
    title: string;
    description: string | null;
    author_login: string;
    labels: any[];
    url: string;
    updated_at: string | null;
    last_seen_at: string;
  }>>([]);
  const [prs, setPRs] = useState<Array<{
    github_pr_id: number;
    number: number;
    state: string;
    title: string;
    author_login: string;
    url: string;
    merged: boolean;
    created_at: string | null;
    updated_at: string | null;
    closed_at: string | null;
    merged_at: string | null;
    last_seen_at: string;
  }>>([]);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      if (!projectId) {
        console.warn('ProjectDetailPage: No projectId provided');
        setIsLoading(false);
        return;
      }
      setIsLoading(true);
      try {
        console.log('ProjectDetailPage: Fetching project data for ID:', projectId);
        const [p, i, pr] = await Promise.all([
          getPublicProject(projectId),
          getPublicProjectIssues(projectId),
          getPublicProjectPRs(projectId),
        ]);
        if (cancelled) return;
        console.log('ProjectDetailPage: Data fetched successfully', {
          project: p,
          issuesCount: i?.issues?.length || 0,
          prsCount: pr?.prs?.length || 0,
        });
        setProject(p);
        setIssues(i?.issues || []);
        setPRs(pr?.prs || []);
        setIsLoading(false);
      } catch (e) {
        if (cancelled) return;
        console.error('ProjectDetailPage: Error loading project data', e);
        // Keep loading state true to show skeleton forever when backend is down
        // Don't set isLoading to false - keep showing skeleton
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

  const ownerLogin = project?.repo?.owner_login || (project?.github_full_name?.split('/')[0] || '');
  const ownerAvatar =
    project?.repo?.owner_avatar_url ||
    (ownerLogin ? `https://github.com/${ownerLogin}.png?size=200` : 'https://github.com/github.png?size=200');
  
  // Use project avatar if available, otherwise fallback to owner avatar
  const projectAvatar = project?.repo?.owner_avatar_url || ownerAvatar;

  const githubUrl = project?.repo?.html_url || (project?.github_full_name ? `https://github.com/${project.github_full_name}` : '');
  const websiteUrl = project?.repo?.homepage || '';
  const description = project?.repo?.description || '';

  const languages = useMemo(() => {
    const list = (project?.languages || [])
      .slice()
      .sort((a, b) => b.percentage - a.percentage)
      .map((l) => {
        const pct = Number(l.percentage) || 0;
        const isTiny = pct > 0 && pct < 1;
        return {
          name: l.name,
          percentage: pct,
          displayLabel: isTiny ? '<1%' : `${Math.round(pct)}%`,
          barWidth: isTiny ? 1 : Math.round(pct),
        };
      });
    if (list.length) return list;
    return project?.language ? [{ name: project.language, percentage: 100, displayLabel: '100%', barWidth: 100 }] : [];
  }, [project?.languages, project?.language]);

  const labelName = (l: any): string | null => {
    if (!l) return null;
    if (typeof l === 'string') return l;
    if (typeof l?.name === 'string') return l.name;
    return null;
  };

  const issueTabs = useMemo(() => {
    // Only count open issues
    const openIssues = issues.filter((it) => it.state === 'open');
    const counts = new Map<string, number>();
    for (const it of openIssues) {
      const labels = Array.isArray(it.labels) ? it.labels : [];
      for (const l of labels) {
        const name = labelName(l);
        if (!name) continue;
        counts.set(name, (counts.get(name) || 0) + 1);
      }
    }
    const top = Array.from(counts.entries())
      .sort((a, b) => b[1] - a[1])
      .slice(0, 6)
      .map(([name, count]) => ({ id: name, label: name, count }));
    return [{ id: 'all', label: 'All issues', count: openIssues.length }, ...top];
  }, [issues]);

  const filteredIssues = useMemo(() => {
    // First filter to only open issues
    const openIssues = issues.filter((it) => it.state === 'open');
    
    // Then apply label filter if not 'all'
    if (activeIssueTab === 'all') return openIssues;
    return openIssues.filter((it) => (Array.isArray(it.labels) ? it.labels : []).some((l) => labelName(l) === activeIssueTab));
  }, [issues, activeIssueTab]);

  const timeAgo = (iso?: string | null) => {
    const s = iso || '';
    const d = s ? new Date(s) : null;
    if (!d || Number.isNaN(d.getTime())) return '';
    const diffMs = Date.now() - d.getTime();
    const mins = Math.floor(diffMs / 60000);
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    return `${days}d ago`;
  };

  const contributors = useMemo(() => {
    const uniq = new Map<string, { name: string; avatar: string }>();
    for (const it of [...issues, ...prs]) {
      const login = (it as any).author_login;
      if (!login || uniq.has(login)) continue;
      uniq.set(login, { name: login, avatar: `https://github.com/${login}.png?size=80` });
      if (uniq.size >= 5) break; // Get top 5
    }
    return Array.from(uniq.values());
  }, [issues, prs]);
  
  const recentPRs = useMemo(() => {
    const allPRs = prs.map((p) => {
      // Determine PR status: merged takes priority, then state
      let status: 'merged' | 'open' | 'closed' = 'open';
      if (p.merged) {
        status = 'merged';
      } else if (p.state === 'open') {
        status = 'open';
      } else {
        status = 'closed';
      }
      
      // Use the most appropriate GitHub timestamp
      // Priority: merged_at > closed_at > created_at > updated_at > last_seen_at
      let dateStr: string | null = null;
      if (p.merged && p.merged_at) {
        dateStr = p.merged_at;
      } else if (p.state === 'closed' && p.closed_at) {
        dateStr = p.closed_at;
      } else if (p.state === 'open' && p.created_at) {
        dateStr = p.created_at;
      } else if (p.updated_at) {
        dateStr = p.updated_at;
      } else {
        dateStr = p.last_seen_at;
      }
      
      return {
        type: 'pr' as const,
        id: p.github_pr_id,
        number: p.number,
        title: p.title,
        date: timeAgo(dateStr),
        status: status,
        url: p.url,
      };
    });

    const allIssues = issues.map((issue) => {
      // For issues, use updated_at (GitHub timestamp) with last_seen_at as fallback
      const dateStr = issue.updated_at || issue.last_seen_at;
      
      return {
        type: 'issue' as const,
        id: issue.github_issue_id,
        number: issue.number,
        title: issue.title,
        date: timeAgo(dateStr),
        status: issue.state === 'open' ? 'open' : 'closed' as 'open' | 'closed',
        url: issue.url,
      };
    });

    const combined = [...allPRs, ...allIssues].sort((a, b) => {
      // Get the most appropriate date for sorting
      let dateA = '';
      let dateB = '';
      
      if (a.type === 'pr') {
        const pr = prs.find(p => p.github_pr_id === a.id);
        if (pr) {
          if (pr.merged && pr.merged_at) {
            dateA = pr.merged_at;
          } else if (pr.state === 'closed' && pr.closed_at) {
            dateA = pr.closed_at;
          } else if (pr.state === 'open' && pr.created_at) {
            dateA = pr.created_at;
          } else {
            dateA = pr.updated_at || pr.last_seen_at || '';
          }
        }
      } else {
        const issue = issues.find(i => i.github_issue_id === a.id);
        if (issue) {
          dateA = issue.updated_at || issue.last_seen_at || '';
        }
      }
      
      if (b.type === 'pr') {
        const pr = prs.find(p => p.github_pr_id === b.id);
        if (pr) {
          if (pr.merged && pr.merged_at) {
            dateB = pr.merged_at;
          } else if (pr.state === 'closed' && pr.closed_at) {
            dateB = pr.closed_at;
          } else if (pr.state === 'open' && pr.created_at) {
            dateB = pr.created_at;
          } else {
            dateB = pr.updated_at || pr.last_seen_at || '';
          }
        }
      } else {
        const issue = issues.find(i => i.github_issue_id === b.id);
        if (issue) {
          dateB = issue.updated_at || issue.last_seen_at || '';
        }
      }
      
      return new Date(dateB).getTime() - new Date(dateA).getTime();
    });

    return combined;
  }, [prs, issues, timeAgo]);
  
  const [showAllIssues, setShowAllIssues] = useState(false);
  const [showAllActivity, setShowAllActivity] = useState(false);
  
  const displayedIssues = useMemo(() => {
    if (showAllIssues) return filteredIssues;
    return filteredIssues.slice(0, 5);
  }, [filteredIssues, showAllIssues]);
  
  const displayedActivity = useMemo(() => {
    if (showAllActivity) return recentPRs;
    return recentPRs.slice(0, 5);
  }, [recentPRs, showAllActivity]);

  const handleCopyLink = () => {
    navigator.clipboard.writeText(window.location.href);
    setCopiedLink(true);
    setTimeout(() => setCopiedLink(false), 2000);
  };

  return (
    <div className="flex gap-6 h-[calc(100vh-120px)] max-h-[calc(100vh-120px)]">
      {/* Left Sidebar */}
      <div className="w-[280px] flex-shrink-0 overflow-y-auto overflow-x-hidden space-y-6 scrollbar-hide">
        {/* Project Logo */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <div className="aspect-square rounded-[20px] overflow-hidden bg-gradient-to-br from-[#c9983a]/20 to-[#d4af37]/10">
            {isLoading ? (
              <SkeletonLoader className="w-full h-full" />
            ) : (
              <img 
                src={projectAvatar} 
                alt={repoName}
                className="w-full h-full object-cover"
                onError={(e) => {
                  // Fallback to GitHub default avatar if image fails to load
                  (e.target as HTMLImageElement).src = 'https://github.com/github.png?size=200';
                }}
              />
            )}
          </div>
        </div>

        {/* Community */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h3 className={`text-[16px] font-bold mb-4 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Community</h3>
          <div className="flex flex-col gap-2">
            {!!websiteUrl && (
            <a
                href={websiteUrl}
              target="_blank"
              rel="noopener noreferrer"
              className={`flex items-center gap-2 px-4 py-2.5 rounded-[12px] backdrop-blur-[20px] border border-white/25 hover:bg-white/[0.2] transition-all text-[13px] font-semibold ${
                theme === 'dark' ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
              }`}
            >
              <ExternalLink className="w-4 h-4" />
              Website
            </a>
            )}
            <a
              href={githubUrl}
              target="_blank"
              rel="noopener noreferrer"
              className={`flex items-center gap-2 px-4 py-2.5 rounded-[12px] backdrop-blur-[20px] border border-white/25 hover:bg-white/[0.2] transition-all text-[13px] font-semibold ${
                theme === 'dark' ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
              }`}
            >
              <ExternalLink className="w-4 h-4" />
              GitHub
            </a>
          </div>
        </div>

        {/* Languages */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h3 className={`text-[16px] font-bold mb-4 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Languages</h3>
          <div className="space-y-3">
            {isLoading ? (
              <>
                {[1, 2, 3].map((i) => (
                  <div key={i}>
                    <div className="flex items-center justify-between mb-1.5">
                      <SkeletonLoader className="h-4 w-20" />
                      <SkeletonLoader className="h-4 w-12" />
                    </div>
                    <SkeletonLoader className="h-2 w-full rounded-full" />
                  </div>
                ))}
              </>
            ) : languages.length ? (
              languages.map((lang, idx) => (
                <div key={idx}>
                  <div className="flex items-center justify-between mb-1.5">
                    <div className="flex items-center gap-2">
                      <LanguageIcon language={lang.name} className="w-4 h-4 flex-shrink-0" />
                      <span className={`text-[13px] font-semibold transition-colors ${
                        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                      }`}>{lang.name}</span>
                    </div>
                    <span className="text-[12px] font-bold text-[#c9983a]">{lang.displayLabel}</span>
                  </div>
                  <div className="h-2 rounded-full backdrop-blur-[15px] bg-white/[0.08] border border-white/15 overflow-hidden">
                    <div 
                      className="h-full bg-gradient-to-r from-[#c9983a] to-[#d4af37] rounded-full transition-all duration-500"
                      style={{ width: `${lang.barWidth}%` }}
                    />
                  </div>
                </div>
              ))
            ) : (
              <div className={`text-[13px] ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                No language data
              </div>
            )}
          </div>
        </div>

        {/* Ecosystems */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h3 className={`text-[16px] font-bold mb-4 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Ecosystems</h3>
          <div className="flex flex-wrap gap-2">
            {isLoading ? (
              <>
                <SkeletonLoader className="h-7 w-24 rounded-[8px]" />
                <SkeletonLoader className="h-7 w-20 rounded-[8px]" />
              </>
            ) : (project?.ecosystem_name ? [project.ecosystem_name] : []).length > 0 ? (
              (project?.ecosystem_name ? [project.ecosystem_name] : []).map((eco, idx) => (
                <span
                  key={idx}
                  className={`px-3 py-1.5 rounded-[8px] text-[12px] font-bold backdrop-blur-[20px] border border-white/25 transition-colors ${
                    theme === 'dark' ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
                  }`}
                >
                  {eco}
                </span>
              ))
            ) : (
              <span className={`text-[12px] ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                No ecosystems
              </span>
            )}
          </div>
        </div>

        {/* Categories */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h3 className={`text-[16px] font-bold mb-4 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Categories</h3>
          <div className="flex flex-wrap gap-2">
            {isLoading ? (
              <>
                <SkeletonLoader className="h-7 w-20 rounded-[8px]" />
                <SkeletonLoader className="h-7 w-16 rounded-[8px]" />
              </>
            ) : (project?.category ? [project.category] : []).length > 0 ? (
              (project?.category ? [project.category] : []).map((cat, idx) => (
                <span
                  key={idx}
                  className={`px-3 py-1.5 rounded-[8px] text-[12px] font-bold backdrop-blur-[20px] border border-white/25 transition-colors ${
                    theme === 'dark' ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
                  }`}
                >
                  {cat}
                </span>
              ))
            ) : (
              <span className={`text-[12px] ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                No categories
              </span>
            )}
          </div>
        </div>

        {/* Owner */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h3 className={`text-[16px] font-bold mb-4 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Owner</h3>
          <div className="space-y-3">
            {isLoading ? (
              <div className="flex items-center gap-3">
                <SkeletonLoader variant="circle" className="w-8 h-8" />
                <SkeletonLoader className="h-4 w-24" />
              </div>
            ) : ownerLogin ? (
              <div className="flex items-center gap-3">
                <img 
                  src={ownerAvatar} 
                  alt={ownerLogin}
                  className="w-8 h-8 rounded-full border-2 border-[#c9983a]/30"
                  onError={(e) => {
                    (e.target as HTMLImageElement).src = 'https://github.com/github.png?size=80';
                  }}
                />
                <span className={`text-[13px] font-semibold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{ownerLogin}</span>
              </div>
            ) : (
              <div className={`text-[13px] ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                Unknown
              </div>
            )}
          </div>
        </div>

        {/* Contributors */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-6 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h3 className={`text-[16px] font-bold mb-4 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Contributors</h3>
          <div className="flex items-center gap-2 mb-3">
            {isLoading ? (
              <div className="flex -space-x-2">
                {[1, 2, 3, 4, 5].map((i) => (
                  <SkeletonLoader key={i} variant="circle" className="w-10 h-10 border-2 border-[#c9983a]/30" />
                ))}
              </div>
            ) : (
              <div className="flex -space-x-2">
                {contributors.slice(0, 5).map((contributor) => (
                  <img 
                    key={contributor.name}
                    src={contributor.avatar} 
                    alt={contributor.name}
                    className="w-10 h-10 rounded-full border-2 border-[#c9983a]/30 hover:z-10 transition-transform hover:scale-110"
                    onError={(e) => {
                      (e.target as HTMLImageElement).src = 'https://github.com/github.png?size=80';
                    }}
                  />
                ))}
                {project?.contributors_count && project.contributors_count > 5 && (
                  <div className={`w-10 h-10 rounded-full border-2 border-[#c9983a]/30 flex items-center justify-center text-[12px] font-bold transition-colors ${
                    theme === 'dark' 
                      ? 'bg-white/[0.12] text-[#f5f5f5]' 
                      : 'bg-white/[0.15] text-[#2d2820]'
                  }`}>
                    +{project.contributors_count - 5}
                  </div>
                )}
              </div>
            )}
          </div>
          {isLoading ? (
            <SkeletonLoader className="h-4 w-full" />
          ) : (
            <p className={`text-[12px] transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              {project?.contributors_count
                ? `${project.contributors_count} ${project.contributors_count === 1 ? 'contributor' : 'contributors'}`
                : 'No contributors yet'}
            </p>
          )}
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 space-y-6 overflow-y-auto scrollbar-hide">
        {/* Back Button */}
        {(onBack || onClose) && (
          <button
            onClick={onBack || onClose}
            className={`flex items-center gap-2 px-4 py-2.5 rounded-[12px] backdrop-blur-[40px] border hover:bg-white/[0.2] transition-all ${
              theme === 'dark'
                ? 'bg-white/[0.12] border-white/20 text-[#f5f5f5]'
                : 'bg-white/[0.12] border-white/20 text-[#2d2820]'
            }`}
          >
            <ArrowLeft className="w-5 h-5" />
            <span className="font-semibold text-[14px]">Back to Browse</span>
          </button>
        )}

        {/* Header */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-8 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <div className="flex items-start justify-between mb-4">
            <div className="flex-1">
              {isLoading ? (
                <>
                  <SkeletonLoader className="h-9 w-64 mb-3" />
                  <SkeletonLoader className="h-5 w-full max-w-md" />
                </>
              ) : (
                <>
                  <h1 className={`text-[32px] font-bold mb-2 transition-colors ${
                    theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>{repoName}</h1>
                  <p className={`text-[15px] transition-colors ${
                    theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}>{description || project?.github_full_name || 'No description available'}</p>
                </>
              )}
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => githubUrl && window.open(githubUrl, '_blank')}
                className={`p-3 rounded-[12px] backdrop-blur-[20px] border border-white/25 hover:bg-white/[0.2] transition-all ${
                  theme === 'dark' ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
                }`}
              >
                <ExternalLink className="w-5 h-5" />
              </button>
              <button
                onClick={handleCopyLink}
                className={`p-3 rounded-[12px] backdrop-blur-[20px] border border-white/25 hover:bg-white/[0.2] transition-all ${
                  theme === 'dark' ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
                }`}
              >
                <Copy className={`w-5 h-5 ${copiedLink ? 'text-[#c9983a]' : ''}`} />
              </button>
              <button className="px-6 py-3 rounded-[14px] bg-gradient-to-br from-[#c9983a] to-[#d4af37] text-white font-bold text-[14px] hover:opacity-90 transition-all">
                Contribute now
              </button>
            </div>
          </div>
        </div>

        {/* Overview */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-8 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.08] border-white/10'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <div className="flex items-center justify-between mb-6">
            <h2 className={`text-[18px] font-bold flex items-center gap-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              <span className="text-[#c9983a]">✦</span>
              Overview
            </h2>
          </div>
          {isLoading ? (
            <div className="space-y-3">
              <SkeletonLoader className="h-4 w-full" />
              <SkeletonLoader className="h-4 w-full" />
              <SkeletonLoader className="h-4 w-3/4" />
            </div>
          ) : project?.readme ? (
            <div className="prose prose-sm max-w-none [&_pre]:my-4 [&_pre]:overflow-x-auto [&_pre]:rounded-[12px] [&_pre]:p-4 [&_pre_code]:!p-0 [&_pre_code]:!bg-transparent [&_pre_code]:!border-0 [&_pre_code]:!text-inherit [&_pre_code]:block">
              <OverviewMarkdown readme={project.readme} theme={theme} />
            </div>
          ) : description ? (
            <p className={`text-[15px] leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#4a3f2f]'
            }`}>
              {description}
            </p>
          ) : (
            <p className={`text-[15px] leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#4a3f2f]'
            }`}>
              No description available. Visit the <a 
                href={githubUrl} 
                target="_blank" 
                rel="noopener noreferrer"
                className={`font-semibold hover:underline transition-colors ${
                  theme === 'dark' 
                    ? 'text-[#f5c563] hover:text-[#ffd700]' 
                    : 'text-[#b8872f] hover:text-[#8b6f3a]'
                }`}
              >GitHub repository</a> for more information.
            </p>
          )}
        </div>

        {/* Issues */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-8 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h2 className={`text-[18px] font-bold mb-6 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Issues</h2>

          {/* Issue Tabs */}
          <div className="flex flex-wrap items-center gap-2 mb-6">
            {isLoading ? (
              <>
                <SkeletonLoader className="h-9 w-32 rounded-[10px]" />
                <SkeletonLoader className="h-9 w-28 rounded-[10px]" />
                <SkeletonLoader className="h-9 w-24 rounded-[10px]" />
              </>
            ) : (
              issueTabs.map((tab) => (
                <button
                  key={tab.id}
                  onClick={() => setActiveIssueTab(tab.id)}
                  className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                    activeIssueTab === tab.id
                      ? 'bg-[#c9983a] text-white'
                      : `backdrop-blur-[20px] border border-white/25 hover:bg-white/[0.2] ${
                          theme === 'dark' ? 'bg-white/[0.08] text-[#f5f5f5]' : 'bg-white/[0.08] text-[#2d2820]'
                        }`
                  }`}
                >
                  {tab.label} ({tab.count})
                </button>
              ))
            )}
          </div>

          {/* Issue List */}
          <div className="space-y-4">
            {isLoading ? (
              <>
                {[1, 2, 3, 4, 5].map((i) => (
                  <div
                    key={i}
                    className={`p-6 rounded-[16px] backdrop-blur-[25px] border border-white/25 ${
                      theme === 'dark' ? 'bg-white/[0.08]' : 'bg-white/[0.08]'
                    }`}
                  >
                    <SkeletonLoader className="h-5 w-3/4 mb-3" />
                    <div className="flex items-center gap-2 mb-2">
                      <SkeletonLoader className="h-6 w-16 rounded-[6px]" />
                      <SkeletonLoader className="h-6 w-20 rounded-[6px]" />
                    </div>
                    <div className="flex items-center justify-between">
                      <SkeletonLoader className="h-4 w-24" />
                      <SkeletonLoader className="h-4 w-32" />
                    </div>
                  </div>
                ))}
              </>
            ) : (
              <>
                {displayedIssues.map((issue) => (
              <div
                key={issue.github_issue_id}
                className={`p-6 rounded-[16px] backdrop-blur-[25px] border border-white/25 hover:bg-white/[0.15] transition-all cursor-pointer ${
                  theme === 'dark' ? 'bg-white/[0.08]' : 'bg-white/[0.08]'
                }`}
                onClick={() => {
                  if (!projectId) return;
                  onIssueClick?.(String(issue.github_issue_id), String(projectId));
                }}
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-start gap-3 flex-1">
                    {/* Issue Icon - Same as maintainers */}
                    <div className="w-5 h-5 rounded-full bg-gradient-to-br from-[#c9983a]/50 to-[#d4af37]/40 flex items-center justify-center border border-[#c9983a]/40 flex-shrink-0 mt-0.5">
                      <Circle className="w-2.5 h-2.5 text-white fill-white" strokeWidth={0} />
                    </div>
                    <h3 className={`text-[15px] font-bold transition-colors ${
                      theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>{issue.title}</h3>
                  </div>
                </div>
                <div className="flex items-center justify-between ml-8">
                  <div className="flex items-center gap-2">
                    {(Array.isArray(issue.labels) ? issue.labels : [])
                      .map((l) => labelName(l))
                      .filter(Boolean)
                      .slice(0, 4)
                      .map((tag, idx) => (
                      <span
                        key={idx}
                        className={`px-3 py-1 rounded-[6px] text-[11px] font-bold backdrop-blur-[20px] border border-white/20 transition-colors ${
                          theme === 'dark' ? 'bg-white/[0.1] text-[#d4d4d4]' : 'bg-white/[0.1] text-[#4a3f2f]'
                        }`}
                      >
                        {String(tag)}
                      </span>
                    ))}
                  </div>
                  <div className="flex items-center gap-3">
                    <span className={`text-[12px] transition-colors ${
                      theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>{timeAgo(issue.updated_at || issue.last_seen_at)}</span>
                    <div className="flex items-center gap-2">
                      <img 
                        src={`https://github.com/${issue.author_login}.png?size=40`} 
                        alt={issue.author_login}
                        className="w-5 h-5 rounded-full border border-[#c9983a]/30"
                      />
                      <span className={`text-[12px] font-semibold transition-colors ${
                        theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                      }`}>By {issue.author_login}</span>
                    </div>
                    <div className={`flex items-center gap-1.5 px-2 py-1 rounded-[6px] backdrop-blur-[20px] border border-white/20 transition-colors ${
                      theme === 'dark' ? 'bg-white/[0.1]' : 'bg-white/[0.1]'
                    }`}>
                      <img 
                        src={projectAvatar} 
                        alt={repoName}
                        className="w-4 h-4 rounded-full border border-[#c9983a]/30 object-cover"
                        onError={(e) => {
                          (e.target as HTMLImageElement).src = 'https://github.com/github.png?size=40';
                        }}
                      />
                      <span className={`text-[11px] font-bold transition-colors ${
                        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#4a3f2f]'
                      }`}>
                        {repoName}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
                ))}
                {!showAllIssues && filteredIssues.length > 5 && (
                  <button
                    onClick={() => setShowAllIssues(true)}
                    className={`w-full py-3 rounded-[12px] border transition-all ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/20 text-[#f5f5f5] hover:bg-white/[0.12]'
                        : 'bg-white/[0.12] border-white/25 text-[#2d2820] hover:bg-white/[0.15]'
                    }`}
                  >
                    <span className="text-[14px] font-semibold">View All ({filteredIssues.length} issues)</span>
                  </button>
                )}
              </>
            )}
            {!isLoading && filteredIssues.length === 0 && (
              <div className={`p-6 rounded-[16px] border text-center ${
                theme === 'dark' ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]' : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
              }`}>
                <p className="text-[14px] font-semibold">No issues found</p>
              </div>
            )}
          </div>
        </div>

        {/* Recent Activity */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border p-8 transition-colors ${
          theme === 'dark'
            ? 'bg-white/[0.12] border-white/20'
            : 'bg-white/[0.12] border-white/20'
        }`}>
          <h2 className={`text-[18px] font-bold mb-6 transition-colors ${
            theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}>Recent Activity</h2>
          <div className="space-y-3">
            {isLoading ? (
              <>
                {[1, 2, 3, 4, 5].map((i) => (
                  <div
                    key={i}
                    className={`flex items-center justify-between p-4 rounded-[12px] backdrop-blur-[20px] border border-white/20 ${
                      theme === 'dark' ? 'bg-white/[0.05]' : 'bg-white/[0.05]'
                    }`}
                  >
                    <div className="flex items-center gap-3 flex-1">
                      <SkeletonLoader className="h-6 w-12 rounded-[6px]" />
                      <SkeletonLoader className="h-4 w-48" />
                    </div>
                    <SkeletonLoader className="h-4 w-20" />
                  </div>
                ))}
              </>
            ) : (
              <>
                {displayedActivity.map((activity, idx) => {
                  // Determine colors based on type and status
                  let iconBgColor = '';
                  let iconColor = '';
                  let badgeBgColor = '';
                  let badgeBorderColor = '';
                  
                  if (activity.type === 'pr') {
                    if (activity.status === 'merged') {
                      // Merged PR: Purple
                      iconBgColor = 'bg-[#8b5cf6]/50';
                      iconColor = 'text-[#8b5cf6]';
                      badgeBgColor = 'bg-[#8b5cf6]/50';
                      badgeBorderColor = 'border-[#8b5cf6]/40';
                    } else if (activity.status === 'open') {
                      // Open PR: Green
                      iconBgColor = 'bg-[#22c55e]/50';
                      iconColor = 'text-[#22c55e]';
                      badgeBgColor = 'bg-[#22c55e]/50';
                      badgeBorderColor = 'border-[#22c55e]/40';
                    } else {
                      // Closed PR: Gray/Red
                      iconBgColor = theme === 'dark' ? 'bg-white/[0.15]' : 'bg-white/[0.2]';
                      iconColor = theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]';
                      badgeBgColor = theme === 'dark' ? 'bg-white/[0.15]' : 'bg-white/[0.2]';
                      badgeBorderColor = theme === 'dark' ? 'border-white/20' : 'border-white/30';
                    }
                  } else {
                    // Issue
                    if (activity.status === 'open') {
                      // Open Issue: Golden/Yellow
                      iconBgColor = 'bg-gradient-to-br from-[#c9983a]/50 to-[#d4af37]/40';
                      iconColor = 'text-white';
                      badgeBgColor = 'bg-gradient-to-br from-[#c9983a]/50 to-[#d4af37]/40';
                      badgeBorderColor = 'border-[#c9983a]/40';
                    } else {
                      // Closed Issue: Gray
                      iconBgColor = theme === 'dark' ? 'bg-white/[0.15]' : 'bg-white/[0.2]';
                      iconColor = theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]';
                      badgeBgColor = theme === 'dark' ? 'bg-white/[0.15]' : 'bg-white/[0.2]';
                      badgeBorderColor = theme === 'dark' ? 'border-white/20' : 'border-white/30';
                    }
                  }
                  
                  const handleClick = () => {
                    if (activity.url) {
                      window.open(activity.url, '_blank', 'noopener,noreferrer');
                    }
                  };
                  
                  return (
                    <div
                      key={idx}
                      onClick={handleClick}
                      className={`flex items-center justify-between p-4 rounded-[12px] backdrop-blur-[20px] border border-white/20 hover:bg-white/[0.15] transition-all cursor-pointer ${
                        theme === 'dark' ? 'bg-white/[0.05]' : 'bg-white/[0.05]'
                      }`}
                    >
                      <div className="flex items-center gap-3">
                        {activity.type === 'issue' ? (
                          // Issue Icon
                          <div className={`w-5 h-5 rounded-full ${iconBgColor} flex items-center justify-center border ${badgeBorderColor} flex-shrink-0`}>
                            <Circle className={`w-2.5 h-2.5 ${iconColor} ${activity.status === 'open' ? 'fill-white' : 'fill-current'}`} strokeWidth={0} />
                          </div>
                        ) : (
                          // PR Icon
                          <div className={`w-5 h-5 rounded-full ${iconBgColor} flex items-center justify-center border ${badgeBorderColor} flex-shrink-0`}>
                            <GitPullRequest className={`w-3 h-3 ${iconColor}`} strokeWidth={2.5} />
                          </div>
                        )}
                        <div className={`px-2 py-1 rounded-[6px] ${badgeBgColor} border ${badgeBorderColor}`}>
                          <span className={`text-[11px] font-bold ${
                            activity.status === 'open' && activity.type === 'issue' ? 'text-white' :
                            activity.status === 'merged' ? 'text-white' :
                            activity.status === 'open' && activity.type === 'pr' ? 'text-white' :
                            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                          }`}>#{activity.number}</span>
                        </div>
                        <span className={`text-[14px] font-semibold transition-colors ${
                          theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>{activity.title}</span>
                      </div>
                      <span className={`text-[13px] transition-colors ${
                        theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                      }`}>{activity.date}</span>
                    </div>
                  );
                })}
                {!showAllActivity && recentPRs.length > 5 && (
                  <button
                    onClick={() => setShowAllActivity(true)}
                    className={`w-full py-3 rounded-[12px] border transition-all ${
                      theme === 'dark'
                        ? 'bg-white/[0.08] border-white/20 text-[#f5f5f5] hover:bg-white/[0.12]'
                        : 'bg-white/[0.12] border-white/25 text-[#2d2820] hover:bg-white/[0.15]'
                    }`}
                  >
                    <span className="text-[14px] font-semibold">View All ({recentPRs.length} activities)</span>
                  </button>
                )}
              </>
            )}
            {!isLoading && recentPRs.length === 0 && (
              <div className={`p-6 rounded-[16px] border text-center ${
                theme === 'dark' ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]' : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
              }`}>
                <p className="text-[14px] font-semibold">No recent pull requests</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
