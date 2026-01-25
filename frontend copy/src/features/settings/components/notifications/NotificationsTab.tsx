import { useState } from 'react';
import { Info, Sparkles } from 'lucide-react';
import { NotificationSettings } from '../../types';
import { NotificationSection } from './NotificationSection';
import { NotificationRow } from './NotificationRow';
import { useTheme } from '../../../../shared/contexts/ThemeContext';

export function NotificationsTab() {
  const { theme } = useTheme();
  const [notifications, setNotifications] = useState<NotificationSettings>({
    // Global
    globalBillingEmail: false,
    globalBillingWeekly: false,
    globalMarketingEmail: false,
    globalMarketingWeekly: false,
    
    // Contributor
    contributorProjectEmail: false,
    contributorProjectWeekly: false,
    contributorRewardEmail: false,
    contributorRewardWeekly: false,
    contributorRewardAcceptedEmail: false,
    contributorRewardAcceptedWeekly: false,
    
    // Maintainer
    maintainerProjectContributorEmail: false,
    maintainerProjectContributorWeekly: false,
    maintainerProjectProgramEmail: false,
    maintainerProjectProgramWeekly: false,
    
    // Programs
    programsTransactionsEmail: false,
    programsTransactionsWeekly: false,
    
    // Sponsors
    sponsorsTransactionsEmail: false,
    sponsorsTransactionsWeekly: false,
  });

  const updateNotification = (key: keyof NotificationSettings, value: boolean) => {
    setNotifications(prev => ({ ...prev, [key]: value }));
  };

  const enableAll = () => {
    const allEnabled = Object.keys(notifications).reduce((acc, key) => {
      acc[key as keyof NotificationSettings] = true;
      return acc;
    }, {} as NotificationSettings);
    setNotifications(allEnabled);
  };

  const disableAll = () => {
    const allDisabled = Object.keys(notifications).reduce((acc, key) => {
      acc[key as keyof NotificationSettings] = false;
      return acc;
    }, {} as NotificationSettings);
    setNotifications(allDisabled);
  };

  return (
    <div className="relative h-[calc(100vh-300px)] min-h-[500px] overflow-hidden">
      {/* Blurred Content */}
      <div className="blur-sm pointer-events-none select-none h-full overflow-y-auto">
        <div className="space-y-6">
          {/* Header */}
          <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
            theme === 'dark'
              ? 'bg-[#2d2820]/[0.4] border-white/10'
              : 'bg-white/[0.12] border-white/20'
          }`}>
        <div className="flex items-start justify-between mb-6">
          <div>
            <h2 className={`text-[28px] font-bold mb-2 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>Notification Preferences</h2>
            <p className={`text-[14px] transition-colors ${
              theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>Customize how you receive virtually for any remote-based updates.</p>
          </div>
          <div className="flex items-center gap-3">
            <button 
              onClick={enableAll}
              className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] hover:bg-white/[0.25] transition-all ${
                theme === 'dark'
                  ? 'bg-[#3d342c]/[0.5] border-white/20 text-[#d4c5b0]'
                  : 'bg-white/[0.2] border-white/30 text-[#2d2820]'
              }`}
            >
              Enable all
            </button>
            <button 
              onClick={disableAll}
              className={`px-5 py-2.5 rounded-[12px] backdrop-blur-[30px] border font-medium text-[14px] hover:bg-white/[0.25] transition-all ${
                theme === 'dark'
                  ? 'bg-[#3d342c]/[0.5] border-white/20 text-[#d4c5b0]'
                  : 'bg-white/[0.2] border-white/30 text-[#2d2820]'
              }`}
            >
              Disable all
            </button>
          </div>
        </div>

        {/* Column Headers */}
        <div className="grid grid-cols-[1fr_200px_220px] gap-4 pb-4 border-b border-white/10">
          <div></div>
          <div className={`text-[13px] font-semibold text-center transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>Email Notification</div>
          <div className={`text-[13px] font-semibold text-center flex items-center justify-center gap-2 transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>
            Weekly Summary Email
            <Info className={`w-4 h-4 transition-colors ${
              theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#7a6b5a]'
            }`} />
          </div>
        </div>
      </div>

      {/* Global Section */}
      <NotificationSection title="Global">
        <NotificationRow
          title="Billing Profile"
          description="You now have billing profile with notifications for allocation reminders and identity checks."
          emailEnabled={notifications.globalBillingEmail}
          weeklyEnabled={notifications.globalBillingWeekly}
          onEmailChange={(val) => updateNotification('globalBillingEmail', val)}
          onWeeklyChange={(val) => updateNotification('globalBillingWeekly', val)}
        />
        <NotificationRow
          title="Marketing"
          description="Receive updates and announcements sent occasionally by clicking the subscribe bar."
          emailEnabled={notifications.globalMarketingEmail}
          weeklyEnabled={notifications.globalMarketingWeekly}
          onEmailChange={(val) => updateNotification('globalMarketingEmail', val)}
          onWeeklyChange={(val) => updateNotification('globalMarketingWeekly', val)}
          showBorder={false}
        />
      </NotificationSection>

      {/* Contributor Section */}
      <NotificationSection title="Contributor">
        <NotificationRow
          title="Project"
          description="Stay informed about project-related updates, including new published and available issues."
          emailEnabled={notifications.contributorProjectEmail}
          weeklyEnabled={notifications.contributorProjectWeekly}
          onEmailChange={(val) => updateNotification('contributorProjectEmail', val)}
          onWeeklyChange={(val) => updateNotification('contributorProjectWeekly', val)}
        />
        <NotificationRow
          title="Reward"
          description="Receive updates on new billings and when rewards from issues to attention."
          emailEnabled={notifications.contributorRewardEmail}
          weeklyEnabled={notifications.contributorRewardWeekly}
          onEmailChange={(val) => updateNotification('contributorRewardEmail', val)}
          onWeeklyChange={(val) => updateNotification('contributorRewardWeekly', val)}
        />
        <NotificationRow
          title="Reward"
          description="All rewards are open for your reward to evaluation."
          emailEnabled={notifications.contributorRewardAcceptedEmail}
          weeklyEnabled={notifications.contributorRewardAcceptedWeekly}
          onEmailChange={(val) => updateNotification('contributorRewardAcceptedEmail', val)}
          onWeeklyChange={(val) => updateNotification('contributorRewardAcceptedWeekly', val)}
          showBorder={false}
        />
      </NotificationSection>

      {/* Maintainer Section */}
      <NotificationSection title="Maintainer">
        <NotificationRow
          title="Project + Contributor Data"
          description="Receive notifications upon new applications and contributions from contributors."
          emailEnabled={notifications.maintainerProjectContributorEmail}
          weeklyEnabled={notifications.maintainerProjectContributorWeekly}
          onEmailChange={(val) => updateNotification('maintainerProjectContributorEmail', val)}
          onWeeklyChange={(val) => updateNotification('maintainerProjectContributorWeekly', val)}
        />
        <NotificationRow
          title="Project + Program"
          description="Get updates on new grants and contributor applications within your projects."
          emailEnabled={notifications.maintainerProjectProgramEmail}
          weeklyEnabled={notifications.maintainerProjectProgramWeekly}
          onEmailChange={(val) => updateNotification('maintainerProjectProgramEmail', val)}
          onWeeklyChange={(val) => updateNotification('maintainerProjectProgramWeekly', val)}
          showBorder={false}
        />
      </NotificationSection>

      {/* Programs Section */}
      <NotificationSection title="Programs">
        <NotificationRow
          title="Transactions"
          description="Receive periodic updates about deposits & withdrawals."
          emailEnabled={notifications.programsTransactionsEmail}
          weeklyEnabled={notifications.programsTransactionsWeekly}
          onEmailChange={(val) => updateNotification('programsTransactionsEmail', val)}
          onWeeklyChange={(val) => updateNotification('programsTransactionsWeekly', val)}
          showBorder={false}
        />
      </NotificationSection>

      {/* Sponsors Section */}
      <NotificationSection title="Sponsors">
        <NotificationRow
          title="Transactions"
          description="Get updates whenever spend about deposits & allocations."
          emailEnabled={notifications.sponsorsTransactionsEmail}
          weeklyEnabled={notifications.sponsorsTransactionsWeekly}
          onEmailChange={(val) => updateNotification('sponsorsTransactionsEmail', val)}
          onWeeklyChange={(val) => updateNotification('sponsorsTransactionsWeekly', val)}
          showBorder={false}
        />
      </NotificationSection>

          {/* Save Button */}
          <div className="flex justify-end">
            <button className="px-8 py-3 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[15px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10">
              Save
            </button>
          </div>
        </div>
      </div>

      {/* Coming Soon Modal Overlay */}
      <div className="absolute inset-0 flex items-center justify-center z-50 pointer-events-auto">
        <div className={`backdrop-blur-[40px] rounded-[32px] border shadow-[0_20px_60px_rgba(0,0,0,0.3)] p-10 max-w-lg w-full mx-4 transition-colors relative overflow-hidden ${
          theme === 'dark'
            ? 'bg-gradient-to-br from-[#2d2820]/[0.95] via-[#3d342c]/[0.95] to-[#2d2820]/[0.95] border-[#c9983a]/30'
            : 'bg-gradient-to-br from-white/[0.15] via-white/[0.18] to-white/[0.15] border-[#c9983a]/40'
        }`}>
          {/* Golden Glow Effects */}
          <div className="absolute inset-0 opacity-20 pointer-events-none">
            <div className="absolute top-1/4 left-1/4 w-32 h-32 bg-[#c9983a]/40 rounded-full blur-[60px]" />
            <div className="absolute bottom-1/4 right-1/4 w-32 h-32 bg-[#d4af37]/30 rounded-full blur-[70px]" />
          </div>
          <div className="text-center relative z-10">
            {/* Icon */}
            <div className="w-16 h-16 mx-auto mb-5 rounded-full bg-gradient-to-br from-[#c9983a]/30 via-[#d4af37]/25 to-[#c9983a]/20 border-2 border-[#c9983a]/50 flex items-center justify-center shadow-[0_10px_30px_rgba(201,152,58,0.3)]">
              <Sparkles className="w-8 h-8 text-[#c9983a] animate-pulse" />
            </div>
            
            {/* Title */}
            <h3 className={`text-[28px] font-black mb-3 transition-colors ${
              theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
            }`}>
              Coming Soon
            </h3>
            
            {/* Description */}
            <p className={`text-[15px] leading-relaxed mb-6 transition-colors ${
              theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>
              Notification preferences are currently under development. We're working hard to bring you a comprehensive notification system soon!
            </p>
            
            {/* Decorative Elements */}
            <div className="flex items-center justify-center gap-2">
              <div className="w-2 h-2 rounded-full bg-[#c9983a] animate-pulse" style={{ animationDelay: '0s' }} />
              <div className="w-2 h-2 rounded-full bg-[#c9983a] animate-pulse" style={{ animationDelay: '0.2s' }} />
              <div className="w-2 h-2 rounded-full bg-[#c9983a] animate-pulse" style={{ animationDelay: '0.4s' }} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}