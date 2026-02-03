import { useState, useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { X, Loader2, AlertCircle, CheckCircle2, ChevronDown } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import {
  getEcosystems,
  updateProjectMetadata,
  type PendingSetupProject,
} from '../../../shared/api/client';
import { SkeletonLoader } from '../../../shared/components/SkeletonLoader';

interface NewProjectSetupModalProps {
  isOpen: boolean;
  project: PendingSetupProject | null;
  onClose: () => void;
  onSuccess: () => void;
  /** Optional title (e.g. "Edit project" when editing). */
  title?: string;
}

export function NewProjectSetupModal({
  isOpen,
  project,
  onClose,
  onSuccess,
  title: titleOverride,
}: NewProjectSetupModalProps) {
  const { theme } = useTheme();
  const darkTheme = theme === 'dark';

  const [description, setDescription] = useState('');
  const [ecosystemName, setEcosystemName] = useState('');
  const [language, setLanguage] = useState('');
  const [tags, setTags] = useState('');
  const [category, setCategory] = useState('');

  const [ecosystems, setEcosystems] = useState<Array<{ name: string; slug: string }>>([]);
  const [isLoadingEcosystems, setIsLoadingEcosystems] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [ecosystemDropdownOpen, setEcosystemDropdownOpen] = useState(false);
  const ecosystemDropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (ecosystemDropdownRef.current && !ecosystemDropdownRef.current.contains(e.target as Node)) {
        setEcosystemDropdownOpen(false);
      }
    };
    if (ecosystemDropdownOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [ecosystemDropdownOpen]);

  // Populate form whenever the modal opens or the project changes (so Edit shows existing data)
  useEffect(() => {
    if (isOpen && project) {
      setDescription(project.description ?? '');
      setEcosystemName(project.ecosystem_name ?? '');
      setLanguage(project.language ?? '');
      setTags(Array.isArray(project.tags) ? project.tags.join(', ') : '');
      setCategory(project.category ?? '');
      setError(null);
      setSuccess(false);
    }
  }, [isOpen, project?.id, project?.description, project?.ecosystem_name, project?.language, project?.tags, project?.category]);

  useEffect(() => {
    if (isOpen) {
      loadEcosystems();
    }
  }, [isOpen]);

  const loadEcosystems = async () => {
    setIsLoadingEcosystems(true);
    setError(null);
    try {
      const data = await getEcosystems();
      setEcosystems(data.ecosystems.map((eco) => ({ name: eco.name, slug: eco.slug })));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load ecosystems');
    } finally {
      setIsLoadingEcosystems(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!project) return;
    setError(null);
    setSuccess(false);

    if (!ecosystemName.trim()) {
      setError('Ecosystem is required');
      return;
    }

    setIsSubmitting(true);

    try {
      const tagsArray = tags
        .split(',')
        .map((tag) => tag.trim())
        .filter((tag) => tag.length > 0);

      await updateProjectMetadata(project.id, {
        description: description.trim() || undefined,
        ecosystem_name: ecosystemName.trim(),
        language: language.trim() || undefined,
        tags: tagsArray.length > 0 ? tagsArray : undefined,
        category: category.trim() || undefined,
      });

      setSuccess(true);
      setTimeout(() => {
        onSuccess();
        onClose();
        setSuccess(false);
      }, 800);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save project details');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleClose = () => {
    if (!isSubmitting) {
      setError(null);
      setSuccess(false);
      onClose();
    }
  };

  if (!isOpen || !project) return null;

  const modalContent = (
    <div className="fixed inset-0 z-[10000] flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={handleClose}
        aria-hidden
      />

      <div
        className={`relative w-full max-w-[520px] rounded-[24px] border-2 shadow-[0_16px_64px_rgba(0,0,0,0.4)] transition-colors ${
          darkTheme
            ? 'bg-[#3a3228] border-white/30'
            : 'bg-[#d4c5b0] border-white/40'
        }`}
      >
        <div
          className={`flex items-center justify-between px-6 py-5 border-b-2 transition-colors ${
            darkTheme ? 'border-white/20' : 'border-white/30'
          }`}
        >
          <h2
            className={`text-[20px] font-bold transition-colors ${
              darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
            }`}
          >
            {titleOverride ?? 'New Project Setup'}
          </h2>
          <button
            type="button"
            onClick={handleClose}
            disabled={isSubmitting}
            className={`p-2 rounded-[10px] transition-all cursor-pointer ${
              darkTheme
                ? 'hover:bg-white/10 text-[#b8a898] hover:text-[#e8dfd0]'
                : 'hover:bg-white/20 text-[#7a6b5a] hover:text-[#2d2820]'
            } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-5">
          {error && (
            <div
              className={`flex items-center gap-3 p-4 rounded-[12px] border-2 ${
                darkTheme
                  ? 'bg-red-500/10 border-red-500/30 text-red-400'
                  : 'bg-red-100 border-red-300 text-red-700'
              }`}
            >
              <AlertCircle className="w-5 h-5 flex-shrink-0" />
              <span className="text-[14px] font-medium">{error}</span>
            </div>
          )}

          {success && (
            <div
              className={`flex items-center gap-3 p-4 rounded-[12px] border-2 ${
                darkTheme
                  ? 'bg-green-500/10 border-green-500/30 text-green-400'
                  : 'bg-green-100 border-green-300 text-green-700'
              }`}
            >
              <CheckCircle2 className="w-5 h-5 flex-shrink-0" />
              <span className="text-[14px] font-medium">Project details saved.</span>
            </div>
          )}

          {/* Repository (read-only) */}
          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Repository
            </label>
            <div
              className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all ${
                darkTheme
                  ? 'bg-white/5 border-white/15 text-[#b8a898]'
                  : 'bg-white/20 border-white/40 text-[#7a6b5a]'
              }`}
            >
              {project.github_full_name}
            </div>
          </div>

          {/* Description */}
          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Description
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Brief description of the project"
              disabled={isSubmitting}
              rows={3}
              className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all resize-none ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a] focus:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a] focus:bg-white/60'
              } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
            />
          </div>

          {/* Ecosystem */}
          <div ref={ecosystemDropdownRef}>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Ecosystem <span className="text-red-500">*</span>
            </label>
            {isLoadingEcosystems ? (
              <SkeletonLoader className="h-12 w-full rounded-[12px]" />
            ) : (
              <div className="relative">
                <button
                  type="button"
                  onClick={() => !(isSubmitting || ecosystems.length === 0) && setEcosystemDropdownOpen((o) => !o)}
                  disabled={isSubmitting || ecosystems.length === 0}
                  className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all flex items-center justify-between gap-2 text-left ${
                    darkTheme
                      ? 'bg-white/10 border-white/20 text-[#e8dfd0] focus:border-[#c9983a] focus:bg-white/15 hover:border-white/30'
                      : 'bg-white/40 border-white/50 text-[#2d2820] focus:border-[#c9983a] focus:bg-white/60 hover:border-white/60'
                  } ${isSubmitting || ecosystems.length === 0 ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
                  aria-haspopup="listbox"
                  aria-expanded={ecosystemDropdownOpen}
                  aria-required
                >
                  <span className={ecosystemName ? '' : 'opacity-70'}>
                    {ecosystemName || 'Select an ecosystem'}
                  </span>
                  <ChevronDown
                    className={`w-5 h-5 flex-shrink-0 transition-transform ${ecosystemDropdownOpen ? 'rotate-180' : ''} ${
                      darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                    }`}
                    aria-hidden
                  />
                </button>
                {ecosystemDropdownOpen && (
                  <ul
                    role="listbox"
                    className={`absolute top-full left-0 right-0 mt-1 py-1 rounded-[12px] border-2 shadow-lg z-50 max-h-48 overflow-auto ${
                      darkTheme
                        ? 'bg-[#3a3228] border-white/30 text-[#e8dfd0]'
                        : 'bg-[#d4c5b0] border-white/40 text-[#2d2820]'
                    }`}
                  >
                    <li
                      role="option"
                      aria-selected={!ecosystemName}
                      onClick={() => {
                        setEcosystemName('');
                        setEcosystemDropdownOpen(false);
                      }}
                      className={`px-4 py-2.5 cursor-pointer text-[14px] transition-colors ${
                        darkTheme
                          ? 'hover:bg-white/15 data-[selected]:bg-white/10'
                          : 'hover:bg-white/40 data-[selected]:bg-white/30'
                      } ${!ecosystemName ? (darkTheme ? 'bg-white/10' : 'bg-white/30') : ''}`}
                    >
                      Select an ecosystem
                    </li>
                    {ecosystems.map((eco) => (
                      <li
                        key={eco.slug}
                        role="option"
                        aria-selected={ecosystemName === eco.name}
                        onClick={() => {
                          setEcosystemName(eco.name);
                          setEcosystemDropdownOpen(false);
                        }}
                        className={`px-4 py-2.5 cursor-pointer text-[14px] transition-colors ${
                          darkTheme
                            ? 'hover:bg-white/15'
                            : 'hover:bg-white/40'
                        } ${ecosystemName === eco.name ? (darkTheme ? 'bg-white/10' : 'bg-white/30') : ''}`}
                      >
                        {eco.name}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
          </div>

          {/* Tags */}
          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Tags (optional)
            </label>
            <input
              type="text"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              placeholder="e.g., Payments, DeFi, Tooling"
              disabled={isSubmitting}
              className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a] focus:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a] focus:bg-white/60'
              } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
            />
            <p
              className={`mt-1.5 text-[12px] transition-colors ${
                darkTheme ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}
            >
              Separate multiple tags with commas
            </p>
          </div>

          {/* Category */}
          <div>
            <label
              className={`block text-[14px] font-semibold mb-2 transition-colors ${
                darkTheme ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
              }`}
            >
              Category (optional)
            </label>
            <input
              type="text"
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              placeholder="e.g., Frontend, Backend"
              disabled={isSubmitting}
              className={`w-full px-4 py-3 rounded-[12px] border-2 transition-all ${
                darkTheme
                  ? 'bg-white/10 border-white/20 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a] focus:bg-white/15'
                  : 'bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a] focus:bg-white/60'
              } ${isSubmitting ? 'opacity-50 cursor-not-allowed' : ''}`}
            />
          </div>

          <div className="flex items-center gap-3 pt-2">
            <button
              type="submit"
              disabled={isSubmitting || success}
              className={`flex-1 px-5 py-3 rounded-[12px] border-2 font-semibold text-[14px] transition-all ${
                darkTheme
                  ? 'bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-[#c9983a]/70 text-[#fef5e7] hover:from-[#c9983a]/50 hover:to-[#d4af37]/40 shadow-[0_4px_16px_rgba(201,152,58,0.4)]'
                  : 'bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border-[#c9983a]/50 text-[#2d2820] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 shadow-[0_4px_16px_rgba(201,152,58,0.25)]'
              } ${isSubmitting || success ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            >
              {isSubmitting ? (
                <span className="flex items-center justify-center gap-2">
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Saving...
                </span>
              ) : success ? (
                'Saved!'
              ) : (
                'Save & Continue'
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );

  return createPortal(modalContent, document.body);
}
