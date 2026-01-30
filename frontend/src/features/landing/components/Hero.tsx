import { Link } from "react-router-dom";
import { ArrowRight, Sparkles } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { useLandingStats } from "../../../shared/hooks/useLandingStats";

export function Hero() {
  const { theme } = useTheme();
  const { display } = useLandingStats();

  return (
    <section className="relative min-h-screen flex items-center justify-center px-4 sm:px-6 pt-20">
      {/* Golden Glassmorphism Orbs (hidden on very small screens to avoid overflow) */}
      <div className="hidden sm:block absolute top-1/4 left-1/4 w-64 sm:w-96 h-64 sm:h-96 rounded-full bg-[#c9983a]/30 blur-3xl animate-pulse" />
      <div className="hidden sm:block absolute bottom-1/4 right-1/4 w-64 sm:w-96 h-64 sm:h-96 rounded-full bg-[#d4af37]/20 blur-3xl animate-pulse delay-1000" />

      {/* Content */}
      <div className="relative z-10 max-w-6xl mx-auto text-center">
        {/* Badge */}
        <div
          className={`inline-flex items-center space-x-2 px-4 py-2 rounded-full backdrop-blur-[30px] border mb-8 transition-colors ${
            theme === "dark"
              ? "bg-white/[0.08] border-white/15"
              : "bg-white/[0.15] border-white/25"
          }`}
        >
          <Sparkles className="w-4 h-4 text-[#c9983a]" />
          <span
            className={`text-sm font-medium transition-colors ${
              theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
            }`}
          >
            Web3 Contributors Platform
          </span>
        </div>

        {/* Heading */}
        <h1
          className={`text-3xl sm:text-4xl md:text-6xl lg:text-7xl font-bold mb-6 leading-tight transition-colors ${
            theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
          }`}
        >
          Connect with
          <span className="bg-gradient-to-r from-[#c9983a] to-[#d4af37] bg-clip-text text-transparent">
            {" "}
            Open Source
          </span>
          <br />
          Opportunities
        </h1>

        {/* Description */}
        <p
          className={`text-base sm:text-lg max-w-2xl mx-auto mb-8 sm:mb-12 transition-colors ${
            theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
          }`}
        >
          Grainlify bridges the gap between talented contributors and innovative
          projects, making open-source collaboration seamless and rewarding.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4 w-full max-w-2xl mx-auto">
          <Link
            to="/signin"
            className="group w-full sm:w-auto px-6 sm:px-8 py-3 sm:py-4 rounded-[16px] bg-gradient-to-r from-[#c9983a] to-[#d4af37] text-white font-medium hover:shadow-2xl hover:shadow-[#c9983a]/50 transition-all flex items-center justify-center sm:inline-flex space-x-2"
          >
            <span>Get Started</span>
            <ArrowRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
          </Link>
          <button
            className={`w-full sm:w-auto px-6 sm:px-8 py-3 sm:py-4 rounded-[16px] backdrop-blur-[30px] border font-medium transition-all ${
              theme === "dark"
                ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12] hover:border-[#c9983a]/30"
                : "bg-white/[0.15] border-white/25 text-[#2d2820] hover:bg-white/[0.2] hover:border-[#c9983a]/30"
            }`}
          >
            Docs
          </button>
        </div>

        {/* Stats */}
        <div className="mt-12 sm:mt-16 grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-6 max-w-4xl mx-auto px-2">
          {[
            { label: "Active Projects", value: display.activeProjects },
            { label: "Contributors", value: display.contributors },
            { label: "Grants Distributed", value: display.grantsDistributed },
          ].map((stat) => (
            <div
              key={stat.label}
              className={`backdrop-blur-[40px] border rounded-[20px] p-4 sm:p-6 transition-all hover:border-[#c9983a]/30 hover:shadow-[0_12px_36px_rgba(201,152,58,0.15)] ${
                theme === "dark"
                  ? "bg-white/[0.08] border-white/15 hover:bg-white/[0.12]"
                  : "bg-white/[0.15] border-white/25 hover:bg-white/[0.2]"
              }`}
            >
              <div
                className={`text-3xl font-bold mb-2 transition-colors ${
                  theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                }`}
              >
                {stat.value}
              </div>
              <div
                className={`transition-colors ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                {stat.label}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
