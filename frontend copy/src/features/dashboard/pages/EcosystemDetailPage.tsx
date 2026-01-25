import { useState } from 'react';
import { ChevronRight, ExternalLink, Users, FolderGit2, AlertCircle, GitPullRequest } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { ProjectCard, Project } from '../components/ProjectCard';
import { SearchWithFilter } from '../components/SearchWithFilter';

interface EcosystemDetailPageProps {
  ecosystemId: string;
  ecosystemName: string;
  onBack: () => void;
  onProjectClick?: (id: string) => void;
}

export function EcosystemDetailPage({ ecosystemId, ecosystemName, onBack, onProjectClick }: EcosystemDetailPageProps) {
  const { theme } = useTheme();
  const [activeTab, setActiveTab] = useState<'overview' | 'projects' | 'community'>('overview');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedLanguages, setSelectedLanguages] = useState<string[]>([]);
  const [selectedCategories, setSelectedCategories] = useState<string[]>([]);

  // Mock data - in real app this would come from API
  const ecosystemData = {
    name: ecosystemName,
    logo: ecosystemName.charAt(0).toUpperCase(),
    description: 'Projects building decentralized protocols, tooling, and infrastructure.',
    languages: [
      { name: 'Q#', percentage: 0 },
      { name: 'M', percentage: 0 },
    ],
    links: [
      { label: 'Official Website', url: 'web3.ecosystem.example', icon: 'website' },
      { label: 'Discord Community', url: 'discord.gg', icon: 'discord' },
      { label: 'Twitter', url: 'twitter.com', icon: 'twitter' },
    ],
    stats: {
      activeContributors: { value: 260, change: '+12' },
      activeProjects: { value: 24, change: '+3' },
      availableIssues: { value: 180, change: '-5' },
      mergedPullRequests: { value: 920, change: '+45' },
    },
    about: `The ${ecosystemName} ecosystem represents a paradigm shift towards decentralized applications, protocols, and infrastructure. This ecosystem brings together innovative projects that are building the next generation of the internet.`,
    keyAreas: [
      { title: 'Blockchain Protocols', description: 'Core blockchain technologies and consensus mechanisms' },
      { title: 'DeFi (Decentralized Finance)', description: 'Financial applications built on blockchain' },
      { title: 'NFTs & Digital Assets', description: 'Tokenization and digital ownership' },
      { title: `${ecosystemName} Infrastructure`, description: 'Wallets, nodes, and developer tools' },
      { title: 'DAOs', description: 'Decentralized autonomous organizations' },
    ],
    technologies: [
      'TypeScript for smart contract development and tooling',
      'Rust for high-performance blockchain infrastructure',
      'Solidity for Ethereum smart contracts',
      'JavaScript/TypeScript for dApp frontends',
    ],
  };

  const projectsData: Project[] = [
    {
      id: '1',
      name: 'React Ecosystem',
      icon: '⚛️',
      stars: '4.9M',
      forks: '2.6M',
      contributors: 45,
      openIssues: 12,
      prs: 0,
      description: 'A modern React framework for building user interfaces with TypeScript support',
      tags: ['TypeScript', 'good first issue'],
      color: 'from-blue-500 to-cyan-500',
    },
    {
      id: '2',
      name: 'Nextjs Framework',
      icon: '▲',
      stars: '120K',
      forks: '24K',
      contributors: 78,
      openIssues: 20,
      prs: 0,
      description: 'The React framework for production with server-side rendering',
      tags: ['Frontend'],
      color: 'from-purple-500 to-pink-500',
    },
    {
      id: '3',
      name: 'Vue.js',
      icon: 'V',
      stars: '214K',
      forks: '36K',
      contributors: 94,
      openIssues: 8,
      prs: 0,
      description: 'Progressive JavaScript framework for building user interfaces',
      tags: ['Framework'],
      color: 'from-green-500 to-emerald-500',
    },
    {
      id: '4',
      name: 'Angular',
      icon: 'A',
      stars: '93.5K',
      forks: '24K',
      contributors: 120,
      openIssues: 35,
      prs: 0,
      description: 'A platform and framework for building single-page client applications',
      tags: ['Frontend', 'TypeScript'],
      color: 'from-red-500 to-pink-500',
    },
  ];

  const filteredProjects = projectsData.filter(project =>
    project.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    project.description.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const isDark = theme === 'dark';

  return (
    <div className="h-full overflow-y-auto">
      {/* Breadcrumb Navigation */}
      <div className="mb-6 flex items-center gap-2 ml-12">
        <button
          onClick={onBack}
          className={`text-[14px] font-semibold transition-colors ${
            isDark 
              ? 'text-[#d4d4d4] hover:text-[#c9983a]' 
              : 'text-[#7a6b5a] hover:text-[#a67c2a]'
          }`}
        >
          Ecosystems
        </button>
        <ChevronRight className={`w-4 h-4 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`} />
        <span className={`text-[14px] font-bold transition-colors ${
          isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
        }`}>
          {ecosystemName}
        </span>
        <ChevronRight className={`w-4 h-4 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`} />
        <span className={`text-[14px] font-semibold transition-colors ${
          isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'
        }`}>
          Overview
        </span>
      </div>

      <div className="flex gap-6">
        {/* Left Sidebar - Ecosystem Info */}
        <div className="flex-[1] flex-shrink-0 space-y-6">
          {/* Ecosystem Header */}
          <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-6">
            <div className="flex items-center gap-4 mb-4">
              <div className="w-16 h-16 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center">
                <span className="text-[24px] font-bold text-white">{ecosystemData.logo}</span>
              </div>
              <div>
                <h1 className={`text-[20px] font-bold transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  {ecosystemData.name} Ecosystem
                </h1>
                <div className="flex items-center gap-4 mt-1">
                  <div className="flex items-center gap-2">
                    <Users className={`w-3.5 h-3.5 ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`} />
                    <span className={`text-[13px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`}>
                      420
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <FolderGit2 className={`w-3.5 h-3.5 ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`} />
                    <span className={`text-[13px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#8b6914]'}`}>
                      {ecosystemData.stats.activeProjects.value}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Description */}
          <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-6">
            <h2 className={`text-[16px] font-bold mb-3 transition-colors ${
              isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Description
            </h2>
            <p className={`text-[13px] leading-relaxed transition-colors ${
              isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              {ecosystemData.description}
            </p>
          </div>

          {/* Languages */}
          <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-6">
            <h2 className={`text-[16px] font-bold mb-3 transition-colors ${
              isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Languages
            </h2>
            <div className="flex gap-3">
              {ecosystemData.languages.map((lang, idx) => (
                <div
                  key={idx}
                  className="px-3 py-1.5 rounded-[8px] backdrop-blur-[20px] border border-white/25 bg-white/[0.08]"
                >
                  <span className="text-[12px] font-semibold text-[#c9983a]">{lang.name}</span>
                  <span className={`ml-2 text-[11px] ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                    {lang.percentage}%
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Links */}
          <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-6">
            <h2 className={`text-[16px] font-bold mb-4 transition-colors ${
              isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>
              Links
            </h2>
            <div className="space-y-3">
              {ecosystemData.links.map((link, idx) => (
                <a
                  key={idx}
                  href={`https://${link.url}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center justify-between p-3 rounded-[12px] backdrop-blur-[20px] border border-white/25 bg-white/[0.08] hover:bg-white/[0.15] transition-all group"
                >
                  <div>
                    <div className={`text-[13px] font-semibold transition-colors ${
                      isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>
                      {link.label}
                    </div>
                    <div className={`text-[11px] transition-colors ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      {link.url}
                    </div>
                  </div>
                  <ExternalLink className="w-4 h-4 text-[#c9983a] opacity-0 group-hover:opacity-100 transition-opacity" />
                </a>
              ))}
            </div>
          </div>
        </div>

        {/* Main Content Area */}
        <div className="flex-[3]">
          {/* Tabs */}
          <div className="flex items-center gap-3 mb-6">
            <button
              onClick={() => setActiveTab('overview')}
              className={`px-5 py-2.5 rounded-[12px] text-[14px] font-semibold transition-all ${
                activeTab === 'overview'
                  ? 'bg-[#c9983a] text-white shadow-lg'
                  : isDark
                    ? 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#d4d4d4] hover:bg-white/[0.15]'
                    : 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#7a6b5a] hover:bg-white/[0.15]'
              }`}
            >
              Overview
            </button>
            <button
              onClick={() => setActiveTab('projects')}
              className={`px-5 py-2.5 rounded-[12px] text-[14px] font-semibold transition-all ${
                activeTab === 'projects'
                  ? 'bg-[#c9983a] text-white shadow-lg'
                  : isDark
                    ? 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#d4d4d4] hover:bg-white/[0.15]'
                    : 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#7a6b5a] hover:bg-white/[0.15]'
              }`}
            >
              Projects
            </button>
            <button
              onClick={() => setActiveTab('community')}
              className={`px-5 py-2.5 rounded-[12px] text-[14px] font-semibold transition-all ${
                activeTab === 'community'
                  ? 'bg-[#c9983a] text-white shadow-lg'
                  : isDark
                    ? 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#d4d4d4] hover:bg-white/[0.15]'
                    : 'backdrop-blur-[40px] bg-white/[0.12] border border-white/20 text-[#7a6b5a] hover:bg-white/[0.15]'
              }`}
            >
              Community
            </button>
          </div>

          {activeTab === 'overview' && (
            <div className="space-y-6">
              {/* Stats Grid */}
              <div className="grid grid-cols-4 gap-4">
                <div className="backdrop-blur-[40px] rounded-[16px] border bg-white/[0.12] border-white/20 p-5">
                  <div className="flex items-center gap-2 mb-2">
                    <Users className={`w-4 h-4 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[11px] font-bold uppercase tracking-wide ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Active Contributors
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {ecosystemData.stats.activeContributors.value}
                    </span>
                  </div>
                </div>

                <div className="backdrop-blur-[40px] rounded-[16px] border bg-white/[0.12] border-white/20 p-5">
                  <div className="flex items-center gap-2 mb-2">
                    <FolderGit2 className={`w-4 h-4 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[11px] font-bold uppercase tracking-wide ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Active Projects
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {ecosystemData.stats.activeProjects.value}
                    </span>
                  </div>
                </div>

                <div className="backdrop-blur-[40px] rounded-[16px] border bg-white/[0.12] border-white/20 p-5">
                  <div className="flex items-center gap-2 mb-2">
                    <AlertCircle className={`w-4 h-4 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[11px] font-bold uppercase tracking-wide ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Available Issues
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {ecosystemData.stats.availableIssues.value}
                    </span>
                  </div>
                </div>

                <div className="backdrop-blur-[40px] rounded-[16px] border bg-white/[0.12] border-white/20 p-5">
                  <div className="flex items-center gap-2 mb-2">
                    <GitPullRequest className={`w-4 h-4 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`} />
                    <span className={`text-[11px] font-bold uppercase tracking-wide ${
                      isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`}>
                      Merged Pull Requests
                    </span>
                  </div>
                  <div className="flex items-end gap-2">
                    <span className={`text-[28px] font-bold ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>
                      {ecosystemData.stats.mergedPullRequests.value}
                    </span>
                  </div>
                </div>
              </div>

              {/* About Section */}
              <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-6">
                <h2 className={`text-[18px] font-bold mb-4 transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  About {ecosystemName}
                </h2>
                <p className={`text-[14px] leading-relaxed transition-colors ${
                  isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                  {ecosystemData.about}
                </p>
              </div>

              {/* Key Areas */}
              <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-6">
                <h2 className={`text-[18px] font-bold mb-4 transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  Key Areas
                </h2>
                <ul className="space-y-3">
                  {ecosystemData.keyAreas.map((area, idx) => (
                    <li key={idx} className="flex gap-3">
                      <span className={`mt-1 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>•</span>
                      <div>
                        <span className={`font-bold text-[14px] ${
                          isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}>
                          {area.title}:
                        </span>{' '}
                        <span className={`text-[14px] ${
                          isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                        }`}>
                          {area.description}
                        </span>
                      </div>
                    </li>
                  ))}
                </ul>
              </div>

              {/* Technologies */}
              <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-6">
                <h2 className={`text-[18px] font-bold mb-4 transition-colors ${
                  isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>
                  Technologies
                </h2>
                <p className={`text-[13px] mb-3 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                  Supported technologies for ecosystem projects:
                </p>
                <ul className="space-y-2">
                  {ecosystemData.technologies.map((tech, idx) => (
                    <li key={idx} className="flex gap-3">
                      <span className={`mt-1 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2a]'}`}>•</span>
                      <span className={`text-[14px] ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                        {tech}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          )}

          {activeTab === 'projects' && (
            <div className="space-y-6">
              {/* Search and Filter */}
              <SearchWithFilter
                searchPlaceholder="Search"
                searchValue={searchQuery}
                onSearchChange={setSearchQuery}
                filterSections={[
                  {
                    title: 'Languages',
                    hasSearch: true,
                    options: [
                      { label: 'TypeScript', value: 'typescript' },
                      { label: 'JavaScript', value: 'javascript' },
                      { label: 'Python', value: 'python' },
                      { label: 'Rust', value: 'rust' },
                    ],
                    selectedValues: selectedLanguages,
                    onToggle: (value) => {
                      setSelectedLanguages(prev =>
                        prev.includes(value)
                          ? prev.filter(v => v !== value)
                          : [...prev, value]
                      );
                    },
                  },
                  {
                    title: 'Categories',
                    hasSearch: false,
                    options: [
                      { label: 'Frontend', value: 'frontend' },
                      { label: 'Backend', value: 'backend' },
                      { label: 'Full Stack', value: 'fullstack' },
                      { label: 'DevOps', value: 'devops' },
                    ],
                    selectedValues: selectedCategories,
                    onToggle: (value) => {
                      setSelectedCategories(prev =>
                        prev.includes(value)
                          ? prev.filter(v => v !== value)
                          : [...prev, value]
                      );
                    },
                  },
                ]}
                onReset={() => {
                  setSearchQuery('');
                  setSelectedLanguages([]);
                  setSelectedCategories([]);
                }}
              />

              {/* Projects Grid */}
              <div className="grid grid-cols-2 gap-5">
                {filteredProjects.map(project => (
                  <ProjectCard key={project.id} project={project} onClick={onProjectClick} />
                ))}
              </div>
            </div>
          )}

          {activeTab === 'community' && (
            <div className="backdrop-blur-[40px] rounded-[24px] border bg-white/[0.12] border-white/20 p-8 text-center">
              <Users className={`w-12 h-12 mx-auto mb-4 ${
                isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`} />
              <p className={`text-[14px] ${
                isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>
                Community view coming soon
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}