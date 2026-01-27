import { useState } from "react";
import {
  Filter,
  Search,
  LayoutGrid,
  Github,
  Check,
  Hourglass,
} from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";

export function RewardsTab() {
  const { theme } = useTheme();
  const [isColumnsModalOpen, setIsColumnsModalOpen] = useState(false);
  const [selectedColumns, setSelectedColumns] = useState<string[]>([
    "Date",
    "ID",
    "Project",
    "From",
    "Contributions",
    "Amount",
    "Status",
  ]);
  const [columnSearchQuery, setColumnSearchQuery] = useState("");

  const rewards = [
    {
      id: 1,
      date: "20/12/2025",
      project: "React Ecosystem",
      logo: "⚛️",
      contribution: "project-lead-1",
      amount: "--- undefined",
      status: "Pending request",
    },
    {
      id: 2,
      date: "13/12/2025",
      project: "Next.js Framework",
      logo: "▲",
      contribution: "project-lead-2",
      amount: "--- undefined",
      status: "Complete",
    },
    {
      id: 3,
      date: "27/11/2025",
      project: "Vue.js",
      logo: "V",
      contribution: "project-lead-3",
      amount: "--",
      status: "Processing",
    },
    {
      id: 4,
      date: "28/10/2025",
      project: "Angular",
      logo: "A",
      contribution: "project-lead-4",
      amount: "--- undefined",
      status: "Complete",
    },
    {
      id: 5,
      date: "25/12/2025",
      project: "Svelte",
      logo: "S",
      contribution: "project-lead-5",
      amount: "--- undefined",
      status: "Pending request",
    },
    {
      id: 6,
      date: "13/12/2025",
      project: "Express.js",
      logo: "E",
      contribution: "project-lead-6",
      amount: "--- undefined",
      status: "Pending request",
    },
    {
      id: 7,
      date: "27/11/2025",
      project: "Rust",
      logo: "R",
      contribution: "project-lead-7",
      amount: "--",
      status: "Complete",
    },
  ];

  return (
    <>
      <div className="space-y-4">
        {/* Filter and Search Bar */}
        <div className="flex items-center gap-2 sm:gap-3">
          {/* Filter Button */}
          <button className="h-12 flex-shrink-0 w-10 sm:w-12 flex items-center justify-center rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 text-[#7a6b5a] hover:bg-white/[0.2] hover:border-[#c9983a]/40 transition-all">
            <Filter className="w-5 h-5" />
          </button>

          {/* Search Bar */}
          <div className="relative flex-1">
            <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-[#7a6b5a] z-10" />
            <input
              type="text"
              placeholder="Search"
              className="w-full h-12 pl-12 pr-4 py-2.5 sm:py-3 rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 text-[#2d2820] placeholder-[#7a6b5a] focus:outline-none focus:bg-white/[0.2] focus:border-[#c9983a]/40 transition-all text-[13px]"
            />
          </div>
        </div>

        {/* Grid Layout Button */}
        <button
          onClick={() => setIsColumnsModalOpen(!isColumnsModalOpen)}
          className="w-full h-12 flex-shrink-0 sm:w-12 flex items-center justify-center rounded-[12px] backdrop-blur-[30px] bg-white/[0.15] border border-white/25 text-[#7a6b5a] hover:bg-white/[0.2] hover:border-[#c9983a]/40 transition-all"
        >
          <LayoutGrid className="w-5 h-5" />
        </button>

        {/* Desktop Table View - Hidden on Mobile */}
        <div className="hidden md:block backdrop-blur-[30px] bg-white/[0.12] rounded-[20px] border border-white/20 overflow-hidden">
          <table className="w-full">
            <thead className="backdrop-blur-[20px] bg-white/[0.08] border-b border-white/20">
              <tr>
                {selectedColumns.includes("Date") && (
                  <th
                    className={`px-4 lg:px-6 py-4 text-left text-[11px] lg:text-[12px] font-semibold uppercase tracking-wider transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Date
                  </th>
                )}
                {selectedColumns.includes("ID") && (
                  <th
                    className={`px-4 lg:px-6 py-4 text-left text-[11px] lg:text-[12px] font-semibold uppercase tracking-wider transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    ID
                  </th>
                )}
                {selectedColumns.includes("Project") && (
                  <th
                    className={`px-4 lg:px-6 py-4 text-left text-[11px] lg:text-[12px] font-semibold uppercase tracking-wider transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Project
                  </th>
                )}
                {selectedColumns.includes("Amount") && (
                  <th
                    className={`px-4 lg:px-6 py-4 text-left text-[11px] lg:text-[12px] font-semibold uppercase tracking-wider transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Amount
                  </th>
                )}
                {selectedColumns.includes("Status") && (
                  <th
                    className={`px-4 lg:px-6 py-4 text-left text-[11px] lg:text-[12px] font-semibold uppercase tracking-wider transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Status
                  </th>
                )}
              </tr>
            </thead>
            <tbody>
              {rewards.map((reward, idx) => (
                <tr
                  key={reward.id}
                  className={`border-b border-white/10 hover:bg-white/[0.05] transition-colors ${
                    idx % 2 === 0 ? "bg-white/[0.02]" : ""
                  }`}
                >
                  {selectedColumns.includes("Date") && (
                    <td
                      className={`px-4 lg:px-6 py-4 text-[12px] lg:text-[13px] transition-colors ${
                        theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                      }`}
                    >
                      {reward.date}
                    </td>
                  )}
                  {selectedColumns.includes("ID") && (
                    <td
                      className={`px-4 lg:px-6 py-4 text-[12px] lg:text-[13px] transition-colors ${
                        theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                      }`}
                    >
                      #{reward.id}
                    </td>
                  )}
                  {selectedColumns.includes("Project") && (
                    <td className="px-4 lg:px-6 py-4">
                      <div className="flex items-center space-x-3">
                        <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-sm">
                          {reward.logo}
                        </div>
                        <span
                          className={`text-[12px] lg:text-[13px] transition-colors ${
                            theme === "dark"
                              ? "text-[#f5f5f5]"
                              : "text-[#2d2820]"
                          }`}
                        >
                          {reward.project}
                        </span>
                      </div>
                    </td>
                  )}
                  {selectedColumns.includes("Amount") && (
                    <td
                      className={`px-4 lg:px-6 py-4 text-[12px] lg:text-[13px] transition-colors ${
                        theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                      }`}
                    >
                      {reward.amount}
                    </td>
                  )}
                  {selectedColumns.includes("Status") && (
                    <td className="px-4 lg:px-6 py-4">
                      <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full backdrop-blur-[20px] bg-white/[0.15] border border-white/20">
                        {reward.status === "Complete" ? (
                          <Check className="w-4 h-4 text-green-600" />
                        ) : reward.status === "Processing" ? (
                          <Hourglass className="w-4 h-4 text-yellow-600" />
                        ) : (
                          <Hourglass className="w-4 h-4 text-orange-600" />
                        )}
                        <span
                          className={`text-[12px] lg:text-[13px] transition-colors ${
                            theme === "dark"
                              ? "text-[#f5f5f5]"
                              : "text-[#2d2820]"
                          }`}
                        >
                          {reward.status}
                        </span>
                      </div>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Mobile Card View - Hidden on Desktop */}
        <div className="md:hidden space-y-3">
          {rewards.map((reward) => (
            <div
              key={reward.id}
              className={`backdrop-blur-[30px] bg-white/[0.12] rounded-[16px] border border-white/20 p-4 transition-colors hover:bg-white/[0.15] ${
                theme === "dark"
                  ? "hover:border-white/30"
                  : "hover:border-white/25"
              }`}
            >
              {/* Header: Date, Project, Status */}
              <div className="flex items-center justify-between gap-2 mb-3">
                <div>
                  <span
                    className={`text-[12px] ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    {reward.date}
                  </span>
                  <div className="flex items-center space-x-2 mt-1">
                    <div className="w-6 h-6 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center text-xs font-bold text-white">
                      {reward.logo}
                    </div>
                    <span
                      className={`text-[13px] font-medium ${
                        theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                      }`}
                    >
                      {reward.project}
                    </span>
                  </div>
                </div>
                <div className="flex-shrink-0 inline-flex items-center gap-2 px-2.5 py-1.5 rounded-full backdrop-blur-[20px] bg-white/[0.15] border border-white/20">
                  {reward.status === "Complete" ? (
                    <Check className="w-4 h-4 text-green-600" />
                  ) : reward.status === "Processing" ? (
                    <Hourglass className="w-4 h-4 text-yellow-600" />
                  ) : (
                    <Hourglass className="w-4 h-4 text-orange-600" />
                  )}
                  <span
                    className={`text-[11px] transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    {reward.status}
                  </span>
                </div>
              </div>

              {/* Stats Grid */}
              <div className="grid grid-cols-2 gap-3">
                {/* Amount */}
                <div className="backdrop-blur-[20px] bg-white/[0.05] rounded-[10px] p-2.5">
                  <span
                    className={`text-[11px] block ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Amount
                  </span>
                  <span
                    className={`text-[13px] font-semibold mt-0.5 ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    {reward.amount}
                  </span>
                </div>

                {/* ID */}
                <div className="backdrop-blur-[20px] bg-white/[0.05] rounded-[10px] p-2.5">
                  <span
                    className={`text-[11px] block ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    ID
                  </span>
                  <span
                    className={`text-[13px] font-semibold mt-0.5 ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    #{reward.id}
                  </span>
                </div>
              </div>

              {/* Details: From and Contributions (optional, can be hidden) */}
              <div className="mt-3 pt-3 border-t border-white/10 space-y-2">
                {/* From */}
                <div className="flex items-center space-x-2">
                  <div className="w-5 h-5 rounded-full bg-gradient-to-br from-blue-500 to-purple-500 flex-shrink-0" />
                  <span
                    className={`text-[12px] ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    contributor-{reward.id}
                  </span>
                </div>

                {/* Contributions */}
                <div className="flex items-center space-x-2">
                  <Github
                    className={`w-4 h-4 flex-shrink-0 ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  />
                  <span
                    className={`text-[12px] ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    {reward.contribution}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Rewards Columns Modal */}
      {isColumnsModalOpen && (
        <>
          {/* Modal Dropdown - Glassmorphism Style */}
          <div className="fixed top-[140px] right-[40px] w-[320px] backdrop-blur-[40px] bg-white/[0.12] rounded-[16px] border border-white/30 z-50 shadow-[0_20px_60px_rgba(0,0,0,0.25)] overflow-hidden">
            {/* Header */}
            <div className="px-5 py-4 border-b border-white/20">
              <h3 className="text-[16px] font-bold text-[#2d2820]">
                Rewards columns
              </h3>
            </div>

            {/* Search Bar */}
            <div className="px-5 pt-4 pb-3">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[#2d2820]" />
                <input
                  type="text"
                  placeholder="Search"
                  value={columnSearchQuery}
                  onChange={(e) => setColumnSearchQuery(e.target.value)}
                  className="w-full pl-9 pr-3 py-2.5 rounded-[10px] backdrop-blur-[20px] bg-white/[0.2] border border-white/25 text-[#2d2820] text-[13px] placeholder-[#7a6b5a] focus:outline-none focus:bg-white/[0.25] focus:border-[#c9983a]/40 transition-all"
                />
              </div>
            </div>

            {/* Column Options */}
            <div className="px-5 pb-4 max-h-[360px] overflow-y-auto scrollbar-hide">
              <div className="space-y-2">
                {[
                  "Date",
                  "ID",
                  "Project",
                  "From",
                  "Contributions",
                  "Amount",
                  "Status",
                ]
                  .filter((col) =>
                    col.toLowerCase().includes(columnSearchQuery.toLowerCase()),
                  )
                  .map((column) => {
                    const isSelected = selectedColumns.includes(column);
                    return (
                      <button
                        key={column}
                        onClick={() => {
                          if (isSelected) {
                            setSelectedColumns(
                              selectedColumns.filter((c) => c !== column),
                            );
                          } else {
                            setSelectedColumns([...selectedColumns, column]);
                          }
                        }}
                        className={`w-full px-3.5 py-3 rounded-[10px] text-left text-[13px] font-medium transition-all flex items-center gap-3 backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2] ${
                          isSelected ? "hover:border-[#c9983a]/40" : ""
                        }`}
                      >
                        <div
                          className={`w-5 h-5 rounded-[6px] flex items-center justify-center border-2 transition-all ${
                            isSelected
                              ? "bg-[#c9983a] border-[#c9983a]"
                              : "bg-white/30 border-[#7a6b5a]/40"
                          }`}
                        >
                          {isSelected && (
                            <Check className="w-3.5 h-3.5 text-white stroke-[3]" />
                          )}
                        </div>
                        <span>{column}</span>
                      </button>
                    );
                  })}
              </div>
            </div>

            {/* Footer Actions */}
            <div className="px-5 py-4 border-t border-white/20 flex items-center justify-between">
              <button
                onClick={() => setIsColumnsModalOpen(false)}
                className="text-[13px] text-[#7a6b5a] hover:text-[#2d2820] transition-all font-medium"
              >
                Pending request
              </button>
              <button
                onClick={() => setIsColumnsModalOpen(false)}
                className="flex items-center gap-1.5 text-[13px] text-[#2d2820] hover:text-[#c9983a] transition-all font-semibold"
              >
                <Check className="w-4 h-4" />
                <span>Complete</span>
              </button>
            </div>
          </div>
        </>
      )}
    </>
  );
}
