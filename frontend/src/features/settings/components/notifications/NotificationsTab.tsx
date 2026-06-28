import { useState } from 'react';
import { Info, Sparkles, CheckCircle2, XCircle, AlertCircle } from 'lucide-react';
import { NotificationSettings, NotificationPreference, NotificationChannel, NotificationPreferenceState } from '../../types';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

const initialPreferences: NotificationPreference[] = [
  {
    event: 'payoutReceived',
    label: 'Payout Received',
    description: 'Get notified when you receive a payout from a project.',
    channels: { inApp: 'enabled', email: 'enabled', push: 'enabled' }
  },
  {
    event: 'programPublished',
    label: 'Program Published',
    description: 'Be informed when a new program or grant is published.',
    channels: { inApp: 'enabled', email: 'enabled', push: 'notAvailable' }
  },
  {
    event: 'bountyClaimed',
    label: 'Bounty Claimed',
    description: 'Get notified when your bounty is claimed or marked as complete.',
    channels: { inApp: 'enabled', email: 'disabled', push: 'enabled' }
  },
  {
    event: 'disputeOpened',
    label: 'Dispute Opened',
    description: 'Receive alerts when a dispute is opened on your contributions.',
    channels: { inApp: 'enabled', email: 'enabled', push: 'enabled' }
  },
  {
    event: 'systemMaintenance',
    label: 'System Maintenance',
    description: 'Be notified about scheduled maintenance and system updates.',
    channels: { inApp: 'enabled', email: 'notAvailable', push: 'notAvailable' }
  }
];

