import { useTheme } from '../../../../shared/contexts/ThemeContext';

export function TermsTab() {
  const { theme } = useTheme();

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <h2 className={`text-[28px] font-bold mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}>Terms and Conditions</h2>
        <p className={`text-[14px] transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>Review our terms of service and privacy policy.</p>
      </div>

      {/* Terms Content */}
      <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <div className="prose prose-sm max-w-none">
          <h3 className={`text-[20px] font-bold mb-4 transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>Terms of Service</h3>
          <p className={`text-[14px] leading-relaxed mb-6 transition-colors ${
            theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
          }`}>
            By using Grainlify, you agree to abide by our terms of service. These terms govern your use of the platform and outline your rights and responsibilities as a user.
          </p>

          <h3 className={`text-[20px] font-bold mb-4 mt-8 transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>Privacy Policy</h3>
          <p className={`text-[14px] leading-relaxed mb-6 transition-colors ${
            theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
          }`}>
            We take your privacy seriously. Our privacy policy explains how we collect, use, and protect your personal information.
          </p>

          <h3 className={`text-[20px] font-bold mb-4 mt-8 transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>Data Collection</h3>
          <p className={`text-[14px] leading-relaxed mb-6 transition-colors ${
            theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
          }`}>
            We collect information necessary to provide our services, including your GitHub profile data, contribution history, and reward preferences.
          </p>

          <h3 className={`text-[20px] font-bold mb-4 mt-8 transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>User Responsibilities</h3>
          <p className={`text-[14px] leading-relaxed mb-6 transition-colors ${
            theme === 'dark' ? 'text-[#c5b5a2]' : 'text-[#6b5d4d]'
          }`}>
            Users are responsible for maintaining the security of their accounts, providing accurate information, and complying with all applicable laws and regulations.
          </p>
        </div>
      </div>

      {/* Acceptance */}
      <div className={`flex items-center justify-between backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
        theme === 'dark'
          ? 'bg-[#2d2820]/[0.4] border-white/10'
          : 'bg-white/[0.12] border-white/20'
      }`}>
        <div>
          <h3 className={`text-[16px] font-bold mb-1 transition-colors ${
            theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
          }`}>Accept Terms</h3>
          <p className={`text-[13px] transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}>
            By clicking accept, you agree to our terms of service and privacy policy.
          </p>
        </div>
        <button className="px-8 py-3 rounded-[16px] bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white font-semibold text-[15px] shadow-[0_6px_24px_rgba(162,121,44,0.4)] hover:shadow-[0_8px_28px_rgba(162,121,44,0.5)] transition-all border border-white/10">
          Accept
        </button>
      </div>
    </div>
  );
}