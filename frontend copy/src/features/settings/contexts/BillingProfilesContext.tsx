import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { BillingProfile } from '../types';

interface BillingProfilesContextType {
  profiles: BillingProfile[];
  setProfiles: (profiles: BillingProfile[]) => void;
  addProfile: (profile: BillingProfile) => void;
  updateProfile: (id: number, updates: Partial<BillingProfile>) => void;
}

const BillingProfilesContext = createContext<BillingProfilesContextType | undefined>(undefined);

const STORAGE_KEY = 'billing_profiles';

function loadProfilesFromStorage(): BillingProfile[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      return JSON.parse(stored);
    }
  } catch (error) {
    console.error('Failed to load billing profiles from storage:', error);
  }
  return [];
}

function saveProfilesToStorage(profiles: BillingProfile[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles));
  } catch (error) {
    console.error('Failed to save billing profiles to storage:', error);
  }
}

export function BillingProfilesProvider({ children }: { children: ReactNode }) {
  const [profiles, setProfiles] = useState<BillingProfile[]>(loadProfilesFromStorage);

  // Load profiles from storage on mount
  useEffect(() => {
    const loaded = loadProfilesFromStorage();
    if (loaded.length > 0) {
      setProfiles(loaded);
    }
  }, []);

  // Save profiles to storage whenever they change
  useEffect(() => {
    saveProfilesToStorage(profiles);
  }, [profiles]);

  const addProfile = (profile: BillingProfile) => {
    setProfiles((prev) => {
      const updated = [...prev, profile];
      saveProfilesToStorage(updated);
      return updated;
    });
  };

  const updateProfile = (id: number, updates: Partial<BillingProfile>) => {
    setProfiles((prev) => {
      const updated = prev.map((p) => (p.id === id ? { ...p, ...updates } : p));
      saveProfilesToStorage(updated);
      return updated;
    });
  };

  const setProfilesWithStorage = (newProfiles: BillingProfile[]) => {
    setProfiles(newProfiles);
    saveProfilesToStorage(newProfiles);
  };

  return (
    <BillingProfilesContext.Provider
      value={{ profiles, setProfiles: setProfilesWithStorage, addProfile, updateProfile }}
    >
      {children}
    </BillingProfilesContext.Provider>
  );
}

export function useBillingProfiles() {
  const context = useContext(BillingProfilesContext);
  if (context === undefined) {
    throw new Error('useBillingProfiles must be used within a BillingProfilesProvider');
  }
  return context;
}

