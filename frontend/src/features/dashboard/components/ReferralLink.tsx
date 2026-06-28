import { useState } from 'react';
import { Share2, Copy, Check, Twitter, Linkedin, Send, Users, Gift, TrendingUp } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';
import { useAuth } from '../../../shared/contexts/AuthContext';

export function ReferralLink() {
  const { theme } = useTheme();
  const { user } = useAuth();
  const [isCopied, setIsCopied] = useState(false);

  // Generate a unique referral link
  const referralLink = `${window.location.origin}/?ref=${user?.github?.login || 'guest'}`;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(referralLink);
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  const shareToTwitter = () => {
    const text = encodeURIComponent(`Check out Grainlify - a platform for open source contributions and bounties! Use my referral link to join: ${referralLink}`);
    window.open(`https://twitter.com/intent/tweet?text=${text}`, '_blank');
  };

  const shareToLinkedIn = () => {
    const url = encodeURIComponent(referralLink);
    const text = encodeURIComponent(`Check out Grainlify - a platform for open source contributions and bounties!`);
    window.open(`https://www.linkedin.com/sharing/share-offsite/?url=${url}`, '_blank');
  };

  const shareToTelegram = () => {
    const text = encodeURIComponent(`Check out Grainlify - a platform for open source contributions and bounties! Use my referral link: ${referralLink}`);
    window.open(`https://t.me/share/url?url=${encodeURIComponent(referralLink)}&text=${text}`, '_blank');
  };

  // Mock stats - in a real app, these would come from an API
  const stats = {
    totalReferrals: 12,
    pendingReferrals: 3,
    earnedBonus: 150
  };

  return (
    <div className="backdrop-blur-[40px] bg-gradient-to-br from-white/[0.18] to-white/[0.10] rounded-[32px] border-2 border-white/30 shadow-[0_20px_60px_rgba(0,0,0,0.15),0_0_80px_rgba(201,152,58,0.08)] p-8 relative overflow-visible group">
      {/* Ambient Background Glow */}
      <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-gradient-to-br from-[#c9983a]/15 via-[#d4af37]/10 to-transparent rounded-full blur-3xl pointer-events-none group-hover:scale-110 transition-transform duration-1000" />
      <div className="absolute bottom-0 left-0 w-[400px] h-[400px] bg-gradient-to-tr from-[#d4af37]/12 to-transparent rounded-full blur-3xl pointer-events-none group-hover:scale-110 transition-transform duration-1000" />

      <div className="relative z-10 space-y-8">
        {/* Header */}
        <div className="flex items-center gap-4">
          <div className="w-14 h-14 rounded-[18px] bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-2 border-[#c9983a]/60 flex items-center justify-center shadow-[0_4px_16px_rgba(201,152,58,0.3)]">
            <Share2 className="w-7 h-7 text-[#c9983a]" />
          </div>
          <div>
            <h2 className={`text-[26px] font-black leading-tight transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
              Invite & Earn
            </h2>
            <p className={`text-sm font-medium transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'}`}>
              Refer friends and earn bonuses when they join
            </p>
          </div>
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="backdrop-blur-[25px] bg-gradient-to-br from-white/[0.12] to-white/[0.08] rounded-[20px] border border-white/20 p-5">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-[14px] bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border border-[#c9983a]/40 flex items-center justify-center">
                <Users className="w-5 h-5 text-[#c9983a]" />
              </div>
              <div className="flex-1">
                <p className={`text-xs font-semibold uppercase tracking-wider transition-colors ${theme === 'dark' ? 'text-[#9b8d7f]' : 'text-[#7a6b5a]'}`}>
                  Total Referrals
                </p>
                <p className={`text-[28px] font-black leading-tight transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                  {stats.totalReferrals}
                </p>
              </div>
            </div>
          </div>

          <div className="backdrop-blur-[25px] bg-gradient-to-br from-white/[0.12] to-white/[0.08] rounded-[20px] border border-white/20 p-5">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-[14px] bg-gradient-to-br from-[#f59e0b]/30 to-[#d97706]/20 border border-[#f59e0b]/40 flex items-center justify-center">
                <TrendingUp className="w-5 h-5 text-[#f59e0b]" />
              </div>
              <div className="flex-1">
                <p className={`text-xs font-semibold uppercase tracking-wider transition-colors ${theme === 'dark' ? 'text-[#9b8d7f]' : 'text-[#7a6b5a]'}`}>
                  Pending
                </p>
                <p className={`text-[28px] font-black leading-tight transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                  {stats.pendingReferrals}
                </p>
              </div>
            </div>
          </div>

          <div className="backdrop-blur-[25px] bg-gradient-to-br from-white/[0.12] to-white/[0.08] rounded-[20px] border border-white/20 p-5">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-10 h-10 rounded-[14px] bg-gradient-to-br from-[#22c55e]/30 to-[#16a34a]/20 border border-[#22c55e]/40 flex items-center justify-center">
                <Gift className="w-5 h-5 text-[#22c55e]" />
              </div>
              <div className="flex-1">
                <p className={`text-xs font-semibold uppercase tracking-wider transition-colors ${theme === 'dark' ? 'text-[#9b8d7f]' : 'text-[#7a6b5a]'}`}>
                  Earned (USD)
                </p>
                <p className={`text-[28px] font-black leading-tight transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>
                  ${stats.earnedBonus}
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Referral Link Section */}
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <label className={`text-sm font-semibold transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#57534e]'}`}>
              Your Referral Link
            </label>
          </div>

          <div className="flex flex-col sm:flex-row gap-3">
            <div className="flex-1 backdrop-blur-[25px] bg-white/[0.10] border-2 border-white/20 rounded-[16px] px-5 py-4 flex items-center gap-3">
              <div className="flex-1 min-w-0">
                <p className={`text-sm font-semibold truncate transition-colors ${theme === 'dark' ? 'text-[#e7e5e4]' : 'text-[#44403c]'}`}>
                  {referralLink}
                </p>
              </div>
            </div>

            <button
              onClick={handleCopy}
              className={`px-6 py-4 rounded-[16px] font-bold flex items-center justify-center gap-2 transition-all duration-200 min-w-[140px] ${
                isCopied
                  ? 'bg-gradient-to-br from-[#22c55e] to-[#16a34a] text-white shadow-[0_4px_16px_rgba(34,197,94,0.4)]'
                  : 'bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white hover:shadow-[0_4px_16px_rgba(201,152,58,0.4)]'
              }`}
            >
              {isCopied ? <Check className="w-5 h-5" /> : <Copy className="w-5 h-5" />}
              <span>{isCopied ? 'Copied!' : 'Copy Link'}</span>
            </button>
          </div>
        </div>

        {/* Social Share Buttons */}
        <div className="space-y-4">
          <div className="flex items-center gap-3">
            <label className={`text-sm font-semibold transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#57534e]'}`}>
              Share to Social
            </label>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <button
              onClick={shareToTwitter}
              className="backdrop-blur-[25px] bg-white/[0.10] border-2 border-white/20 rounded-[16px] px-5 py-4 flex items-center justify-center gap-3 hover:bg-white/[0.15] hover:border-white/30 transition-all duration-200"
            >
              <Twitter className="w-5 h-5 text-[#c9983a]" />
              <span className={`font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#44403c]'}`}>
                X/Twitter
              </span>
            </button>

            <button
              onClick={shareToLinkedIn}
              className="backdrop-blur-[25px] bg-white/[0.10] border-2 border-white/20 rounded-[16px] px-5 py-4 flex items-center justify-center gap-3 hover:bg-white/[0.15] hover:border-white/30 transition-all duration-200"
            >
              <Linkedin className="w-5 h-5 text-[#c9983a]" />
              <span className={`font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#44403c]'}`}>
                LinkedIn
              </span>
            </button>

            <button
              onClick={shareToTelegram}
              className="backdrop-blur-[25px] bg-white/[0.10] border-2 border-white/20 rounded-[16px] px-5 py-4 flex items-center justify-center gap-3 hover:bg-white/[0.15] hover:border-white/30 transition-all duration-200"
            >
              <Send className="w-5 h-5 text-[#c9983a]" />
              <span className={`font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#44403c]'}`}>
                Telegram
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