export function NotificationsTab() {
  const { theme } = useTheme();
  const [notifications, setNotifications] = useState<NotificationSettings>({
    preferences: initialPreferences
  });
  const [originalNotifications, setOriginalNotifications] = useState<NotificationSettings>({
    preferences: initialPreferences
  });
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  const getUnsavedCount = () => {
    let count = 0;
    notifications.preferences.forEach((pref, idx) => {
      Object.keys(pref.channels).forEach((channel) => {
        if (pref.channels[channel as NotificationChannel] !== originalNotifications.preferences[idx].channels[channel as NotificationChannel]) {
          count++;
        }
      });
    });
    return count;
  };

  const updatePreference = (eventIdx: number, channel: NotificationChannel, state: NotificationPreferenceState) => {
    if (state === 'notAvailable') return;
    setNotifications(prev => {
      const newPreferences = [...prev.preferences];
      newPreferences[eventIdx] = {
        ...newPreferences[eventIdx],
        channels: {
          ...newPreferences[eventIdx].channels,
          [channel]: state
        }
      };
      const newSettings = { ...prev, preferences: newPreferences };
      const hasChanges = JSON.stringify(newSettings) !== JSON.stringify(originalNotifications);
      setHasUnsavedChanges(hasChanges);
      return newSettings;
    });
  };

  const saveChanges = () => {
    setOriginalNotifications(notifications);
    setHasUnsavedChanges(false);
  };

  const discardChanges = () => {
    setNotifications(originalNotifications);
    setHasUnsavedChanges(false);
  };

  const getChannelLabel = (channel: NotificationChannel) => {
    const labels = { inApp: 'In-app', email: 'Email', push: 'Browser Push' };
    return labels[channel];
  };

  const getStateIcon = (state: NotificationPreferenceState) => {
    if (state === 'enabled') {
      return <CheckCircle2 className="w-4 h-4 text-green-500" />;
    } else if (state === 'disabled') {
      return <XCircle className="w-4 h-4 text-gray-500" />;
    }
    return <AlertCircle className="w-4 h-4 text-gray-400" />;
  };

  const getStateLabel = (state: NotificationPreferenceState) => {
    const labels = { enabled: 'On', disabled: 'Off', notAvailable: 'N/A' };
    return labels[state];
  };

  const unsavedCount = getUnsavedCount();

  return (
    <div className="relative pb-32">
      {/* Header */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 mb-6 transition-colors ${
        theme === 'dark' ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
      }`}>
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
          <div>
            <h2 className={`text-[28px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Notification Preferences</h2>
            <p className={`text-[14px] transition-colors ${
              theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>Customize how you receive updates across different channels.</p>
          </div>
        </div>
      </div>

      {/* Desktop Table */}
      <div className={`hidden md:block backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark' ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
      }`}>
        {/* Table Header */}
        <div className="grid grid-cols-[2fr_1fr_1fr_1fr] gap-4 pb-4 border-b border-white/10">
          <div></div>
          {(['inApp', 'email', 'push'] as NotificationChannel[]).map(channel => (
            <div key={channel} className={`text-[13px] font-semibold text-center transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>
              {getChannelLabel(channel)}
            </div>
          ))}
        </div>

        {/* Table Body */}
        {notifications.preferences.map((pref, idx) => (
          <div key={pref.event} className="grid grid-cols-[2fr_1fr_1fr_1fr] gap-4 items-center py-5 border-b border-white/10 last:border-b-0">
            <div>
              <div className={`text-[15px] font-semibold mb-1 transition-colors ${
                theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
              }`}>{pref.label}</div>
              <div className={`text-[13px] transition-colors ${
                theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>{pref.description}</div>
            </div>
            {(['inApp', 'email', 'push'] as NotificationChannel[]).map(channel => (
              <div key={channel} className="flex justify-center">
                <button
                  onClick={() => {
                    const currentState = pref.channels[channel];
                    const newState: NotificationPreferenceState = currentState === 'enabled' ? 'disabled' : 'enabled';
                    updatePreference(idx, channel, newState);
                  }}
                  disabled={pref.channels[channel] === 'notAvailable'}
                  className={`flex items-center gap-2 px-4 py-2 rounded-[12px] border font-medium text-[13px] transition-all ${
                    pref.channels[channel] === 'enabled'
                      ? 'bg-green-500/20 border-green-500/30 text-green-400 hover:bg-green-500/30'
                      : pref.channels[channel] === 'disabled'
                      ? 'bg-gray-500/10 border-gray-500/20 text-gray-400 hover:bg-gray-500/20'
                      : 'bg-gray-500/5 border-gray-500/10 text-gray-500 cursor-not-allowed'
                  } ${theme === 'dark' ? '' : ''}`}
                >
                  {getStateIcon(pref.channels[channel])}
                  <span>{getStateLabel(pref.channels[channel])}</span>
                </button>
              </div>
            ))}
          </div>
        ))}
      </div>

      {/* Mobile Stacked Cards */}
      <div className={`md:hidden space-y-4`}>
        {notifications.preferences.map((pref, idx) => (
          <div key={pref.event} className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-6 transition-colors ${
            theme === 'dark' ? 'bg-[#2d2820]/[0.4] border-white/10' : 'bg-white/[0.12] border-white/20'
          }`}>
            <div className="mb-4">
              <div className={`text-[15px] font-semibold mb-1 transition-colors ${
                theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
              }`}>{pref.label}</div>
              <div className={`text-[13px] transition-colors ${
                theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>{pref.description}</div>
            </div>
            <div className="space-y-3">
              {(['inApp', 'email', 'push'] as NotificationChannel[]).map(channel => (
                <div key={channel} className="flex items-center justify-between">
                  <span className={`text-[14px] transition-colors ${
                    theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#4a3d2f]'
                  }`}>{getChannelLabel(channel)}</span>
                  <button
                    onClick={() => {
                      const currentState = pref.channels[channel];
                      const newState: NotificationPreferenceState = currentState === 'enabled' ? 'disabled' : 'enabled';
                      updatePreference(idx, channel, newState);
                    }}
                    disabled={pref.channels[channel] === 'notAvailable'}
                    className={`flex items-center gap-2 px-4 py-2 rounded-[12px] border font-medium text-[13px] transition-all ${
                      pref.channels[channel] === 'enabled'
                        ? 'bg-green-500/20 border-green-500/30 text-green-400 hover:bg-green-500/30'
                        : pref.channels[channel] === 'disabled'
                        ? 'bg-gray-500/10 border-gray-500/20 text-gray-400 hover:bg-gray-500/20'
                        : 'bg-gray-500/5 border-gray-500/10 text-gray-500 cursor-not-allowed'
                    }`}
                  >
                    {getStateIcon(pref.channels[channel])}
                    <span>{getStateLabel(pref.channels[channel])}</span>
                  </button>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>

      {/* Sticky Footer */}
      {hasUnsavedChanges && (
        <div className={`fixed bottom-0 left-0 right-0 backdrop-blur-[40px] border-t shadow-[0_-8px_32px_rgba(0,0,0,0.08)] p-6 transition-colors ${
          theme === 'dark' ? 'bg-[#2d2820]/[0.95] border-white/10' : 'bg-white/[0.95] border-white/20'
        }`}>
          <div className="max-w-4xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <div className={`px-3 py-1 rounded-full font-semibold text-[13px] ${
                theme === 'dark'
                  ? 'bg-[#c9983a]/20 text-[#c9983a]'
                  : 'bg-[#c9983a]/15 text-[#a67c2e]'
              }`}>
                {unsavedCount} unsaved change{unsavedCount !== 1 ? 's' : ''}
              </div>
              <span className={`text-[14px] transition-colors ${
                theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>You have unsaved changes</span>
            </div>
            <div className="flex items-center gap-3">
              <button 
                onClick={discardChanges}
                className={`px-6 py-2.5 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] hover:bg-white/[0.25] transition-all ${
                  theme === 'dark'
                    ? 'bg-[#3d342c]/[0.5] border-white/20 text-[#d4c5b0]'
                    : 'bg-white/[0.2] border-white/30 text-[#2d2820]'
                }`}
              >
                Discard
              </button>
              <button 
                onClick={saveChanges}
                className="px-8 py-3 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[15px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10"
              >
                Save Changes
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
