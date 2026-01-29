import { useState, useEffect } from 'react';
import { Search, ArrowRight, X, Loader, Folder, AlertTriangle, User } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';
import { useSearch } from '../hooks/useSearch';
import { Project, Issue, Contributor } from '../types/search';

interface SearchModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const Spinner = () => (
  <div className="flex justify-center items-center p-8">
    <Loader className="w-8 h-8 animate-spin text-[#c9983a]" />
  </div>
);

const SearchResultItem = ({
  icon,
  title,
  subtitle,
  darkTheme,
}: {
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  darkTheme: boolean;
}) => (
  <div
    className={`group flex items-center justify-between px-5 py-4 rounded-[16px] transition-all hover:scale-[1.02] ${
      darkTheme
        ? 'bg-[#2d2820]/40 hover:bg-[#2d2820]/60 border border-white/5 hover:border-white/10'
        : 'bg-white/40 hover:bg-white/60 border border-black/5 hover:border-black/10'
    }`}
    style={{ backdropFilter: 'blur(20px)' }}
  >
    <div className="flex items-center">
      <div className="mr-4">{icon}</div>
      <div>
        <div className={`text-left text-[14px] transition-colors ${darkTheme ? 'text-[#d4c5b0]' : 'text-[#6b5d4d]'}`}>
          {title}
        </div>
        <div className={`text-left text-xs transition-colors ${darkTheme ? 'text-[#b8a898]/70' : 'text-[#6b5d4d]/70]'}`}>
          {subtitle}
        </div>
      </div>
    </div>
    <ArrowRight
      className={`w-4 h-4 ml-3 flex-shrink-0 transition-all group-hover:translate-x-1 ${
        darkTheme ? 'text-[#c9983a]' : 'text-[#a2792c]'
      }`}
    />
  </div>
);

const SearchResults = ({ darkTheme }: { darkTheme: boolean }) => {
  const { results, isLoading } = useSearch();

  if (isLoading) {
    return <Spinner />;
  }

  if (!results || (results.projects.length === 0 && results.issues.length === 0 && results.contributors.length === 0)) {
    return <div className={`text-center py-8 ${darkTheme ? 'text-white/60' : 'text-black/60'}`}>No results found.</div>;
  }

  return (
    <div>
      {results.projects.length > 0 && (
        <div className="mb-4">
          <h2 className={`font-semibold mb-2 transition-colors ${darkTheme ? 'text-[#f5efe5]' : 'text-[#2d2820]'}`}>
            Projects
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {results.projects.map((project: Project) => (
              <SearchResultItem
                key={project.id}
                icon={<Folder className="w-5 h-5 text-[#c9983a]" />}
                title={project.name}
                subtitle={project.description}
                darkTheme={darkTheme}
              />
            ))}
          </div>
        </div>
      )}
      {results.issues.length > 0 && (
        <div className="mb-4">
          <h2 className={`font-semibold mb-2 transition-colors ${darkTheme ? 'text-[#f5efe5]' : 'text-[#2d2820]'}`}>
            Issues
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {results.issues.map((issue: Issue) => (
              <SearchResultItem
                key={issue.id}
                icon={<AlertTriangle className="w-5 h-5 text-[#c9983a]" />}
                title={issue.title}
                subtitle={issue.project}
                darkTheme={darkTheme}
              />
            ))}
          </div>
        </div>
      )}
      {results.contributors.length > 0 && (
        <div>
          <h2 className={`font-semibold mb-2 transition-colors ${darkTheme ? 'text-[#f5efe5]' : 'text-[#2d2820]'}`}>
            Contributors
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {results.contributors.map((contributor: Contributor) => (
              <SearchResultItem
                key={contributor.id}
                icon={<User className="w-5 h-5 text-[#c9983a]" />}
                title={contributor.name}
                subtitle={contributor.githubHandle}
                darkTheme={darkTheme}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export function SearchModal({ isOpen, onClose }: SearchModalProps) {
  const { theme } = useTheme();
  const { searchQuery, setSearchQuery, isLoading } = useSearch();
  const darkTheme = theme === 'dark';

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      document.body.style.overflow = 'hidden';
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = 'unset';
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[200] flex items-start justify-center pt-[10vh]">
      {/* Backdrop */}
      <div
        className={`absolute inset-0 transition-colors ${
          darkTheme ? 'bg-black/70' : 'bg-black/40'
        }`}
        onClick={onClose}
        style={{ backdropFilter: 'blur(12px)' }}
      />

      {/* Modal Content */}
      <div
        className={`relative w-full max-w-[900px] mx-4 rounded-[32px] border shadow-2xl transition-colors ${
          darkTheme
            ? 'bg-[#1a1512]/95 border-white/10'
            : 'bg-white/95 border-white/30'
        }`}
        style={{ backdropFilter: 'blur(90px)' }}
      >
        {/* Close Button */}
        <button
          onClick={onClose}
          className={`absolute top-6 right-6 w-10 h-10 rounded-full flex items-center justify-center transition-all hover:scale-105 ${
            darkTheme
              ? 'bg-white/5 hover:bg-white/10 text-white/60 hover:text-white/80'
              : 'bg-black/5 hover:bg-black/10 text-black/60 hover:text-black/80'
          }`}
        >
          <X className="w-5 h-5" />
        </button>

        <div className="p-12">
          {/* Search Input */}
          <div
            className={`relative h-[64px] rounded-[32px] mb-12 transition-colors ${
              darkTheme
                ? 'bg-[#2d2820]/60 border border-white/10'
                : 'bg-white/60 border border-black/10'
            }`}
            style={{ backdropFilter: 'blur(40px)' }}
          >
            <div className="absolute inset-0 flex items-center px-6">
              <Search
                className={`w-5 h-5 mr-4 flex-shrink-0 transition-colors ${
                  darkTheme ? 'text-white/50' : 'text-black/50'
                }`}
              />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search for projects, issues, contributors..."
                autoFocus
                className={`flex-1 bg-transparent outline-none text-[16px] transition-colors ${
                  darkTheme
                    ? 'text-white placeholder:text-white/40'
                    : 'text-[#2d2820] placeholder:text-black/40'
                }`}
              />
              {isLoading && <Loader className="w-5 h-5 animate-spin text-[#c9983a]" />}
            </div>
          </div>

          {searchQuery ? (
            <SearchResults darkTheme={darkTheme} />
          ) : (
            <div>
              <h1
                className={`text-[42px] font-bold text-center mb-4 leading-tight transition-colors ${
                  darkTheme ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                }`}
              >
                Search Open Source Projects and
                <br />
                Build Your Confidence
              </h1>
              <p
                className={`text-center text-[15px] mb-8 transition-colors ${
                  darkTheme ? 'text-[#b8a898]/80' : 'text-[#6b5d4d]/80'
                }`}
              >
                Build your open source portfolio to optimize your chances of getting funded.
                <br />
                Explore projects that help you stand out.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
