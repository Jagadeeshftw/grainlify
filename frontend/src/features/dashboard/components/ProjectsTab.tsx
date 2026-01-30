import { Github, ChevronDown } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";

export function ProjectsTab() {
  const { theme } = useTheme();

  // Language icon helper
  const getLanguageIcon = (language: string) => {
    const icons: { [key: string]: { icon: string; color: string } } = {
      TypeScript: { icon: "TS", color: "bg-blue-500" },
      JavaScript: { icon: "JS", color: "bg-yellow-500" },
      Python: { icon: "Py", color: "bg-green-600" },
      Java: { icon: "Jv", color: "bg-red-600" },
      Rust: { icon: "Rs", color: "bg-orange-600" },
      Go: { icon: "Go", color: "bg-cyan-600" },
    };
    return (
      icons[language] || {
        icon: language.substring(0, 2),
        color: "bg-gray-500",
      }
    );
  };

  const projects = [
    {
      id: 1,
      name: "React Ecosystem",
      logo: "⚛️",
      lead: "project-lead-1",
      contributors: 1250,
      availableIssues: "3 GFI",
      myContributions: 15,
      myRewards: "3,600 USD",
      languages: ["TypeScript", "JavaScript"],
      repository: "react-ecosystem/react-core",
      billingProfile: "React Foundation",
    },
    {
      id: 2,
      name: "Next.js Framework",
      logo: "▲",
      lead: "project-lead-2",
      contributors: 890,
      availableIssues: "2 GFI",
      myContributions: 8,
      myRewards: "2,640 USD",
      languages: ["TypeScript", "JavaScript"],
      repository: "vercel/next.js",
      billingProfile: "Vercel Inc.",
    },
    {
      id: 3,
      name: "Vue.js",
      logo: "V",
      lead: "project-lead-3",
      contributors: 650,
      availableIssues: "3 GFI",
      myContributions: 15,
      myRewards: "1,800 USD",
      languages: ["TypeScript", "JavaScript"],
      repository: "vuejs/vue",
      billingProfile: "Vue.js Foundation",
    },
    {
      id: 4,
      name: "Express.js",
      logo: "E",
      lead: "project-lead-4",
      contributors: 350,
      availableIssues: "2 GFI",
      myContributions: 8,
      myRewards: "1,260 USD",
      languages: ["JavaScript"],
      repository: "expressjs/express",
      billingProfile: "Express.js Foundation",
    },
    {
      id: 5,
      name: "Django",
      logo: "D",
      lead: "project-lead-5",
      contributors: 2820,
      availableIssues: "3 GFI",
      myContributions: 10,
      myRewards: "2,800 USD",
      languages: ["Python"],
      repository: "django/django",
      billingProfile: "Django Software Foundation",
    },
  ];

  return (
    <>
      {/* Desktop Table View - Hidden on Mobile */}
      <div className="hidden md:block backdrop-blur-[30px] bg-white/[0.12] rounded-[20px] border border-white/20 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="backdrop-blur-[20px] bg-white/[0.08] border-b border-white/20">
              <tr>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Project name
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Project lead
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Contributors
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Available issues
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  My contributions
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  My rewards
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Languages
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Repositories
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Billing profile
                </th>
                <th
                  className={`px-6 py-4 text-left text-[12px] font-semibold uppercase tracking-wider whitespace-nowrap transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Actions
                </th>
              </tr>
            </thead>
            <tbody>
              {projects.map((project, idx) => (
                <tr
                  key={project.id}
                  className={`border-b border-white/10 hover:bg-white/[0.05] transition-colors ${
                    idx % 2 === 0 ? "bg-white/[0.02]" : ""
                  }`}
                >
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center space-x-3">
                      <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-lg">
                        {project.logo}
                      </div>
                      <span
                        className={`text-[14px] font-semibold transition-colors ${
                          theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                        }`}
                      >
                        {project.name}
                      </span>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center space-x-2">
                      <Github
                        className={`w-4 h-4 transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      />
                      <span
                        className={`text-[13px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        {project.lead}
                      </span>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`inline-flex items-center px-3 py-1.5 rounded-full text-[13px] font-semibold transition-colors ${
                        theme === "dark"
                          ? "bg-[#c9983a]/20 text-[#f5c563]"
                          : "bg-[#c9983a]/25 border border-[#c9983a]/30 text-[#2d2820]"
                      }`}
                    >
                      {project.contributors}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center space-x-1">
                      <span
                        className={`text-[13px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        {project.availableIssues}
                      </span>
                      <ChevronDown
                        className={`w-4 h-4 transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      />
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`inline-flex items-center px-3 py-1.5 rounded-full text-[13px] font-semibold transition-colors ${
                        theme === "dark"
                          ? "bg-[#c9983a]/20 text-[#f5c563]"
                          : "bg-[#c9983a]/25 border border-[#c9983a]/30 text-[#2d2820]"
                      }`}
                    >
                      {project.myContributions}
                    </span>
                  </td>
                  <td
                    className={`px-6 py-4 text-[14px] font-semibold whitespace-nowrap transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    {project.myRewards}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center gap-2">
                      {project.languages.map((lang, idx) => {
                        const iconInfo = getLanguageIcon(lang);
                        return (
                          <span
                            key={idx}
                            className={`flex items-center gap-1.5 px-2 py-1 rounded-[6px] text-[11px] font-medium transition-colors ${
                              theme === "dark"
                                ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                                : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                            }`}
                          >
                            <span
                              className={`w-4 h-4 ${iconInfo.color} rounded-sm flex items-center justify-center text-[8px] font-bold text-white`}
                            >
                              {iconInfo.icon}
                            </span>
                            <span>{lang}</span>
                          </span>
                        );
                      })}
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center space-x-2">
                      <Github
                        className={`w-4 h-4 transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      />
                      <span
                        className={`text-[12px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        {project.repository}
                      </span>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center space-x-1">
                      <span
                        className={`text-[12px] transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      >
                        {project.billingProfile}
                      </span>
                      <ChevronDown
                        className={`w-4 h-4 transition-colors ${
                          theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                        }`}
                      />
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <button
                      className={`px-4 py-2 rounded-[8px] backdrop-blur-[20px] border text-[12px] font-medium transition-all ${
                        theme === "dark"
                          ? "bg-white/[0.08] border-white/15 text-[#f5f5f5] hover:bg-white/[0.12] hover:border-[#c9983a]/40"
                          : "bg-white/[0.15] border-white/25 text-[#2d2820] hover:bg-white/[0.2] hover:border-[#c9983a]/40"
                      }`}
                    >
                      See project
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Mobile Card View - Hidden on Desktop */}
      <div className="md:hidden space-y-3">
        {projects.map((project) => (
          <div
            key={project.id}
            className={`backdrop-blur-[30px] bg-white/[0.12] rounded-[16px] border border-white/20 p-4 transition-colors hover:bg-white/[0.15] ${
              theme === "dark"
                ? "hover:border-white/30"
                : "hover:border-white/25"
            }`}
          >
            {/* Header: Project name with logo */}
            <div className="flex items-start justify-between gap-3 mb-3">
              <div className="flex items-center gap-3 flex-1 min-w-0">
                <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-lg flex-shrink-0">
                  {project.logo}
                </div>
                <div className="min-w-0 flex-1">
                  <h4
                    className={`text-[14px] font-semibold truncate ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    {project.name}
                  </h4>
                  <p
                    className={`text-[12px] truncate ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    {project.billingProfile}
                  </p>
                </div>
              </div>
            </div>

            {/* Stats Grid */}
            <div className="grid grid-cols-2 gap-3 mb-3">
              {/* Contributors */}
              <div className="backdrop-blur-[20px] bg-white/[0.05] rounded-[10px] p-2.5">
                <span
                  className={`text-[11px] block ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Contributors
                </span>
                <span
                  className={`text-[13px] font-semibold mt-0.5 ${
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }`}
                >
                  {project.contributors}
                </span>
              </div>

              {/* Available Issues */}
              <div className="backdrop-blur-[20px] bg-white/[0.05] rounded-[10px] p-2.5">
                <span
                  className={`text-[11px] block ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  Available
                </span>
                <span
                  className={`text-[13px] font-semibold mt-0.5 ${
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }`}
                >
                  {project.availableIssues}
                </span>
              </div>

              {/* My Contributions */}
              <div className="backdrop-blur-[20px] bg-white/[0.05] rounded-[10px] p-2.5">
                <span
                  className={`text-[11px] block ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  My contributions
                </span>
                <span
                  className={`text-[13px] font-semibold mt-0.5 ${
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }`}
                >
                  {project.myContributions}
                </span>
              </div>

              {/* My Rewards */}
              <div className="backdrop-blur-[20px] bg-white/[0.05] rounded-[10px] p-2.5">
                <span
                  className={`text-[11px] block ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                  }`}
                >
                  My rewards
                </span>
                <span
                  className={`text-[13px] font-semibold mt-0.5 ${
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }`}
                >
                  {project.myRewards}
                </span>
              </div>
            </div>

            {/* Languages */}
            <div className="flex items-center gap-2 mb-3 flex-wrap">
              {project.languages.map((lang, idx) => {
                const iconInfo = getLanguageIcon(lang);
                return (
                  <span
                    key={idx}
                    className={`flex items-center gap-1.5 px-2 py-1 rounded-[6px] text-[11px] font-medium transition-colors ${
                      theme === "dark"
                        ? "bg-[#c9983a]/20 border border-[#c9983a]/30 text-[#f5c563]"
                        : "bg-[#c9983a]/15 border border-[#c9983a]/25 text-[#8b6f3a]"
                    }`}
                  >
                    <span
                      className={`w-4 h-4 ${iconInfo.color} rounded-sm flex items-center justify-center text-[8px] font-bold text-white`}
                    >
                      {iconInfo.icon}
                    </span>
                    <span>{lang}</span>
                  </span>
                );
              })}
            </div>

            {/* See Project Button */}
            <button
              className={`w-full px-4 py-2.5 rounded-[8px] backdrop-blur-[20px] border text-[13px] font-medium transition-all ${
                theme === "dark"
                  ? "bg-[#c9983a]/80 border-[#c9983a] text-[#2d2820] hover:bg-[#c9983a] hover:border-[#c9983a]"
                  : "bg-[#c9983a]/90 border-[#c9983a] text-white hover:bg-[#c9983a] hover:border-[#c9983a]"
              }`}
            >
              See project
            </button>
          </div>
        ))}
      </div>
    </>
  );
}
