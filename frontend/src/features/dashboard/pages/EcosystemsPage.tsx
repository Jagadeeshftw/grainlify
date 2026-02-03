import React, { useState, useEffect } from 'react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { Search, Globe, Plus, ArrowUpRight, Sparkles, Send } from 'lucide-react';
import { Modal, ModalFooter, ModalButton, ModalInput, ModalSelect } from '../../../shared/components/ui/Modal';
import { getEcosystems } from '../../../shared/api/client';

interface EcosystemsPageProps {
  onEcosystemClick: (id: string, name: string, description?: string | null, logoUrl?: string | null) => void;
}

export function EcosystemsPage({ onEcosystemClick }: EcosystemsPageProps) {
  console.log('=== EcosystemsPage (features/dashboard) FUNCTION CALLED ===');
  const { theme } = useTheme();
  const [showAddModal, setShowAddModal] = useState(false);
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    status: 'active',
    websiteUrl: ''
  });
  const [ecosystems, setEcosystems] = useState<any[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');

  // Fetch ecosystems function
  const fetchEcosystems = async () => {
    console.log('fetchEcosystems function called');
    setIsLoading(true);
    try {
      console.log('Fetching ecosystems from API...');
      const response = await getEcosystems();
      console.log('Ecosystems API response:', response);
      console.log('Response type:', typeof response);
      console.log('Response.ecosystems:', response?.ecosystems);
      console.log('Is array?', Array.isArray(response?.ecosystems));
      
      // Handle different response structures
      let ecosystemsArray: any[] = [];
      
      if (response && Array.isArray(response)) {
        // Response is directly an array
        ecosystemsArray = response;
        console.log('Response is direct array');
      } else if (response && response.ecosystems && Array.isArray(response.ecosystems)) {
        // Response has ecosystems property
        ecosystemsArray = response.ecosystems;
        console.log('Response has ecosystems property');
      } else if (response && typeof response === 'object') {
        // Try to find any array property
        const keys = Object.keys(response);
        console.log('Response keys:', keys);
        for (const key of keys) {
          if (Array.isArray((response as any)[key])) {
            ecosystemsArray = (response as any)[key];
            console.log(`Found array in key: ${key}`);
            break;
          }
        }
      }
      
      if (ecosystemsArray.length === 0) {
        console.warn('No ecosystems found in response:', response);
        setEcosystems([]);
        setIsLoading(false);
        return;
      }
      
      // Transform API response to match UI format
      const transformed = ecosystemsArray.map((eco: any) => {
        const firstLetter = eco.name ? eco.name.charAt(0).toUpperCase() : '?';
        const colors = [
          'from-[#c9983a] to-[#a67c2e]',
          'from-[#8b5cf6] to-[#7c3aed]',
          'from-[#06b6d4] to-[#0891b2]',
          'from-[#10b981] to-[#059669]',
          'from-[#f59e0b] to-[#d97706]',
          'from-[#ef4444] to-[#dc2626]',
        ];
        const colorIndex = eco.name ? eco.name.length % colors.length : 0;
        return {
          id: eco.id,
          name: eco.name || 'Unnamed Ecosystem',
          slug: eco.slug || '',
          description: eco.description || 'No description available.',
          projects: eco.project_count || 0,
          contributors: eco.user_count || 0,
          logo_url: eco.logo_url || null,
          website_url: eco.website_url || null,
          status: eco.status || 'active',
          letter: firstLetter,
          color: colors[colorIndex],
          languages: [] // Can be populated later if needed
        };
      });
      console.log('Transformed ecosystems:', transformed);
      setEcosystems(transformed);
    } catch (error) {
      console.error('Failed to fetch ecosystems:', error);
      console.error('Error details:', error instanceof Error ? error.message : error);
      setEcosystems([]);
    } finally {
      setIsLoading(false);
    }
  };

  // Fetch ecosystems on mount and when updated
  useEffect(() => {
    console.log('EcosystemsPage useEffect running');
    console.log('Calling fetchEcosystems...');
    fetchEcosystems();
    
    // Listen for ecosystem updates
    const handleUpdate = () => {
      console.log('Ecosystems updated event received');
      fetchEcosystems();
    };
    window.addEventListener('ecosystems-updated', handleUpdate);
    
    return () => {
      window.removeEventListener('ecosystems-updated', handleUpdate);
    };
  }, []);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Handle form submission here
    console.log('Form data:', formData);
    setShowAddModal(false);
    // Reset form
    setFormData({
      name: '',
      description: '',
      status: 'active',
      websiteUrl: ''
    });
  };

  const [showRequestModal, setShowRequestModal] = useState(false);
  const [requestData, setRequestData] = useState({
    userName: '',
    userEmail: '',
    ecosystemName: '',
    reason: '',
    additionalInfo: ''
  });

  const handleRequestSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // Handle request submission
    console.log('Request data:', requestData);
    setShowRequestModal(false);
    // Reset form
    setRequestData({
      userName: '',
      userEmail: '',
      ecosystemName: '',
      reason: '',
      additionalInfo: ''
    });
  };

  // Filter ecosystems based on search query
  const filteredEcosystems = ecosystems.filter(eco =>
    eco.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    (eco.description && eco.description.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  return (
    <div className="space-y-4 md:space-y-6 px-4 md:px-0">
      {/* Header Section */}
      <div className={`backdrop-blur-[40px] bg-gradient-to-br rounded-[20px] md:rounded-[28px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6 md:p-10 transition-colors ${
        theme === 'dark'
          ? 'from-white/[0.08] to-white/[0.04] border-white/10'
          : 'from-white/[0.15] to-white/[0.08] border-white/20'
      }`}>
        <div className="flex flex-col sm:flex-row items-start justify-between gap-4">
          <div className="flex-1">
            <h1 className={`text-[24px] md:text-[36px] font-bold mb-2 md:mb-3 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Explore Ecosystems</h1>
            <p className={`text-[14px] md:text-[16px] max-w-3xl transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Discover a wide range of projects shaping the future of open source, each driving revolutionary change.
            </p>
          </div>
          <div className="w-12 h-12 md:w-16 md:h-16 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] flex items-center justify-center shadow-[0_8px_24px_rgba(162,121,44,0.3)] border border-white/15 flex-shrink-0">
            <Globe className="w-6 h-6 md:w-8 md:h-8 text-white" />
          </div>
        </div>
      </div>

      {/* Search Bar */}
      <div className="relative">
        <Search className={`absolute left-3 md:left-4 top-1/2 -translate-y-1/2 w-4 h-4 md:w-5 md:h-5 z-10 pointer-events-none transition-colors ${
          theme === 'dark' ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
        }`} />
        <input
          type="text"
          placeholder="Search ecosystems..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className={`w-full pl-10 md:pl-12 pr-4 py-3 md:py-3.5 rounded-[12px] md:rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[13px] md:text-[14px] shadow-[inset_0px_0px_4px_0px_rgba(0,0,0,0.12)] relative touch-manipulation ${
            theme === 'dark'
              ? 'bg-white/[0.08] border-white/15 text-[#f5f5f5] placeholder-[#d4d4d4] focus:bg-white/[0.12] focus:border-[#c9983a]/30'
              : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2] focus:border-[#c9983a]/30'
          }`}
        />
      </div>

      {/* Ecosystems Grid */}
      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-5 animate-pulse">
          {Array.from({ length: 6 }).map((_, idx) => (
            <div
              key={idx}
              className={`backdrop-blur-[30px] rounded-[16px] md:rounded-[20px] border p-4 md:p-6 ${
                theme === 'dark'
                  ? 'bg-white/[0.08] border-white/10'
                  : 'bg-white/[0.15] border-white/25'
              }`}
            >
              {/* Icon */}
              <div className="flex items-start justify-between mb-4 md:mb-5">
                <div className={`w-12 h-12 md:w-14 md:h-14 rounded-[12px] md:rounded-[14px] ${
                  theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                }`} />
              </div>

              {/* Title */}
              <div className={`h-5 w-2/3 rounded ${
                theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
              }`} />

              {/* Stats */}
              <div className="flex items-center gap-6 mt-4 mb-4">
                <div className="flex-1">
                  <div className={`h-3 w-16 rounded ${
                    theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                  }`} />
                  <div className={`h-6 w-10 rounded mt-2 ${
                    theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                  }`} />
                </div>
                <div className="flex-1">
                  <div className={`h-3 w-24 rounded ${
                    theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                  }`} />
                  <div className={`h-6 w-10 rounded mt-2 ${
                    theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                  }`} />
                </div>
              </div>

              {/* Description */}
              <div className={`h-3 w-full rounded ${
                theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
              }`} />
              <div className={`h-3 w-5/6 rounded mt-2 ${
                theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
              }`} />

              {/* Footer */}
              <div className="mt-4">
                <div className={`h-4 w-28 rounded ${
                  theme === 'dark' ? 'bg-white/10' : 'bg-black/10'
                }`} />
              </div>
            </div>
          ))}
        </div>
      ) : filteredEcosystems.length === 0 ? (
        <div className={`text-center py-8 md:py-12 px-4 ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
          <p className={`text-[14px] md:text-[16px] ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
            {searchQuery ? 'No ecosystems found matching your search.' : 'No ecosystems available yet.'}
          </p>
          {!isLoading && ecosystems.length > 0 && (
            <div className="mt-2 text-[11px] md:text-xs opacity-70">
              (Filtered from {ecosystems.length} ecosystems)
            </div>
          )}
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 md:gap-5">
          {filteredEcosystems.map((ecosystem) => {
            console.log('Rendering ecosystem:', ecosystem);
            return (
          <div
            key={ecosystem.id}
            onClick={() => onEcosystemClick(ecosystem.id, ecosystem.name, ecosystem.description, ecosystem.logo_url)}
            className={`backdrop-blur-[30px] rounded-[16px] md:rounded-[20px] border p-4 md:p-6 transition-all cursor-pointer group touch-manipulation ${
              theme === 'dark'
                ? 'bg-white/[0.08] border-white/10 hover:bg-white/[0.12] hover:shadow-[0_8px_24px_rgba(201,152,58,0.15)] active:bg-white/[0.15]'
                : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:shadow-[0_8px_24px_rgba(0,0,0,0.08)] active:bg-white/[0.25]'
            }`}
          >
            {/* Header with Icon */}
            <div className="flex items-start justify-between mb-4 md:mb-5">
              <div className={`w-12 h-12 md:w-14 md:h-14 rounded-[12px] md:rounded-[14px] flex items-center justify-center shadow-lg border border-white/20 overflow-hidden ${ecosystem.logo_url ? 'bg-white' : `bg-gradient-to-br ${ecosystem.color}`}`}>
                {ecosystem.logo_url ? (
                  <img
                    src={ecosystem.logo_url}
                    alt={`${ecosystem.name} logo`}
                    className="w-full h-full object-cover"
                    onError={(event) => {
                      (event.target as HTMLImageElement).style.display = 'none';
                    }}
                  />
                ) : (
                  <span className="text-white text-[20px] md:text-[24px] font-bold">
                    {ecosystem.letter}
                  </span>
                )}
              </div>
            </div>

            {/* Title */}
            <h3 className={`text-[16px] md:text-[18px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>{ecosystem.name}</h3>

            {/* Stats */}
            <div className="flex items-center gap-4 md:gap-6 mb-3 md:mb-4">
              <div>
                <div className={`text-[10px] md:text-[11px] mb-0.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Projects</div>
                <div className={`text-[18px] md:text-[20px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{ecosystem.projects}</div>
              </div>
              <div>
                <div className={`text-[10px] md:text-[11px] mb-0.5 transition-colors ${
                  theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                }`}>Contributors</div>
                <div className={`text-[18px] md:text-[20px] font-bold transition-colors ${
                  theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>{ecosystem.contributors}</div>
              </div>
            </div>

            {/* Description */}
            <p className={`text-[12px] md:text-[13px] mb-4 md:mb-5 leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              {ecosystem.description}
            </p>

            {/* Website Link */}
            {ecosystem.website_url && (
              <div className="flex items-center gap-2 mt-3 md:mt-4">
                <Globe className={`w-4 h-4 ${theme === 'dark' ? 'text-[#c9983a]' : 'text-[#8b6f3a]'}`} />
                <a
                  href={ecosystem.website_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={(e) => e.stopPropagation()}
                  className={`text-[11px] md:text-[12px] hover:underline transition-colors touch-manipulation ${
                    theme === 'dark' ? 'text-[#c9983a]' : 'text-[#8b6f3a]'
                  }`}
                >
                  Visit Website
                </a>
              </div>
            )}
          </div>
            );
          })}
        </div>
      )}

      {/* Request Ecosystem Section */}
      <div className={`backdrop-blur-[40px] bg-gradient-to-br rounded-[20px] md:rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6 md:p-10 transition-all overflow-hidden relative ${
        theme === 'dark'
          ? 'from-white/[0.08] to-white/[0.04] border-white/10'
          : 'from-white/[0.15] to-white/[0.08] border-white/20'
      }`}>
        {/* Decorative gradient circles */}
        <div className="absolute -top-20 -right-20 w-40 h-40 md:w-60 md:h-60 bg-gradient-to-br from-[#c9983a]/20 to-transparent rounded-full blur-3xl"></div>
        <div className="absolute -bottom-20 -left-20 w-40 h-40 md:w-60 md:h-60 bg-gradient-to-br from-[#c9983a]/10 to-transparent rounded-full blur-3xl"></div>
        
        <div className="relative z-10">
          <div className="text-center max-w-2xl mx-auto">
            <div className="inline-flex items-center justify-center w-16 h-16 md:w-20 md:h-20 rounded-full bg-gradient-to-br from-[#c9983a] to-[#a67c2e] shadow-[0_8px_24px_rgba(162,121,44,0.4)] mb-4 md:mb-6 border border-white/15">
              <Sparkles className="w-8 h-8 md:w-10 md:h-10 text-white" />
            </div>
            
            <h3 className={`text-[22px] md:text-[28px] font-bold mb-3 md:mb-4 transition-colors ${
              theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
            }`}>Missing Your Ecosystem?</h3>
            
            <p className={`text-[14px] md:text-[16px] mb-5 md:mb-6 leading-relaxed transition-colors ${
              theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
            }`}>
              Don't see your ecosystem in the list? No worries! Request the admin to add it to our platform.
            </p>
            
            <button
              onClick={() => setShowRequestModal(true)}
              className="group px-5 md:px-8 py-3 md:py-4 bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white rounded-[12px] md:rounded-[16px] font-semibold text-[13px] md:text-[15px] shadow-[0_6px_20px_rgba(162,121,44,0.35)] hover:shadow-[0_10px_30px_rgba(162,121,44,0.5)] transition-all flex items-center justify-center gap-2 md:gap-3 mx-auto border border-white/10 hover:scale-105 active:scale-100 touch-manipulation min-h-[44px] w-full sm:w-auto"
            >
              <Plus className="w-4 h-4 md:w-5 md:h-5 group-hover:rotate-90 transition-transform flex-shrink-0" />
              <span className="text-center">Request Ecosystem Addition</span>
              <ArrowUpRight className="w-4 h-4 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform hidden sm:block flex-shrink-0" />
            </button>
          </div>
        </div>
      </div>

      {/* Add Ecosystem Modal (Admin Only) */}
      <Modal
        isOpen={showAddModal}
        onClose={() => setShowAddModal(false)}
        title="Add New Ecosystem"
        width="md"
      >
        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <ModalInput
              label="Ecosystem Name"
              value={formData.name}
              onChange={(value) => setFormData({ ...formData, name: value })}
              placeholder="e.g., Web3 Ecosystem"
              required
            />

            <ModalInput
              label="Description"
              value={formData.description}
              onChange={(value) => setFormData({ ...formData, description: value })}
              placeholder="Describe the ecosystem..."
              rows={4}
              required
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
              onChange={(value) => setFormData({ ...formData, websiteUrl: value })}
              placeholder="https://example.com"
              required
            />
          </div>

          <ModalFooter>
            <ModalButton onClick={() => setShowAddModal(false)} variant="secondary">
              Cancel
            </ModalButton>
            <ModalButton type="submit" variant="primary">
              Add Ecosystem
            </ModalButton>
          </ModalFooter>
        </form>
      </Modal>

      {/* Request Ecosystem Modal */}
      <Modal
        isOpen={showRequestModal}
        onClose={() => setShowRequestModal(false)}
        title="Request Ecosystem Addition"
        icon={<Sparkles className="w-5 h-5 md:w-6 md:h-6 text-white" />}
        width="lg"
        maxHeight
      >
        <p className={`text-[13px] md:text-[14px] mb-4 md:mb-6 transition-colors ${
          theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}>Fill out the form below and we'll review your request</p>

        <form onSubmit={handleRequestSubmit}>
          <div className="space-y-4">
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <ModalInput
                label="Your Name"
                value={requestData.userName}
                onChange={(value) => setRequestData({ ...requestData, userName: value })}
                placeholder="John Doe"
                required
              />

              <ModalInput
                label="Your Email"
                type="email"
                value={requestData.userEmail}
                onChange={(value) => setRequestData({ ...requestData, userEmail: value })}
                placeholder="john@example.com"
                required
              />
            </div>

            <ModalInput
              label="Ecosystem Name"
              value={requestData.ecosystemName}
              onChange={(value) => setRequestData({ ...requestData, ecosystemName: value })}
              placeholder="e.g., Web3 Ecosystem"
              required
            />

            <ModalInput
              label="Why do you want this ecosystem added?"
              value={requestData.reason}
              onChange={(value) => setRequestData({ ...requestData, reason: value })}
              placeholder="Tell us why this ecosystem would be valuable to the community..."
              rows={4}
              required
            />

            <ModalInput
              label="Additional Information (Optional)"
              value={requestData.additionalInfo}
              onChange={(value) => setRequestData({ ...requestData, additionalInfo: value })}
              placeholder="Any other details you'd like to share..."
              rows={3}
            />
          </div>

          <ModalFooter>
            <ModalButton onClick={() => setShowRequestModal(false)}>
              Cancel
            </ModalButton>
            <ModalButton type="submit" variant="primary">
              <Send className="w-4 h-4" />
              Submit Request
            </ModalButton>
          </ModalFooter>
        </form>
      </Modal>
    </div>
  );
}
