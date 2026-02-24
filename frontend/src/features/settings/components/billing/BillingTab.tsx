import { useState, useEffect, useRef } from 'react';
import { Plus, X, Loader2, AlertCircle, Info, ChevronDown, MessageSquare } from 'lucide-react';
import { BillingProfile, BillingProfileType, BillingProfileStatus, ProfileDetailTabType, PaymentMethod, Invoice } from '../../types';
import { initialBillingProfiles } from '../../data/billingProfilesData';
import { sampleInvoices } from '../../data/invoicesData';
import { BillingProfileCard } from './BillingProfileCard';
import { PaymentMethodsTab } from './PaymentMethodsTab';
import { InvoicesTab } from './InvoicesTab';
import { SkeletonLoader } from '../shared/SkeletonLoader';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { startKYCVerification, getKYCStatus } from '../../../../shared/api/client';
import { useBillingProfiles } from '../../contexts/BillingProfilesContext';

interface ProfileTypeOption {
  value: BillingProfileType;
  label: string;
  disabled: boolean;
  comingSoon?: boolean;
}

function ProfileTypeSelect({
  value,
  onChange,
  hasIndividualProfile,
  theme
}: {
  value: BillingProfileType;
  onChange: (val: BillingProfileType) => void;
  hasIndividualProfile: boolean;
  theme: string;
}) {
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const options: ProfileTypeOption[] = [
    {
      value: 'individual',
      label: `Individual${hasIndividualProfile ? ' (Already Created)' : ''}`,
      disabled: hasIndividualProfile
    },
    {
      value: 'self-employed',
      label: 'Self-Employed',
      disabled: true,
      comingSoon: true
    },
    {
      value: 'organization',
      label: 'Organization',
      disabled: true,
      comingSoon: true
    }
  ];

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const selectedOption = options.find(o => o.value === value) || options[0];

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className={`w-full flex items-center justify-between gap-3 px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none transition-all text-[14px] ${theme === 'dark'
          ? 'bg-white/[0.08] border-white/15 text-[#f5efe5] focus:border-[#c9983a]/30'
          : 'bg-white/[0.15] border-white/25 text-[#2d2820] focus:border-[#c9983a]/30'
          }`}
      >
        <span className="flex-1 text-left truncate">{selectedOption.label}</span>
        <ChevronDown className={`w-5 h-5 flex-shrink-0 transition-transform duration-200 ${isOpen ? 'rotate-180' : ''} ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`} />
      </button>

      {isOpen && (
        <div className={`absolute z-[110] mt-2 w-full min-w-[240px] rounded-[14px] border shadow-[0_10px_30px_rgba(0,0,0,0.12)] overflow-hidden backdrop-blur-[40px] animate-in fade-in slide-in-from-top-2 duration-200 ${theme === 'dark'
            ? 'bg-[#2d2820]/[0.95] border-white/10'
          : 'bg-[#d5cabc] border-[#c9983a]/30'
          }`}>
          <div className="py-1">
            {options.map((option) => (
              <button
                key={option.value}
                type="button"
                disabled={option.disabled}
                onClick={() => {
                  if (!option.disabled) {
                    onChange(option.value);
                    setIsOpen(false);
                  }
                }}
                className={`w-full px-4 py-3 text-left text-[14px] transition-all flex items-center justify-between group ${value === option.value
                  ? theme === 'dark'
                    ? 'bg-[#c9983a]/20 text-[#c9983a]'
                    : 'bg-[#c9983a]/10 text-[#a67c2e]'
                  : theme === 'dark'
                    ? 'text-[#f5efe5] hover:bg-white/5'
                    : 'text-[#2d2820] hover:bg-[#c9983a]/15'
                  } ${option.disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
              >
                <span className="truncate mr-2">
                  {option.label}
                </span>
                {option.comingSoon && (
                  <span className={`text-[10px] uppercase tracking-wider font-bold px-2 py-0.5 rounded-full flex-shrink-0 ${theme === 'dark' ? 'bg-white/10 text-white/50' : 'bg-[#c9983a]/15 text-[#a67c2e]'
                    }`}>
                    Soon
                  </span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function BillingTab() {
  const { theme } = useTheme();
  const { profiles, setProfiles, addProfile, updateProfile } = useBillingProfiles();
  const [isLoading, setIsLoading] = useState(false);
  const [selectedProfile, setSelectedProfile] = useState<BillingProfile | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [profileName, setProfileName] = useState('');
  const [profileType, setProfileType] = useState<BillingProfileType>('individual');
  const [detailTab, setDetailTab] = useState<ProfileDetailTabType>('general');
  const [isVerifying, setIsVerifying] = useState(false);
  const [kycStatus, setKycStatus] = useState<string | null>(null);
  const [isCheckingKYC, setIsCheckingKYC] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  
  const [kycWindowOpened, setKycWindowOpened] = useState(false);

  const handleCreateProfile = () => {
    if (!profileName.trim()) return;

    // Check if individual profile already exists
    if (profileType === 'individual') {
      const existingIndividual = profiles.find(p => p.type === 'individual');
      if (existingIndividual) {
        alert('An individual billing profile already exists. You can only create one individual profile.');
        return;
      }
    }

    const newProfile: BillingProfile = {
      id: Date.now(),
      name: profileName,
      type: profileType,
      status: 'missing-verification',
    };

    addProfile(newProfile);
    setShowModal(false);
    setProfileName('');
    setProfileType('individual');
  };

  // Check KYC status on component mount and when selected profile changes
  useEffect(() => {
    if (selectedProfile) {
      // Check KYC status for missing-verification profiles or verified profiles without data
      if (selectedProfile.status === 'missing-verification' ||
        (selectedProfile.status === 'verified' && !selectedProfile.firstName)) {
        checkKYCStatus();
      }
    }
  }, [selectedProfile]);

  const checkKYCStatus = async () => {
    setIsCheckingKYC(true);
    try {
      const statusResponse = await getKYCStatus();
      setKycStatus(statusResponse.status || null);

      // If verified, update the profile with KYC data
      if (statusResponse.status === 'verified' && statusResponse.extracted) {
        const extracted = statusResponse.extracted as any;
        updateProfileWithKYCData(extracted);
      }
    } catch (error) {
      console.error('Failed to check KYC status:', error);
      setErrorMessage("VerificationFailed: Connection to the identity server failed. Please try again.");
    } finally {
      setIsCheckingKYC(false);
    }
  };

  const updateProfileWithKYCData = (extracted: any) => {
    if (!selectedProfile) return;

    // Preserve the original profile name
    const updates: Partial<BillingProfile> = {
      name: selectedProfile.name, // Preserve the original name
      status: 'verified' as BillingProfileStatus,
      firstName: extracted.first_name || extracted.full_name?.split(' ')[0] || '',
      lastName: extracted.last_name || extracted.full_name?.split(' ').slice(1).join(' ') || '',
      address: extracted.address || '',
      city: extracted.city || '',
      postalCode: extracted.postal_code || extracted.postalCode || '',
      country: extracted.country || '',
      taxId: extracted.document_number || extracted.tax_id || '',
    };

    updateProfile(selectedProfile.id, updates);
    setSelectedProfile({ ...selectedProfile, ...updates });
  };

  const handleVerifyKYC = async () => {
    if (!selectedProfile) return;

    setIsVerifying(true);
    setKycWindowOpened(false);
    try {
      // Start KYC verification
      const response = await startKYCVerification();

      // Open the KYC URL in a new window
      if (response.url) {
        window.open(response.url, '_blank', 'width=800,height=600');
        setErrorMessage("");
        
        // Window opened successfully - update state to reflect this
        if (kycWindow) {
          setKycWindowOpened(true);
          setIsVerifying(false); // Stop the "Starting Verification..." state
          
          // Immediately check current status
          try {
            const initialStatus = await getKYCStatus();
            setKycStatus(initialStatus.status || 'pending');
          } catch {
            // Default to pending if we can't fetch status
            setKycStatus('pending');
          }
        }

        // Poll for status updates
        const pollInterval = setInterval(async () => {
          try {
            const statusResponse = await getKYCStatus();
            setKycStatus(statusResponse.status || null);

            if (statusResponse.status === 'verified') {
              clearInterval(pollInterval);
              setKycWindowOpened(false);
              if (statusResponse.extracted) {
                updateProfileWithKYCData(statusResponse.extracted);
              }
            } else if (statusResponse.status === 'rejected' || statusResponse.status === 'expired') {
              clearInterval(pollInterval);
              setKycWindowOpened(false);
            }
          } catch (error) {
            console.error('Failed to poll KYC status:', error);
            setErrorMessage("Connection lost. We're having trouble checking your verification status. Please refresh the page.");
          }
        }, 3000); // Poll every 3 seconds

        // Stop polling after 5 minutes
        setTimeout(() => {
          clearInterval(pollInterval);
          setKycWindowOpened(false);
        }, 5 * 60 * 1000);
      }
    } catch (error) {
      console.error('Failed to start KYC verification:', error);
      setErrorMessage("Could not start verification. Please try again later.");
      setIsVerifying(false);
      setKycWindowOpened(false);
    }
  };

  // Payment methods handlers
  const handleAddPaymentMethod = (method: PaymentMethod) => {
    if (!selectedProfile) return;

    const updatedProfile = {
      ...selectedProfile,
      paymentMethods: [...(selectedProfile.paymentMethods || []), method],
    };

    const updatedProfiles = profiles.map(p =>
      p.id === selectedProfile.id ? updatedProfile : p
    );

    setProfiles(updatedProfiles);
    setSelectedProfile(updatedProfile);
  };

  const handleRemovePaymentMethod = (methodId: number) => {
    if (!selectedProfile) return;

    const updatedProfile = {
      ...selectedProfile,
      paymentMethods: (selectedProfile.paymentMethods || []).filter(m => m.id !== methodId),
    };

    const updatedProfiles = profiles.map(p =>
      p.id === selectedProfile.id ? updatedProfile : p
    );

    setProfiles(updatedProfiles);
    setSelectedProfile(updatedProfile);
  };

  const handleSetDefaultPaymentMethod = (methodId: number) => {
    if (!selectedProfile) return;

    const updatedProfile = {
      ...selectedProfile,
      paymentMethods: (selectedProfile.paymentMethods || []).map(m => ({
        ...m,
        isDefault: m.id === methodId,
      })),
    };

    const updatedProfiles = profiles.map(p =>
      p.id === selectedProfile.id ? updatedProfile : p
    );

    setProfiles(updatedProfiles);
    setSelectedProfile(updatedProfile);
  };

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <SkeletonLoader className="h-[180px]" />
        <SkeletonLoader className="h-[180px]" />
        <SkeletonLoader className="h-[180px]" />
      </div>
    );
  }

  if (selectedProfile) {
    // Profile Detail View
    return (
      <div className="space-y-6">
        {errorMessage && (
                <div className="text-red-500 bg-red-100 p-2 rounded mb-4 border border-red-200">
                        {errorMessage}
                              </div>
                                  )}
                                  
        )
        {/* Back Button */}
        <button
          onClick={() => setSelectedProfile(null)}
          className={`flex items-center gap-2 hover:text-[#c9983a] transition-colors ${theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#2d2820]'
            }`}
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          <span className="text-[15px] font-medium">Back to billing profiles</span>
        </button>

        {/* Profile Header */}
        <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
          }`}>
          <div className="flex items-start justify-between mb-6">
            <div>
              <h2 className={`text-[28px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                }`}>{selectedProfile.name}</h2>
              <p className={`text-[14px] capitalize transition-colors ${theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
                }`}>
                {selectedProfile.type === 'organization' ? 'Company' : selectedProfile.type.replace('-', ' ')}
              </p>
              <p className={`text-[13px] mt-2 transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                }`}>
                As an individual, when you reach the annual reward amount limited by your tax residency you'll need to create a dedicated entity.
              </p>
            </div>

            {/* Reward Limit */}
            <div className="text-right">
              <div className={`text-[24px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                }`}>5000.00 USD</div>
              <div className={`text-[13px] transition-colors ${theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
                }`}>left</div>
            </div>
          </div>

          {/* Tab Navigation */}
          <div className="flex items-center gap-2">
            <button
              onClick={() => setDetailTab('general')}
              className={`px-5 py-2.5 rounded-[12px] text-[14px] font-medium transition-all ${detailTab === 'general'
                ? 'bg-[#a2792c] text-white shadow-[0_4px_16px_rgba(162,121,44,0.25)]'
                : theme === 'dark'
                  ? 'text-[#d4c5b0] hover:bg-white/[0.1]'
                  : 'text-[#6b5d4d] hover:bg-white/[0.1]'
                }`}
            >
              General Information
            </button>
            <button
              onClick={() => setDetailTab('payment')}
              className={`px-5 py-2.5 rounded-[12px] text-[14px] font-medium transition-all ${detailTab === 'payment'
                ? 'bg-[#a2792c] text-white shadow-[0_4px_16px_rgba(162,121,44,0.25)]'
                : theme === 'dark'
                  ? 'text-[#d4c5b0] hover:bg-white/[0.1]'
                  : 'text-[#6b5d4d] hover:bg-white/[0.1]'
                }`}
            >
              Payment Methods
            </button>
            <button
              onClick={() => setDetailTab('invoices')}
              className={`px-5 py-2.5 rounded-[12px] text-[14px] font-medium transition-all ${detailTab === 'invoices'
                ? 'bg-[#a2792c] text-white shadow-[0_4px_16px_rgba(162,121,44,0.25)]'
                : theme === 'dark'
                  ? 'text-[#d4c5b0] hover:bg-white/[0.1]'
                  : 'text-[#6b5d4d] hover:bg-white/[0.1]'
                }`}
            >
              Invoices
            </button>
          </div>
        </div>

        {/* General Information Tab */}
        {detailTab === 'general' && (
          <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${theme === 'dark'
            ? 'bg-[#2d2820]/[0.4] border-white/10'
            : 'bg-white/[0.12] border-white/20'
            }`}>
            <div className="flex items-center justify-between mb-6">
              <h3 className={`text-[20px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                }`}>General Information</h3>

              {/* Status Badge */}
              {selectedProfile.status === 'missing-verification' && kycStatus === null && (
                <div className={`flex items-center gap-2 px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${theme === 'dark'
                  ? 'bg-gradient-to-br from-[#f59e0b]/20 to-[#d97706]/15 border-[#f59e0b]/40'
                  : 'bg-gradient-to-br from-[#f59e0b]/15 to-[#d97706]/10 border-[#f59e0b]/35'
                  }`}>
                  <AlertCircle className={`w-4 h-4 ${theme === 'dark' ? 'text-[#f59e0b]' : 'text-[#d97706]'}`} />
                  <span className={`text-[13px] font-medium transition-colors ${theme === 'dark' ? 'text-[#f59e0b]' : 'text-[#d97706]'
                    }`}>Missing Verification</span>
                </div>
              )}

              {kycStatus === 'in_review' && (
                <div className={`flex items-center gap-2 px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${theme === 'dark'
                  ? 'bg-gradient-to-br from-[#c9983a]/20 to-[#d4af37]/15 border-[#c9983a]/40'
                  : 'bg-gradient-to-br from-[#c9983a]/15 to-[#d4af37]/10 border-[#c9983a]/35'
                  }`}>
                  <MessageSquare className={`w-4 h-4 ${theme === 'dark' ? 'text-[#c9983a]' : 'text-[#a67c2e]'}`} />
                  <span className={`text-[13px] font-medium transition-colors ${theme === 'dark' ? 'text-[#c9983a]' : 'text-[#a67c2e]'
                    }`}>In Review</span>
                </div>
              )}

              {kycStatus === 'rejected' && (
                <div className={`flex items-center gap-2 px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${theme === 'dark'
                  ? 'bg-gradient-to-br from-[#ef4444]/20 to-[#dc2626]/15 border-[#ef4444]/50'
                  : 'bg-gradient-to-br from-[#ef4444]/15 to-[#dc2626]/10 border-[#ef4444]/50'
                  }`}>
                  <AlertCircle className={`w-4 h-4 ${theme === 'dark' ? 'text-[#ef4444]' : 'text-[#dc2626]'}`} />
                  <span className={`text-[13px] font-medium transition-colors ${theme === 'dark' ? 'text-[#ef4444]' : 'text-[#dc2626]'
                    }`}>Verification Rejected</span>
                </div>
              )}

              {selectedProfile.status === 'limit-reached' && (
                <div className={`flex items-center gap-2 px-4 py-2 rounded-[12px] backdrop-blur-[20px] border transition-colors ${theme === 'dark'
                  ? 'bg-gradient-to-br from-[#ef4444]/20 to-[#dc2626]/15 border-[#ef4444]/40'
                  : 'bg-gradient-to-br from-[#ef4444]/15 to-[#dc2626]/10 border-[#ef4444]/35'
                  }`}>
                  <AlertCircle className={`w-4 h-4 ${theme === 'dark' ? 'text-[#ef4444]' : 'text-[#dc2626]'}`} />
                  <span className={`text-[13px] font-medium transition-colors ${theme === 'dark' ? 'text-[#ef4444]' : 'text-[#dc2626]'
                    }`}>Individual Limit Reached</span>
                </div>
              )}
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* First Name */}
              <div>
                <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>
                  {selectedProfile.type === 'organization' ? 'Company Name' : 'First Name'}
                </label>
                <input
                  type="text"
                  value={selectedProfile.firstName || ''}
                  placeholder={selectedProfile.status === 'verified' ? '' : 'Will be filled after KYC'}
                  readOnly
                  className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none text-[14px] transition-colors ${theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]/50'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]/50'
                    }`}
                />
              </div>

              {/* Last Name */}
              <div>
                <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>
                  {selectedProfile.type === 'organization' ? 'Legal Form' : 'Last Name'}
                </label>
                <input
                  type="text"
                  value={selectedProfile.lastName || ''}
                  placeholder={selectedProfile.status === 'verified' ? '' : 'Will be filled after KYC'}
                  readOnly
                  className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none text-[14px] transition-colors ${theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]/50'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]/50'
                    }`}
                />
              </div>

              {/* Address */}
              <div className="md:col-span-2">
                <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>Address</label>
                <input
                  type="text"
                  value={selectedProfile.address || ''}
                  placeholder={selectedProfile.status === 'verified' ? '' : 'Will be filled after KYC'}
                  readOnly
                  className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none text-[14px] transition-colors ${theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]/50'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]/50'
                    }`}
                />
              </div>

              {/* City */}
              <div>
                <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>City</label>
                <input
                  type="text"
                  value={selectedProfile.city || ''}
                  placeholder={selectedProfile.status === 'verified' ? '' : 'Will be filled after KYC'}
                  readOnly
                  className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none text-[14px] transition-colors ${theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]/50'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]/50'
                    }`}
                />
              </div>

              {/* Postal Code */}
              <div>
                <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>Postal Code</label>
                <input
                  type="text"
                  value={selectedProfile.postalCode || ''}
                  placeholder={selectedProfile.status === 'verified' ? '' : 'Will be filled after KYC'}
                  readOnly
                  className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none text-[14px] transition-colors ${theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]/50'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]/50'
                    }`}
                />
              </div>

              {/* Country */}
              <div>
                <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>Country</label>
                <input
                  type="text"
                  value={selectedProfile.country || ''}
                  placeholder={selectedProfile.status === 'verified' ? '' : 'Will be filled after KYC'}
                  readOnly
                  className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none text-[14px] transition-colors ${theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]/50'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]/50'
                    }`}
                />
              </div>

              {/* Tax ID */}
              <div>
                <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>
                  {selectedProfile.type === 'organization' ? 'Company ID Number' : 'Tax ID'}
                </label>
                <input
                  type="text"
                  value={selectedProfile.taxId || ''}
                  placeholder={selectedProfile.status === 'verified' ? '' : 'Will be filled after KYC'}
                  readOnly
                  className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none text-[14px] transition-colors ${theme === 'dark'
                    ? 'bg-[#3d342c]/[0.4] border-white/15 text-[#f5efe5] placeholder-[#8a7e70]/50'
                    : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a]/50'
                    }`}
                />
              </div>
            </div>

            {/* Verify KYC Button - Only show for missing-verification status (not in_review, rejected, or verified) */}
            {selectedProfile.status === 'missing-verification' &&
              kycStatus !== 'in_review' &&
              kycStatus !== 'rejected' &&
              kycStatus !== 'verified' && (
                <div className="mt-8 flex items-center gap-4">
                  <button
                    onClick={handleVerifyKYC}
                    disabled={isVerifying || isCheckingKYC || kycWindowOpened}
                    className="px-8 py-3 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[15px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                  >
                    {isVerifying ? (
                      <>
                        <Loader2 className="w-5 h-5 animate-spin" />
                        Starting Verification...
                      </>
                    ) : isCheckingKYC ? (
                      <>
                        <Loader2 className="w-5 h-5 animate-spin" />
                        Checking Status...
                      </>
                    ) : kycWindowOpened ? (
                      <>
                        <Loader2 className="w-5 h-5 animate-spin" />
                        {kycStatus === 'pending' || kycStatus === 'not_started' 
                          ? 'Verification in Progress...' 
                          : kycStatus === 'in_review' 
                            ? 'Under Review...' 
                            : 'Awaiting Completion...'}
                      </>
                    ) : (
                      'Verify KYC'
                    )}
                  </button>
                  {(isVerifying || kycWindowOpened) && (
                    <span className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
                      }`}>
                      {isVerifying 
                        ? 'A new window will open for verification. Please complete the process there.'
                        : 'Please complete the verification in the opened window.'}
                    </span>
                  )}
                </div>
              )}

            {/* In Review Message */}
            {kycStatus === 'in_review' && (
              <div className={`mt-8 p-4 rounded-[14px] backdrop-blur-[30px] border transition-colors ${theme === 'dark'
                ? 'bg-gradient-to-br from-[#2d2820]/[0.4] to-[#3d342c]/[0.3] border-[#c9983a]/30'
                : 'bg-gradient-to-br from-white/[0.12] to-white/[0.08] border-[#c9983a]/30'
                }`}>
                <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                  Your KYC verification is currently under review. Please wait while we process your information.
                </p>
              </div>
            )}

            {/* Rejected Message */}
            {kycStatus === 'rejected' && (
              <div className={`mt-8 p-4 rounded-[14px] backdrop-blur-[30px] border transition-colors ${theme === 'dark'
                ? 'bg-gradient-to-br from-[#2d2820]/[0.4] to-[#3d342c]/[0.3] border-[#ef4444]/50'
                : 'bg-gradient-to-br from-white/[0.12] to-white/[0.08] border-[#ef4444]/50'
                }`}>
                <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                  Your KYC verification was rejected. Please try again or contact support for assistance.
                </p>
              </div>
            )}

            {/* Info Message for Verified Profiles */}
            {selectedProfile.status === 'verified' && (
              <div className="mt-6 flex items-start gap-2 p-4 rounded-[14px] backdrop-blur-[30px] bg-[#c9983a]/10 border border-[#c9983a]/20">
                <Info className="w-5 h-5 text-[#c9983a] flex-shrink-0 mt-0.5" />
                <p className={`text-[13px] transition-colors ${theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
                  }`}>
                  This profile has been verified. All information is populated from your government ID.
                </p>
              </div>
            )}
          </div>
        )}

        {/* Payment Methods Tab */}
        {detailTab === 'payment' && (
          <PaymentMethodsTab
            paymentMethods={selectedProfile.paymentMethods || []}
            onAddPaymentMethod={handleAddPaymentMethod}
            onRemovePaymentMethod={handleRemovePaymentMethod}
            onSetDefault={handleSetDefaultPaymentMethod}
          />
        )}

        {/* Invoices Tab */}
        {detailTab === 'invoices' && (
          <InvoicesTab invoices={sampleInvoices} />
        )}
      </div>
    );
  }

  // Profile List View
  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className={`text-[28px] font-bold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Billing Profiles</h2>
          <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>Manage your billing profiles and payment information.</p>
        </div>
        <button
          onClick={() => setShowModal(true)}
          className="px-6 py-3 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[15px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10 flex items-center gap-2"
        >
          <Plus className="w-5 h-5" />
          New Profile
        </button>
      </div>

      {/* Profile Grid */}
      {profiles.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {profiles.map((profile) => (
            <BillingProfileCard
              key={profile.id}
              profile={profile}
              onClick={() => setSelectedProfile(profile)}
            />
          ))}
        </div>
      ) : (
        <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-12 text-center transition-colors ${theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
          }`}>
          <div className={`w-16 h-16 rounded-full mx-auto mb-4 flex items-center justify-center ${theme === 'dark' ? 'bg-white/[0.08]' : 'bg-white/[0.15]'
            }`}>
            <Plus className={`w-8 h-8 ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`} />
          </div>
          <p className={`text-[16px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>
            No billing profiles yet
          </p>
          <p className={`text-[14px] transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
            Create your first billing profile to start receiving payments
          </p>
        </div>
      )}

      {/* Create Profile Modal */}
      {showModal && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
          <div className={`relative w-full max-w-md rounded-[24px] border shadow-[0_20px_60px_rgba(0,0,0,0.15)] p-8 backdrop-blur-[40px] ${theme === 'dark'
            ? 'bg-gradient-to-br from-[#2d2820]/[0.4] via-[#3d342c]/[0.4] to-[#2d2820]/[0.4] border-white/10'
            : 'bg-gradient-to-br from-white/[0.12] via-white/[0.15] to-white/[0.12] border-white/20'
            }`}>
            {/* Golden Glow Effects */}
            <div className="absolute inset-0 opacity-15 pointer-events-none">
              <div className="absolute top-1/4 left-1/4 w-32 h-32 bg-[#c9983a]/30 rounded-full blur-[60px]" />
              <div className="absolute bottom-1/4 right-1/4 w-32 h-32 bg-[#d4af37]/25 rounded-full blur-[70px]" />
            </div>

            <div className="relative z-10">
              <div className="flex items-center justify-between mb-6">
                <h3 className={`text-[20px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                  }`}>Create Billing Profile</h3>
                <button onClick={() => setShowModal(false)} className={`w-8 h-8 rounded-[10px] backdrop-blur-[20px] border flex items-center justify-center transition-all ${theme === 'dark'
                  ? 'bg-white/[0.1] hover:bg-white/[0.15] border-white/20'
                  : 'bg-white/[0.3] hover:bg-white/[0.5] border-white/40'
                  }`}>
                  <X className={`w-4 h-4 transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                    }`} />
                </button>
              </div>

              <div className="space-y-4">
                <div>
                  <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                    }`}>Profile Name</label>
                  <input
                    type="text"
                    value={profileName}
                    onChange={(e) => setProfileName(e.target.value)}
                    placeholder="Enter profile name"
                    className={`w-full px-4 py-3 rounded-[14px] backdrop-blur-[30px] border focus:outline-none focus:border-[#c9983a]/30 transition-all text-[14px] ${theme === 'dark'
                      ? 'bg-white/[0.08] border-white/15 text-[#f5efe5] placeholder-[#8a7e70] focus:bg-white/[0.12]'
                      : 'bg-white/[0.15] border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:bg-white/[0.2]'
                      }`}
                  />
                </div>

                <div>
                  <label className={`block text-[14px] font-semibold mb-2 transition-colors ${theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
                    }`}>Profile Type</label>
                  <div className="relative">
                    {(() => {
                      const hasIndividualProfile = profiles.some(p => p.type === 'individual');
                      return (
                        <ProfileTypeSelect
                          value={profileType}
                          onChange={(val) => setProfileType(val)}
                          hasIndividualProfile={hasIndividualProfile}
                          theme={theme}
                        />
                      );
                    })()}
                  </div>
                  {profiles.some(p => p.type === 'individual') && (
                    <p className={`text-[12px] mt-2 transition-colors ${theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#6b7280]'
                      }`}>
                      An individual profile already exists. You can only create one individual profile.
                    </p>
                  )}
                </div>
              </div>

              <div className="flex items-center gap-3 mt-6">
                <button
                  onClick={() => setShowModal(false)}
                  className={`flex-1 px-6 py-3 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] transition-all ${theme === 'dark'
                    ? 'bg-white/[0.1] border-white/20 text-[#d4c5b0] hover:bg-white/[0.15]'
                    : 'bg-white/[0.2] border-white/30 text-[#2d2820] hover:bg-white/[0.25]'
                    }`}
                >
                  Cancel
                </button>
                <button
                  onClick={handleCreateProfile}
                  disabled={profileType === 'individual' && profiles.some(p => p.type === 'individual')}
                  className="flex-1 px-6 py-3 rounded-[12px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[14px] shadow-[0_4px_16px_rgba(162,121,44,0.3)] hover:shadow-[0_6px_20px_rgba(162,121,44,0.4)] transition-all border border-white/10 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Create
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}