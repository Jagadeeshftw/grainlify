import { useState, useEffect } from 'react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { Shield, Globe, Plus, Sparkles, Trash2, ExternalLink, Calendar } from 'lucide-react';
import { Modal, ModalFooter, ModalButton, ModalInput, ModalSelect } from '../../../shared/components/ui/Modal';
import { DatePicker } from '../../../shared/components/ui/DatePicker';
import { createEcosystem, getAdminEcosystems, deleteEcosystem, createOpenSourceWeekEvent, getAdminOpenSourceWeekEvents, deleteOpenSourceWeekEvent } from '../../../shared/api/client';

interface Ecosystem {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  website_url: string | null;
  status: string;
  project_count: number;
  user_count: number;
  created_at: string;
  updated_at: string;
}

export function AdminPage() {
  const { theme } = useTheme();
  const [showAddModal, setShowAddModal] = useState(false);
  const [ecosystems, setEcosystems] = useState<Ecosystem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{ id: string; name: string } | null>(null);
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    status: 'active',
    websiteUrl: ''
  });
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validateName = (name: string) => {
    if (!name.trim()) return 'Ecosystem name is required';
    if (name.length < 2) return 'Ecosystem name must be at least 2 characters';
    if (name.length > 100) return 'Ecosystem name must be less than 100 characters';
    if (!/^[a-zA-Z0-9\s-]+$/.test(name)) return 'Name can only contain letters, numbers, spaces, and hyphens';
    return null;
  };

  const validateDescription = (description: string) => {
    if (!description.trim()) return 'Description is required';
    if (description.length < 10) return 'Description must be at least 10 characters';
    if (description.length > 500) return 'Description must be less than 500 characters';
    return null;
  };

  const validateWebsiteUrl = (url: string) => {
    if (!url.trim()) return 'Website URL is required';
    try {
      new URL(url);
      if (!url.startsWith('http')) return 'URL must start with http:// or https://';
      return null;
    } catch {
      return 'Please enter a valid URL (e.g., https://example.com)';
    }
  };

  const [isSubmitting, setIsSubmitting] = useState(false);

  // Open Source Week events
  const [oswEvents, setOswEvents] = useState<Array<{
    id: string;
    title: string;
    description: string | null;
    location: string | null;
    status: string;
    start_at: string;
    end_at: string;
  }>>([]);
  const [isOswLoading, setIsOswLoading] = useState(true);
  const [showAddOswModal, setShowAddOswModal] = useState(false);
  const [oswDeletingId, setOswDeletingId] = useState<string | null>(null);
  const [oswDeleteConfirm, setOswDeleteConfirm] = useState<{ id: string; title: string } | null>(null);
  const [oswForm, setOswForm] = useState({
    title: '',
    description: '',
    location: '',
    status: 'upcoming',
    startDate: '',
    startTime: '00:00',
    endDate: '',
    endTime: '00:00',
  });

  const fetchOswEvents = async () => {
    try {
      setIsOswLoading(true);
      const res = await getAdminOpenSourceWeekEvents();
      setOswEvents(res.events || []);
    } catch (e) {
      setOswEvents([]);
    } finally {
      setIsOswLoading(false);
    }
  };

  const fetchEcosystems = async () => {
    try {
      setIsLoading(true);
      setErrorMessage(null);
      const response = await getAdminEcosystems();
      setEcosystems(response.ecosystems || []);
    } catch (error) {
      console.error('Failed to fetch ecosystems:', error);
      setEcosystems([]);
      setErrorMessage(error instanceof Error ? error.message : 'Failed to load ecosystems.');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchEcosystems();
    fetchOswEvents();

    // Listen for ecosystem updates
    const handleEcosystemsUpdated = () => {
      fetchEcosystems();
    };
    window.addEventListener('ecosystems-updated', handleEcosystemsUpdated);
    return () => {
      window.removeEventListener('ecosystems-updated', handleEcosystemsUpdated);
    };
  }, []);

  const confirmDeleteOsw = (id: string, title: string) => {
    setOswDeleteConfirm({ id, title });
  };

  const handleDeleteOswConfirmed = async () => {
    if (!oswDeleteConfirm) return;
    setOswDeletingId(oswDeleteConfirm.id);
    try {
      await deleteOpenSourceWeekEvent(oswDeleteConfirm.id);
      await fetchOswEvents();
      setOswDeleteConfirm(null);
    } catch (e) {
      setErrorMessage(e instanceof Error ? e.message : 'Failed to delete event.');
    } finally {
      setOswDeletingId(null);
    }
  };

  const handleCreateOsw = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      setErrorMessage(null);
      const start_at = new Date(`${oswForm.startDate}T${oswForm.startTime}:00.000Z`).toISOString();
      const end_at = new Date(`${oswForm.endDate}T${oswForm.endTime}:00.000Z`).toISOString();
      await createOpenSourceWeekEvent({
        title: oswForm.title,
        description: oswForm.description || undefined,
        location: oswForm.location || undefined,
        status: oswForm.status as any,
        start_at,
        end_at,
      });
      setShowAddOswModal(false);
      setOswForm({
        title: '',
        description: '',
        location: '',
        status: 'upcoming',
        startDate: '',
        startTime: '00:00',
        endDate: '',
        endTime: '00:00',
      });
      await fetchOswEvents();
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : 'Failed to create event.');
    } finally {
      setIsSubmitting(false);
    }
  };

  const confirmDelete = (id: string, name: string) => {
    setDeleteConfirm({ id, name });
  };

  const handleDeleteConfirmed = async () => {
    if (!deleteConfirm) return;
    const { id } = deleteConfirm;
    setDeletingId(id);
    try {
      setErrorMessage(null);
      await deleteEcosystem(id);
      // Refresh the list
      await fetchEcosystems();
      // Dispatch event to update other pages
      window.dispatchEvent(new CustomEvent('ecosystems-updated'));
      setDeleteConfirm(null);
    } catch (error) {
      console.error('Failed to delete ecosystem:', error);
      setErrorMessage(error instanceof Error ? error.message : 'Failed to delete ecosystem. Make sure it has no associated projects.');
    } finally {
      setDeletingId(null);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validate all fields
    const nameError = validateName(formData.name);
    const descError = validateDescription(formData.description);
    const urlError = validateWebsiteUrl(formData.websiteUrl);

    const newErrors: Record<string, string> = {};
    if (nameError) newErrors.name = nameError;
    if (descError) newErrors.description = descError;
    if (urlError) newErrors.websiteUrl = urlError;

    setErrors(newErrors);

    if (Object.keys(newErrors).length > 0) {
      return;
    }

    setIsSubmitting(true);

    try {
      setErrorMessage(null);
      await createEcosystem({
        name: formData.name,
        description: formData.description || undefined,
        website_url: formData.websiteUrl || undefined,
        status: formData.status as 'active' | 'inactive',
      });

      // Success - close modal and reset form
      setShowAddModal(false);
      setErrors({});
      setFormData({
        name: '',
        description: '',
        status: 'active',
        websiteUrl: ''
      });

      // Refresh ecosystems list
      await fetchEcosystems();
      // Dispatch event to update other pages
      window.dispatchEvent(new CustomEvent('ecosystems-updated'));
    } catch (error) {
      console.error('Failed to create ecosystem:', error);
      setErrorMessage(error instanceof Error ? error.message : 'Failed to create ecosystem. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Admin Header */}
      <div className={`backdrop-blur-[40px] bg-gradient-to-br rounded-[28px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-10 transition-all overflow-hidden relative ${theme === 'dark'
        ? 'from-white/[0.08] to-white/[0.04] border-white/10'
        : 'from-white/[0.15] to-white/[0.08] border-white/20'
        }`}>
        {/* Decorative gradient */}
        <div className="absolute -top-20 -right-20 w-80 h-80 bg-gradient-to-br from-[#c9983a]/20 to-transparent rounded-full blur-3xl"></div>

        <div className="relative z-10">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] shadow-[0_6px_20px_rgba(162,121,44,0.35)] border border-white/10">
                  <Shield className="w-6 h-6 text-white" />
                </div>
                <h1 className={`text-[36px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                  }`}>Admin Panel</h1>
              </div>
              <p className={`text-[16px] max-w-3xl transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>
                Manage ecosystems, review requests, and oversee platform operations.
              </p>
            </div>
            <div className="flex items-center gap-3">
              <div className={`px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${theme === 'dark'
                ? 'bg-white/[0.08] border-white/15 text-[#d4d4d4]'
                : 'bg-white/[0.15] border-white/25 text-[#7a6b5a]'
                }`}>
                <span className="text-[13px] font-medium">Admin Access</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Ecosystem Management Section */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
        ? 'bg-white/[0.08] border-white/10'
        : 'bg-white/[0.15] border-white/20'
        }`}>
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className={`text-[24px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>Ecosystem Management</h2>
            <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>Add, edit, or remove ecosystems from the platform</p>
          </div>
          <button
            onClick={() => setShowAddModal(true)}
            className="group px-6 py-3.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_10px_30px_rgba(162,121,44,0.5)] transition-all flex items-center gap-2.5 border border-white/10 hover:scale-105"
          >
            <Plus className="w-5 h-5 group-hover:rotate-90 transition-transform" />
            Add New Ecosystem
            <Sparkles className="w-4 h-4 group-hover:scale-110 transition-transform" />
          </button>
        </div>

        {/* Inline error (avoid ugly alerts) */}
        {errorMessage && (
          <div className={`mb-4 rounded-[16px] border px-4 py-3 text-[13px] ${theme === 'dark'
            ? 'bg-red-500/10 border-red-500/20 text-red-200'
            : 'bg-red-500/10 border-red-500/20 text-red-700'
            }`}>
            {errorMessage}
          </div>
        )}

        {/* Ecosystems List */}
        <div className="mt-6">
          {isLoading ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 animate-pulse">
              {Array.from({ length: 6 }).map((_, idx) => (
                <div
                  key={idx}
                  className={`backdrop-blur-[30px] rounded-[16px] border p-5 ${theme === 'dark'
                    ? 'bg-white/[0.06] border-white/10'
                    : 'bg-white/[0.12] border-white/20'
                    }`}
                >
                  <div className="flex items-start justify-between mb-3">
                    <div
                      className={`w-12 h-12 rounded-[12px] ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                        }`}
                    />
                    <div
                      className={`w-8 h-8 rounded-[10px] ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                        }`}
                    />
                  </div>
                  <div
                    className={`h-5 w-2/3 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                      }`}
                  />
                  <div className="flex items-center gap-4 mt-3 mb-3">
                    <div className="flex-1">
                      <div
                        className={`h-3 w-16 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                          }`}
                      />
                      <div
                        className={`h-6 w-10 rounded mt-2 ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                          }`}
                      />
                    </div>
                    <div className="flex-1">
                      <div
                        className={`h-3 w-24 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                          }`}
                      />
                      <div
                        className={`h-6 w-10 rounded mt-2 ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                          }`}
                      />
                    </div>
                  </div>
                  <div
                    className={`h-3 w-full rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                      }`}
                  />
                  <div
                    className={`h-3 w-5/6 rounded mt-2 ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                      }`}
                  />
                  <div className="mt-4 pt-3 border-t border-white/10">
                    <div
                      className={`h-5 w-16 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                        }`}
                    />
                  </div>
                </div>
              ))}
            </div>
          ) : ecosystems.length === 0 ? (
            <div className={`text-center py-12 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>
              No ecosystems found. Add your first ecosystem above.
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {ecosystems.map((ecosystem) => {
                const firstLetter = ecosystem.name.charAt(0).toUpperCase();
                const colors = [
                  'bg-gradient-to-br from-[#c9983a] to-[#a67c2e]',
                  'bg-gradient-to-br from-purple-500 to-purple-600',
                  'bg-gradient-to-br from-blue-500 to-blue-600',
                  'bg-gradient-to-br from-green-500 to-green-600',
                  'bg-gradient-to-br from-red-500 to-red-600',
                  'bg-gradient-to-br from-pink-500 to-pink-600',
                ];
                const colorIndex = ecosystem.name.charCodeAt(0) % colors.length;
                const bgColor = colors[colorIndex];

                return (
                  <div
                    key={ecosystem.id}
                    className={`backdrop-blur-[30px] rounded-[16px] border p-5 transition-all hover:scale-[1.02] ${theme === 'dark'
                      ? 'bg-white/[0.06] border-white/10'
                      : 'bg-white/[0.12] border-white/20'
                      }`}
                  >
                    <div className="flex items-start justify-between mb-3">
                      <div className={`w-12 h-12 rounded-[12px] ${bgColor} flex items-center justify-center text-white font-bold text-lg shadow-lg`}>
                        {firstLetter}
                      </div>
                      <button
                        onClick={() => confirmDelete(ecosystem.id, ecosystem.name)}
                        disabled={deletingId === ecosystem.id}
                        className={`p-2 rounded-[10px] transition-all ${deletingId === ecosystem.id
                          ? 'opacity-50 cursor-not-allowed'
                          : theme === 'dark'
                            ? 'hover:bg-red-500/20 text-red-400'
                            : 'hover:bg-red-500/30 text-red-600'
                          }`}
                        title="Delete ecosystem"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>

                    <h3 className={`text-[18px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                      }`}>{ecosystem.name}</h3>

                    <div className="flex items-center gap-4 mb-3">
                      <div>
                        <p className={`text-[11px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                          }`}>Projects</p>
                        <p className={`text-[20px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                          }`}>{ecosystem.project_count}</p>
                      </div>
                      <div>
                        <p className={`text-[11px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                          }`}>Contributors</p>
                        <p className={`text-[20px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                          }`}>{ecosystem.user_count}</p>
                      </div>
                    </div>

                    {ecosystem.description && (
                      <p className={`text-[13px] mb-3 line-clamp-2 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                        }`}>{ecosystem.description}</p>
                    )}

                    {ecosystem.website_url && (
                      <a
                        href={ecosystem.website_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className={`flex items-center gap-2 text-[13px] transition-colors ${theme === 'dark' ? 'text-[#c9983a] hover:text-[#e8c77f]' : 'text-[#a67c2e] hover:text-[#c9983a]'
                          }`}
                      >
                        <Globe className="w-4 h-4" />
                        <span>Visit Website</span>
                        <ExternalLink className="w-3 h-3" />
                      </a>
                    )}

                    <div className="mt-3 pt-3 border-t border-white/10">
                      <span className={`text-[11px] px-2 py-1 rounded-[6px] ${ecosystem.status === 'active'
                        ? theme === 'dark'
                          ? 'bg-green-500/20 text-green-400'
                          : 'bg-green-500/30 text-green-700'
                        : theme === 'dark'
                          ? 'bg-gray-500/20 text-gray-400'
                          : 'bg-gray-500/30 text-gray-700'
                        }`}>
                        {ecosystem.status}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Info Message */}
        <div className={`backdrop-blur-[30px] rounded-[16px] border p-5 flex items-start gap-4 transition-colors mt-6 ${theme === 'dark'
          ? 'bg-white/[0.06] border-white/10'
          : 'bg-white/[0.12] border-white/20'
          }`}>
          <div className="p-2 rounded-[10px] bg-gradient-to-br from-[#c9983a]/20 to-[#a67c2e]/10 border border-[#c9983a]/20">
            <Sparkles className="w-5 h-5 text-[#c9983a]" />
          </div>
          <div>
            <p className={`text-[14px] font-medium mb-1 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>Ecosystem Management Tips</p>
            <p className={`text-[13px] leading-relaxed transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>
              Add ecosystems with accurate descriptions and valid website URLs. You can only delete ecosystems that have no associated projects.
            </p>
          </div>
        </div>
      </div>

      {/* Open Source Week Events Section */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
        ? 'bg-white/[0.08] border-white/10'
        : 'bg-white/[0.15] border-white/20'
        }`}>
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className={`text-[24px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}>Open-Source Week Events</h2>
            <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
              }`}>Create and manage Open-Source Week events (no hardcoded data)</p>
          </div>
          <button
            onClick={() => setShowAddOswModal(true)}
            className="group px-6 py-3.5 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[16px] font-semibold text-[14px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_10px_30px_rgba(162,121,44,0.5)] transition-all flex items-center gap-2.5 border border-white/10 hover:scale-105"
          >
            <Plus className="w-5 h-5 group-hover:rotate-90 transition-transform" />
            Add Event
            <Calendar className="w-4 h-4 group-hover:scale-110 transition-transform" />
          </button>
        </div>

        {isOswLoading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 animate-pulse">
            {Array.from({ length: 3 }).map((_, idx) => (
              <div
                key={idx}
                className={`backdrop-blur-[30px] rounded-[16px] border p-5 ${theme === 'dark' ? 'bg-white/[0.06] border-white/10' : 'bg-white/[0.12] border-white/20'
                  }`}
              >
                <div className={`h-5 w-2/3 rounded ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                <div className={`h-3 w-full rounded mt-3 ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
                <div className={`h-3 w-5/6 rounded mt-2 ${theme === 'dark' ? 'bg-white/10' : 'bg-black/10'}`} />
              </div>
            ))}
          </div>
        ) : oswEvents.length === 0 ? (
          <div className={`text-center py-10 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
            No Open-Source Week events yet. Create one (e.g. Feb 21–Feb 28) using “Add Event”.
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {oswEvents.map((ev) => (
              <div
                key={ev.id}
                className={`backdrop-blur-[30px] rounded-[16px] border p-5 ${theme === 'dark' ? 'bg-white/[0.06] border-white/10' : 'bg-white/[0.12] border-white/20'
                  }`}
              >
                <div className="flex items-start justify-between">
                  <div>
                    <h3 className={`text-[16px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                      }`}>{ev.title}</h3>
                    <p className={`text-[12px] mt-1 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                      }`}>
                      {new Date(ev.start_at).toLocaleDateString()} → {new Date(ev.end_at).toLocaleDateString()}
                    </p>
                  </div>
                  <button
                    onClick={() => confirmDeleteOsw(ev.id, ev.title)}
                    className={`p-2 rounded-[10px] transition-all ${theme === 'dark' ? 'hover:bg-red-500/20 text-red-400' : 'hover:bg-red-500/30 text-red-600'
                      }`}
                    title="Delete event"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
                {ev.location && (
                  <p className={`text-[12px] mt-2 transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                    }`}>Location: {ev.location}</p>
                )}
                <div className="mt-3">
                  <span className={`text-[11px] px-2 py-1 rounded-[6px] ${theme === 'dark' ? 'bg-white/10 text-[#d4d4d4]' : 'bg-black/10 text-[#7a6b5a]'
                    }`}>{ev.status}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Add Ecosystem Modal */}
      <Modal
        isOpen={showAddModal}
        onClose={() => setShowAddModal(false)}
        title="Add New Ecosystem"
        width="lg"
      >
        <p className={`text-[14px] mb-6 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
          }`}>Create a new ecosystem entry for the platform</p>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <ModalInput
              label="Ecosystem Name"
              value={formData.name}
              onChange={(value) => {
                setFormData({ ...formData, name: value });
                if (errors.name) setErrors({ ...errors, name: '' });
              }}
              onBlur={() => {
                const error = validateName(formData.name);
                if (error) setErrors(prev => ({ ...prev, name: error }));
              }}
              placeholder="e.g., Web3 Ecosystem"
              error={errors.name}
            />

            <ModalInput
              label="Description"
              value={formData.description}
              onChange={(value) => {
                setFormData({ ...formData, description: value });
                if (errors.description) setErrors({ ...errors, description: '' });
              }}
              onBlur={() => {
                const error = validateDescription(formData.description);
                if (error) setErrors(prev => ({ ...prev, description: error }));
              }}
              placeholder="Describe the ecosystem..."
              rows={4}
              error={errors.description}
            />

            <ModalSelect
              label="Status"
              value={formData.status}
              onChange={(value) => setFormData({ ...formData, status: value })}
              options={[
                { value: 'active', label: 'Active' },
                { value: 'inactive', label: 'Inactive' }
              ]}
            />

            <ModalInput
              label="Website URL"
              type="url"
              value={formData.websiteUrl}
              onChange={(value) => {
                setFormData({ ...formData, websiteUrl: value });
                if (errors.websiteUrl) setErrors({ ...errors, websiteUrl: '' });
              }}
              onBlur={() => {
                const error = validateWebsiteUrl(formData.websiteUrl);
                if (error) setErrors(prev => ({ ...prev, websiteUrl: error }));
              }}
              placeholder="https://example.com"
              error={errors.websiteUrl}
            />
          </div>

          <ModalFooter>
            <ModalButton onClick={() => setShowAddModal(false)}>
              Cancel
            </ModalButton>
            <ModalButton type="submit" variant="primary" disabled={isSubmitting}>
              <Plus className="w-4 h-4" />
              {isSubmitting ? 'Adding...' : 'Add Ecosystem'}
            </ModalButton>
          </ModalFooter>
        </form>
      </Modal>

      {/* Add Open Source Week Event Modal */}
      <Modal
        isOpen={showAddOswModal}
        onClose={() => setShowAddOswModal(false)}
        title="Add Open-Source Week Event"
        icon={<Calendar className="w-6 h-6 text-[#c9983a]" />}
        width="lg"
      >
        <form onSubmit={handleCreateOsw}>
          <div className="space-y-4">
            <ModalInput
              label="Title"
              value={oswForm.title}
              onChange={(value) => setOswForm({ ...oswForm, title: value })}
              placeholder="Open-Source Week"
              required
            />
            <ModalInput
              label="Description"
              value={oswForm.description}
              onChange={(value) => setOswForm({ ...oswForm, description: value })}
              placeholder="Describe the event..."
              rows={3}
            />
            <ModalInput
              label="Location"
              value={oswForm.location}
              onChange={(value) => setOswForm({ ...oswForm, location: value })}
              placeholder="Worldwide"
            />
            <ModalSelect
              label="Status"
              value={oswForm.status}
              onChange={(value) => setOswForm({ ...oswForm, status: value })}
              options={[
                { value: 'upcoming', label: 'Upcoming' },
                { value: 'running', label: 'Running' },
                { value: 'completed', label: 'Completed' },
                { value: 'draft', label: 'Draft (hidden from public)' },
              ]}
            />
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <DatePicker
                label="Start date (UTC)"
                value={oswForm.startDate}
                onChange={(value) => setOswForm({ ...oswForm, startDate: value })}
                placeholder="Select start date"
                required
              />
              <ModalInput
                label="Start time (UTC)"
                type="time"
                value={oswForm.startTime}
                onChange={(value) => setOswForm({ ...oswForm, startTime: value })}
                required
              />
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <DatePicker
                label="End date (UTC)"
                value={oswForm.endDate}
                onChange={(value) => setOswForm({ ...oswForm, endDate: value })}
                placeholder="Select end date"
                required
              />
              <ModalInput
                label="End time (UTC)"
                type="time"
                value={oswForm.endTime}
                onChange={(value) => setOswForm({ ...oswForm, endTime: value })}
                required
              />
            </div>
          </div>
          <ModalFooter>
            <ModalButton onClick={() => setShowAddOswModal(false)} disabled={isSubmitting}>
              Cancel
            </ModalButton>
            <ModalButton type="submit" variant="primary" disabled={isSubmitting || !oswForm.title.trim()}>
              <Plus className="w-4 h-4" />
              {isSubmitting ? 'Creating...' : 'Create Event'}
            </ModalButton>
          </ModalFooter>
        </form>
      </Modal>

      {/* Delete OSW Event Modal */}
      <Modal
        isOpen={!!oswDeleteConfirm}
        onClose={() => setOswDeleteConfirm(null)}
        title="Delete Event"
        icon={<Trash2 className="w-6 h-6 text-[#c9983a]" />}
        width="md"
      >
        <div className="space-y-4">
          <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
            Are you sure you want to delete <span className={theme === 'dark' ? 'text-[#f5f5f5] font-semibold' : 'text-[#2d2820] font-semibold'}>
              {oswDeleteConfirm?.title}
            </span>? This action cannot be undone.
          </p>
          <ModalFooter>
            <ModalButton variant="secondary" onClick={() => setOswDeleteConfirm(null)} disabled={!!oswDeletingId}>
              Cancel
            </ModalButton>
            <ModalButton variant="primary" onClick={handleDeleteOswConfirmed} disabled={!!oswDeletingId}>
              {oswDeletingId ? 'Deleting...' : 'Delete'}
            </ModalButton>
          </ModalFooter>
        </div>
      </Modal>

      {/* Delete Confirmation Modal (no browser confirm) */}
      <Modal
        isOpen={!!deleteConfirm}
        onClose={() => setDeleteConfirm(null)}
        title="Delete Ecosystem"
        icon={<Trash2 className="w-6 h-6 text-[#c9983a]" />}
        width="md"
      >
        <div className="space-y-4">
          <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
            Are you sure you want to delete <span className={theme === 'dark' ? 'text-[#f5f5f5] font-semibold' : 'text-[#2d2820] font-semibold'}>
              {deleteConfirm?.name}
            </span>? This action cannot be undone.
          </p>
          <div className={`rounded-[16px] border px-4 py-3 text-[13px] ${theme === 'dark'
            ? 'bg-white/[0.06] border-white/10 text-[#d4d4d4]'
            : 'bg-white/[0.12] border-white/20 text-[#7a6b5a]'
            }`}>
            You can only delete ecosystems that have <span className={theme === 'dark' ? 'text-[#f5f5f5] font-medium' : 'text-[#2d2820] font-medium'}>no associated projects</span>.
          </div>
          <ModalFooter>
            <ModalButton
              variant="secondary"
              onClick={() => setDeleteConfirm(null)}
              disabled={!!deletingId}
            >
              Cancel
            </ModalButton>
            <ModalButton
              variant="primary"
              onClick={handleDeleteConfirmed}
              disabled={!deleteConfirm || !!deletingId}
            >
              {deletingId ? 'Deleting...' : 'Delete'}
            </ModalButton>
          </ModalFooter>
        </div>
      </Modal>
    </div>
  );
}