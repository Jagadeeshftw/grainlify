import { useMemo, useState, useEffect } from "react";
import type { ComponentType } from "react";
import { ChevronDown, Info, Sun, Moon } from "lucide-react";
import {
  Bar,
  Line as RechartsLine,
  XAxis as XAxisBase,
  YAxis as YAxisBase,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ComposedChart,
} from "recharts";

const XAxis = XAxisBase as unknown as ComponentType<any>;
const YAxis = YAxisBase as unknown as ComponentType<any>;
import {
  ComposableMap,
  Geographies,
  Geography,
  Marker,
  ZoomableGroup,
  Line as MapLine,
} from "react-simple-maps";
import { useTheme } from "../../../shared/contexts/ThemeContext";

type ActivityPoint = {
  label: string;
  value: number;
  trend: number;
  new: number;
  reactivated: number;
  active: number;
  churned: number;
  rewarded: number;
};

const buildRatios = (base: any) => ({
  trend: base.trend ? base.trend / base.value : 0,
  new: base.new ? base.new / base.value : 0,
  reactivated: base.reactivated ? base.reactivated / base.value : 0,
  active: base.active ? base.active / base.value : 0,
  churned: base.churned ? base.churned / base.value : 0,
  rewarded: base.rewarded ? base.rewarded / base.value : 0,
});

const scalePoint = (
  label: string,
  value: number,
  ratios: ReturnType<typeof buildRatios>,
): ActivityPoint => ({
  label,
  value,
  trend: Math.round(value * ratios.trend),
  new: Math.round(value * ratios.new),
  reactivated: Math.round(value * ratios.reactivated),
  active: Math.round(value * ratios.active),
  churned: Math.round(value * ratios.churned),
  rewarded: Math.round(value * ratios.rewarded),
});

const aggregateRange = (label: string, slice: any[]): ActivityPoint => {
  const totals = slice.reduce(
    (acc, item) => ({
      value: acc.value + item.value,
      trend: acc.trend + item.trend,
      new: acc.new + item.new,
      reactivated: acc.reactivated + item.reactivated,
      active: acc.active + item.active,
      churned: acc.churned + item.churned,
      rewarded: acc.rewarded + item.rewarded,
    }),
    {
      value: 0,
      trend: 0,
      new: 0,
      reactivated: 0,
      active: 0,
      churned: 0,
      rewarded: 0,
    },
  );
  return { label, ...totals };
};

const buildDailyData = (monthly: any[]): ActivityPoint[] => {
  const base = monthly[monthly.length - 1];
  const ratios = buildRatios(base);
  const days = 28;
  const baseValue = Math.max(1, Math.round(base.value / days));
  return Array.from({ length: days }).map((_, idx) => {
    const bump = 0.85 + (idx % 7) * 0.03;
    const value = Math.max(1, Math.round(baseValue * bump));
    return scalePoint(`D${idx + 1}`, value, ratios);
  });
};

const buildWeeklyData = (monthly: any[]): ActivityPoint[] => {
  const recent = monthly.slice(-3);
  const avg = Math.round(
    recent.reduce((sum, item) => sum + item.value, 0) / recent.length,
  );
  const ratios = buildRatios(recent[recent.length - 1]);
  const weeks = 12;
  const baseValue = Math.max(1, Math.round(avg / 4));
  return Array.from({ length: weeks }).map((_, idx) => {
    const bump = 0.9 + (idx % 4) * 0.05;
    const value = Math.max(1, Math.round(baseValue * bump));
    return scalePoint(`W${idx + 1}`, value, ratios);
  });
};

const buildQuarterlyData = (monthly: any[]): ActivityPoint[] => [
  aggregateRange("Q1", monthly.slice(0, 3)),
  aggregateRange("Q2", monthly.slice(3, 6)),
  aggregateRange("Q3", monthly.slice(6, 9)),
  aggregateRange("Q4", monthly.slice(9, 12)),
];

const buildYearlyData = (monthly: any[]): ActivityPoint[] => [
  aggregateRange("2024", monthly.slice(0, 6)),
  aggregateRange("2025", monthly.slice(6, 12)),
];

export function DataPage() {
  const { theme, toggleTheme } = useTheme();
  const [mapZoom, setMapZoom] = useState(1);
  const [mapCenter, setMapCenter] = useState<[number, number]>([0, 0]);

  const geoUrl =
    "https://cdn.jsdelivr.net/npm/world-atlas@2/countries-110m.json";

  const countryCoordinates: Record<string, [number, number]> = {
    "United Kingdom": [-3.435973, 55.378051],
    Germany: [10.451526, 51.165691],
    Canada: [-106.346771, 56.130366],
    India: [78.96288, 20.593684],
    Brazil: [-51.92528, -14.235004],
    Netherlands: [5.291266, 52.132633],
    Australia: [133.775136, -25.274398],
    Spain: [-3.74922, 40.463667],
    Italy: [12.56738, 41.87194],
    Poland: [19.145136, 51.919438],
    Sweden: [18.643501, 60.128161],
    Japan: [138.252924, 36.204824],
    China: [104.195397, 35.86166],
  };
  const [activeTab, setActiveTab] = useState("overview");
  const [projectInterval, setProjectInterval] = useState("Monthly interval");
  const contributorIntervalLabels = {
    daily: "Daily interval",
    weekly: "Weekly interval",
    monthly: "Monthly interval",
    quarterly: "Quarterly interval",
    yearly: "Yearly interval",
  };
  const [contributorInterval, setContributorInterval] =
    useState<keyof typeof contributorIntervalLabels>("monthly");
  const [showProjectIntervalDropdown, setShowProjectIntervalDropdown] =
    useState(false);
  const [showContributorIntervalDropdown, setShowContributorIntervalDropdown] =
    useState(false);
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth < 768);
    };
    checkMobile();
    window.addEventListener("resize", checkMobile);
    return () => window.removeEventListener("resize", checkMobile);
  }, []);

  const [projectFilters, setProjectFilters] = useState({
    new: false,
    reactivated: false,
    active: false,
    churned: false,
    prMerged: false,
  });
  const [contributorFilters, setContributorFilters] = useState({
    new: false,
    reactivated: false,
    active: false,
    churned: false,
    prMerged: false,
  });

  // Sample data for project activity (monthly data)
  const projectActivityData = [
    {
      month: "January",
      value: 45,
      trend: 40,
      new: 12,
      reactivated: 5,
      active: 28,
      churned: -8,
      rewarded: 15420,
    },
    {
      month: "February",
      value: 38,
      trend: 42,
      new: 8,
      reactivated: 4,
      active: 26,
      churned: -6,
      rewarded: 12300,
    },
    {
      month: "March",
      value: 52,
      trend: 45,
      new: 15,
      reactivated: 7,
      active: 30,
      churned: -5,
      rewarded: 18650,
    },
    {
      month: "April",
      value: 48,
      trend: 50,
      new: 11,
      reactivated: 6,
      active: 31,
      churned: -7,
      rewarded: 16800,
    },
    {
      month: "May",
      value: 58,
      trend: 52,
      new: 18,
      reactivated: 8,
      active: 32,
      churned: -4,
      rewarded: 22100,
    },
    {
      month: "June",
      value: 55,
      trend: 55,
      new: 14,
      reactivated: 6,
      active: 35,
      churned: -9,
      rewarded: 20500,
    },
    {
      month: "July",
      value: 42,
      trend: 54,
      new: 9,
      reactivated: 5,
      active: 28,
      churned: -10,
      rewarded: 14200,
    },
    {
      month: "August",
      value: 48,
      trend: 50,
      new: 12,
      reactivated: 7,
      active: 29,
      churned: -6,
      rewarded: 17300,
    },
    {
      month: "September",
      value: 62,
      trend: 52,
      new: 20,
      reactivated: 9,
      active: 33,
      churned: -5,
      rewarded: 24800,
    },
    {
      month: "October",
      value: 58,
      trend: 58,
      new: 16,
      reactivated: 8,
      active: 34,
      churned: -7,
      rewarded: 21900,
    },
    {
      month: "November",
      value: 45,
      trend: 56,
      new: 10,
      reactivated: 6,
      active: 29,
      churned: -8,
      rewarded: 15600,
    },
    {
      month: "December",
      value: 52,
      trend: 52,
      new: 13,
      reactivated: 7,
      active: 32,
      churned: -10,
      rewarded: 18900,
    },
  ];

  // Sample data for contributor activity
  const contributorActivityData = [
    {
      month: "January",
      value: 42,
      trend: 38,
      new: 10,
      reactivated: 4,
      active: 28,
      churned: -6,
      rewarded: 14200,
    },
    {
      month: "February",
      value: 35,
      trend: 40,
      new: 7,
      reactivated: 3,
      active: 25,
      churned: -5,
      rewarded: 11800,
    },
    {
      month: "March",
      value: 48,
      trend: 42,
      new: 13,
      reactivated: 6,
      active: 29,
      churned: -4,
      rewarded: 16900,
    },
    {
      month: "April",
      value: 45,
      trend: 46,
      new: 11,
      reactivated: 5,
      active: 29,
      churned: -6,
      rewarded: 15300,
    },
    {
      month: "May",
      value: 38,
      trend: 44,
      new: 8,
      reactivated: 4,
      active: 26,
      churned: -7,
      rewarded: 12700,
    },
    {
      month: "June",
      value: 52,
      trend: 45,
      new: 15,
      reactivated: 7,
      active: 30,
      churned: -5,
      rewarded: 19100,
    },
    {
      month: "July",
      value: 48,
      trend: 48,
      new: 12,
      reactivated: 6,
      active: 30,
      churned: -8,
      rewarded: 17400,
    },
    {
      month: "August",
      value: 55,
      trend: 50,
      new: 17,
      reactivated: 8,
      active: 30,
      churned: -4,
      rewarded: 21300,
    },
    {
      month: "September",
      value: 50,
      trend: 52,
      new: 14,
      reactivated: 7,
      active: 29,
      churned: -6,
      rewarded: 18600,
    },
    {
      month: "October",
      value: 58,
      trend: 54,
      new: 19,
      reactivated: 9,
      active: 30,
      churned: -5,
      rewarded: 23800,
    },
    {
      month: "November",
      value: 52,
      trend: 56,
      new: 15,
      reactivated: 7,
      active: 30,
      churned: -7,
      rewarded: 19500,
    },
    {
      month: "December",
      value: 48,
      trend: 52,
      new: 12,
      reactivated: 6,
      active: 30,
      churned: -8,
      rewarded: 17200,
    },
  ];

  // Contributors by country/region
  const contributorsByRegion = [
    { name: "United Kingdom", value: 625, percentage: 45 },
    { name: "Germany", value: 720, percentage: 52 },
    { name: "Canada", value: 580, percentage: 42 },
    { name: "India", value: 560, percentage: 40 },
    { name: "Brazil", value: 490, percentage: 35 },
    { name: "Netherlands", value: 300, percentage: 22 },
    { name: "Australia", value: 430, percentage: 31 },
    { name: "Spain", value: 280, percentage: 20 },
    { name: "Italy", value: 220, percentage: 16 },
    { name: "Poland", value: 280, percentage: 20 },
    { name: "Sweden", value: 210, percentage: 15 },
    { name: "Japan", value: 240, percentage: 17 },
    { name: "China", value: 220, percentage: 16 },
  ];

  const toggleProjectFilter = (filter: keyof typeof projectFilters) => {
    setProjectFilters((prev) => ({ ...prev, [filter]: !prev[filter] }));
  };

  const toggleContributorFilter = (filter: keyof typeof contributorFilters) => {
    setContributorFilters((prev) => ({ ...prev, [filter]: !prev[filter] }));
  };

  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0].payload;
      return (
        <div className="backdrop-blur-[30px] bg-[#1a1410]/95 border-2 border-white/20 rounded-[10px] md:rounded-[12px] px-3 md:px-5 py-3 md:py-4 min-w-[160px] md:min-w-[200px] max-w-[90vw]">
          <p className="text-[11px] md:text-[13px] font-bold text-white mb-2 md:mb-3">
            {data.label || data.month} 2025
          </p>
          <div className="space-y-1.5 md:space-y-2">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 md:gap-2">
                <div className="w-2.5 h-2.5 md:w-3 md:h-3 rounded-full bg-[#c9983a] flex-shrink-0" />
                <span className="text-[10px] md:text-[12px] text-white/80">
                  New
                </span>
              </div>
              <span className="text-[11px] md:text-[13px] font-bold text-[#c9983a] flex-shrink-0">
                {data.new}
              </span>
            </div>
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 md:gap-2">
                <div className="w-2.5 h-2.5 md:w-3 md:h-3 rounded-full bg-[#d4af37] flex-shrink-0" />
                <span className="text-[10px] md:text-[12px] text-white/80">
                  Reactivated
                </span>
              </div>
              <span className="text-[11px] md:text-[13px] font-bold text-[#d4af37] flex-shrink-0">
                {data.reactivated}
              </span>
            </div>
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 md:gap-2">
                <div className="w-2.5 h-2.5 md:w-3 md:h-3 rounded-full bg-[#c9983a]/70 flex-shrink-0" />
                <span className="text-[10px] md:text-[12px] text-white/80">
                  Active
                </span>
              </div>
              <span className="text-[11px] md:text-[13px] font-bold text-[#c9983a]/90 flex-shrink-0">
                {data.active}
              </span>
            </div>
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 md:gap-2">
                <div className="w-2.5 h-2.5 md:w-3 md:h-3 rounded-full bg-[#ff6b6b] flex-shrink-0" />
                <span className="text-[10px] md:text-[12px] text-white/80">
                  Churned
                </span>
              </div>
              <span className="text-[11px] md:text-[13px] font-bold text-[#ff6b6b] flex-shrink-0">
                {data.churned}
              </span>
            </div>
            <div className="h-px bg-white/10 my-1.5 md:my-2" />
            <div className="flex items-center justify-between gap-2 pt-0.5 md:pt-1">
              <div className="flex items-center gap-1.5 md:gap-2">
                <div className="w-2.5 h-2.5 md:w-3 md:h-3 rounded-full bg-gradient-to-r from-[#c9983a] to-[#d4af37] flex-shrink-0" />
                <span className="text-[10px] md:text-[12px] text-white/80">
                  Rewarded
                </span>
              </div>
              <span className="text-[11px] md:text-[13px] font-bold text-white flex-shrink-0">
                {data.rewarded.toLocaleString()} USD
              </span>
            </div>
          </div>
        </div>
      );
    }
    return null;
  };

  const contributorChartData = useMemo<ActivityPoint[]>(() => {
    if (contributorInterval === "daily") {
      return buildDailyData(contributorActivityData);
    }
    if (contributorInterval === "weekly") {
      return buildWeeklyData(contributorActivityData);
    }
    if (contributorInterval === "quarterly") {
      return buildQuarterlyData(contributorActivityData);
    }
    if (contributorInterval === "yearly") {
      return buildYearlyData(contributorActivityData);
    }
    return contributorActivityData.map((item) => ({
      label: item.month,
      value: item.value,
      trend: item.trend,
      new: item.new,
      reactivated: item.reactivated,
      active: item.active,
      churned: item.churned,
      rewarded: item.rewarded,
    }));
  }, [contributorInterval]);

  return (
    <div
      className={`min-h-screen space-y-6 ${
        theme === "dark" ? "bg-[#1a1612]" : "bg-gray-50"
      }`}
    >
      {/* Navbar */}
      <div
        className={`fixed top-0 left-0 right-0 z-50 backdrop-blur-[40px] border-b p-4 transition-colors ${
          theme === "dark"
            ? "bg-[#2d2820]/[0.95] border-white/20"
            : "bg-white/[0.55] border-white/20"
        }`}
      >
        <div className="flex justify-between items-center">
          <h1
            className={`text-xl font-bold ${
              theme === "dark" ? "text-white" : "text-[#2d2820]"
            }`}
          >
            DataPage
          </h1>
          <button
            onClick={toggleTheme}
            className={`p-2 rounded-full transition-colors ${
              theme === "dark"
                ? "bg-white/[0.1] hover:bg-white/[0.2] text-white"
                : "bg-black/[0.1] hover:bg-black/[0.2] text-black"
            }`}
            title={`Switch to ${theme === "light" ? "dark" : "light"} mode`}
          >
            {theme === "light" ? <Moon size={20} /> : <Sun size={20} />}
          </button>
        </div>
      </div>

      {/* Content with top padding to account for fixed navbar */}
      <div className="pt-20 space-y-4 md:space-y-6 px-4 md:px-0">
        {/* Header Tabs */}
        <div
          className={`backdrop-blur-[40px] rounded-[16px] md:rounded-[24px] border p-1.5 md:p-2 transition-colors ${
            theme === "dark"
              ? "bg-white/[0.08] border-white/20"
              : "bg-white/[0.12] border-white/20"
          }`}
        >
          <div className="flex items-center gap-1.5 md:gap-2 overflow-x-auto scrollbar-hide">
            <button
              onClick={() => setActiveTab("overview")}
              className={`px-4 md:px-6 py-2 md:py-3 rounded-[12px] md:rounded-[16px] font-bold text-[12px] md:text-[14px] transition-all duration-300 whitespace-nowrap flex-shrink-0 ${
                activeTab === "overview"
                  ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${
                      theme === "dark" ? "text-[#f5c563]" : "text-[#2d2820]"
                    }`
                  : `${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    } hover:bg-white/[0.08]`
              }`}
            >
              Overview
            </button>
            <button
              onClick={() => setActiveTab("projects")}
              className={`px-4 md:px-6 py-2 md:py-3 rounded-[12px] md:rounded-[16px] font-bold text-[12px] md:text-[14px] transition-all duration-300 whitespace-nowrap flex-shrink-0 ${
                activeTab === "projects"
                  ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${
                      theme === "dark" ? "text-[#f5c563]" : "text-[#2d2820]"
                    }`
                  : `${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    } hover:bg-white/[0.08]`
              }`}
            >
              Projects
            </button>
            <button
              onClick={() => setActiveTab("contributions")}
              className={`px-4 md:px-6 py-2 md:py-3 rounded-[12px] md:rounded-[16px] font-bold text-[12px] md:text-[14px] transition-all duration-300 whitespace-nowrap flex-shrink-0 ${
                activeTab === "contributions"
                  ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${
                      theme === "dark" ? "text-[#f5c563]" : "text-[#2d2820]"
                    }`
                  : `${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    } hover:bg-white/[0.08]`
              }`}
            >
              Contributions
            </button>
          </div>
        </div>

        {/* Main Content Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6">
          {/* Left Column - Project Activity */}
          <div
            className={`backdrop-blur-[40px] rounded-[24px] border border-white/20 p-8 transition-colors ${
              theme === "dark"
                ? "bg-white/[0.15] shadow-[0_8px_32px_rgba(0,0,0,0.12)]"
                : "bg-white/[0.12] shadow-[0_8px_32px_rgba(0,0,0,0.08)]"
            }`}
          >
            <div className="flex items-center justify-between mb-6">
              <h2
                className={`text-[18px] font-bold transition-colors ${
                  theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                }`}
              >
                Project activity
              </h2>
              <div className="relative">
                <button
                  onClick={() =>
                    setShowProjectIntervalDropdown(!showProjectIntervalDropdown)
                  }
                  className={`flex items-center gap-2 px-4 py-2 rounded-[10px] backdrop-blur-[20px] border transition-all ${
                    theme === "dark"
                      ? "bg-white/[0.08] border-white/20 hover:bg-white/[0.12] text-[#f5f5f5]"
                      : "bg-white/[0.15] border-white/25 hover:bg-white/[0.2] text-[#2d2820]"
                  }`}
                >
                  <span
                    className={`text-[13px] font-semibold transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    {projectInterval}
                  </span>
                  <ChevronDown
                    className={`w-4 h-4 transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  />
                </button>
                {showProjectIntervalDropdown && (
                  <div className="absolute right-0 sm:right-0 left-0 sm:left-auto mt-2 w-full sm:w-[180px] backdrop-blur-[30px] bg-white/[0.55] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-50">
                    <button
                      onClick={() => {
                        setProjectInterval("Daily interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all touch-manipulation"
                    >
                      Daily interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Weekly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all touch-manipulation"
                    >
                      Weekly interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Monthly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all ${
                        projectInterval === "Monthly interval"
                          ? theme === "dark"
                            ? "bg-[#c9983a]/[0.15] text-[#f5c563] font-bold"
                            : "bg-white/[0.35] text-[#2d2820] font-bold"
                          : theme === "dark"
                          ? "text-[#f5f5f5] hover:bg-white/[0.05]"
                          : "text-[#2d2820] hover:bg-white/[0.3]"
                      }`}
                    >
                      Monthly interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Quarterly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all touch-manipulation"
                    >
                      Quarterly interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Yearly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all touch-manipulation"
                    >
                      Yearly interval
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Chart */}
            <div className="h-[280px] mb-6">
              <ResponsiveContainer width="100%" height="100%">
                <ComposedChart data={projectActivityData}>
                  <defs>
                    <linearGradient
                      id="barGradient"
                      x1="0"
                      y1="0"
                      x2="0"
                      y2="1"
                    >
                      <stop offset="0%" stopColor="#c9983a" stopOpacity={0.8} />
                      <stop
                        offset="100%"
                        stopColor="#d4af37"
                        stopOpacity={0.4}
                      />
                    </linearGradient>
                  </defs>
                  <CartesianGrid
                    strokeDasharray="3 3"
                    stroke="rgba(122, 107, 90, 0.1)"
                  />
                  <XAxis
                    dataKey="month"
                    stroke="#7a6b5a"
                    tick={{ fill: "#7a6b5a", fontSize: 11, fontWeight: 600 }}
                    angle={-45}
                    textAnchor="end"
                    height={80}
                  />
                  <YAxis
                    stroke="#7a6b5a"
                    tick={{ fill: "#7a6b5a", fontSize: 11, fontWeight: 600 }}
                  />
                  <Tooltip content={<CustomTooltip />} />
                  <Bar
                    dataKey="value"
                    fill="url(#barGradient)"
                    radius={[8, 8, 0, 0]}
                    maxBarSize={40}
                  />
                  <RechartsLine
                    type="monotone"
                    dataKey="trend"
                    stroke="#2d2820"
                    strokeWidth={3}
                    dot={false}
                  />
                </ComposedChart>
              </ResponsiveContainer>
            </div>

            {/* Filters */}
            <div className="flex flex-wrap items-center gap-2">
              <button
                onClick={() => toggleProjectFilter("new")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  projectFilters.new
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                New
              </button>
              <button
                onClick={() => toggleProjectFilter("reactivated")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  projectFilters.reactivated
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                Reactivated
              </button>
              <button
                onClick={() => toggleProjectFilter("active")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  projectFilters.active
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                Active
              </button>
              <button
                onClick={() => toggleProjectFilter("churned")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  projectFilters.churned
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                Churned
              </button>
              <button
                onClick={() => toggleProjectFilter("prMerged")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  projectFilters.prMerged
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                PR merged
              </button>
            </div>
          </div>
        {/* Left Column - Activity Sections */}
        <div className="flex flex-col gap-6">
          {/* Project Activity */}
          <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className={`text-[16px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>Project activity</h2>
              <div className="relative">
                <button
                  onClick={() => setShowProjectIntervalDropdown(!showProjectIntervalDropdown)}
                  className="flex items-center gap-2 px-3 py-1.5 rounded-[8px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all"
                >
                  <span className={`text-[12px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>{projectInterval}</span>
                  <ChevronDown className={`w-3.5 h-3.5 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`} />
                </button>
                {showProjectIntervalDropdown && (
                  <div className="absolute right-0 mt-2 w-[160px] backdrop-blur-[30px] bg-white/[0.55] border-2 border-white/30 rounded-[10px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-50">
                    <button
                      onClick={() => {
                        setProjectInterval('Daily interval');
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-2 text-left text-[12px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all"
                    >
                      Daily interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval('Weekly interval');
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-2 text-left text-[12px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all"
                    >
                      Weekly interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval('Monthly interval');
                        setShowProjectIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-2 text-left text-[12px] font-medium transition-all ${projectInterval === 'Monthly interval'
                          ? 'bg-white/[0.35] text-[#2d2820] font-bold'
                          : 'text-[#2d2820] hover:bg-white/[0.3]'
                        }`}
                    >
                      Monthly interval
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Chart */}
            <div className="h-[140px] mb-6">
              <ResponsiveContainer width="100%" height="100%">
                <ComposedChart data={projectActivityData}>
                  <defs>
                    <linearGradient id="barGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="#c9983a" stopOpacity={0.8} />
                      <stop offset="100%" stopColor="#d4af37" stopOpacity={0.4} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(122, 107, 90, 0.1)" />
                  <XAxis
                    dataKey="month"
                    stroke="#7a6b5a"
                    tick={{ fill: '#7a6b5a', fontSize: 10, fontWeight: 600 }}
                    angle={-45}
                    textAnchor="end"
                    height={50}
                  />
                  <Tooltip content={<CustomTooltip />} />
                  <Bar
                    dataKey="value"
                    fill="url(#barGradient)"
                    radius={[4, 4, 0, 0]}
                    maxBarSize={30}
                  />
                  <RechartsLine
                    type="monotone"
                    dataKey="trend"
                    stroke="#2d2820"
                    strokeWidth={2}
                    dot={false}
                  />
                </ComposedChart>
              </ResponsiveContainer>
            </div>

            {/* Filters */}
            <div className="flex flex-wrap items-center gap-1.5">
              <button
                onClick={() => toggleProjectFilter('new')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${projectFilters.new
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                New
              </button>
              <button
                onClick={() => toggleProjectFilter('reactivated')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${projectFilters.reactivated
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                Reactivated
              </button>
              <button
                onClick={() => toggleProjectFilter('active')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${projectFilters.active
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                Active
              </button>
              <button
                onClick={() => toggleProjectFilter('churned')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${projectFilters.churned
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                Churned
              </button>
              <button
                onClick={() => toggleProjectFilter('prMerged')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${projectFilters.prMerged
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                PR merged
              </button>
            </div>
          </div>

          {/* Contributor Activity */}
          <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className={`text-[16px] font-bold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}>Contributor activity</h2>
              <div className="relative">
                <button
                  onClick={() => setShowContributorIntervalDropdown(!showContributorIntervalDropdown)}
                  className="flex items-center gap-2 px-3 py-1.5 rounded-[8px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all"
                >
                  <span className={`text-[12px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                    }`}>{contributorInterval}</span>
                  <ChevronDown className={`w-3.5 h-3.5 transition-colors ${theme === 'dark' ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                    }`} />
                </button>
                {showContributorIntervalDropdown && (
                  <div className="absolute right-0 mt-2 w-[160px] backdrop-blur-[30px] bg-white/[0.55] border-2 border-white/30 rounded-[10px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-50">
                    <button
                      onClick={() => {
                        setContributorInterval('daily');
                        setShowContributorIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-2 text-left text-[12px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all"
                    >
                      Daily interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval('weekly');
                        setShowContributorIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-2 text-left text-[12px] font-medium text-[#2d2820] hover:bg-white/[0.3] transition-all"
                    >
                      Weekly interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval('monthly');
                        setShowContributorIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-2 text-left text-[12px] font-medium transition-all ${contributorInterval === 'monthly'
                          ? 'bg-white/[0.35] text-[#2d2820] font-bold'
                          : 'text-[#2d2820] hover:bg-white/[0.3]'
                        }`}
                    >
                      Monthly interval
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Chart */}
            <div className="h-[140px] mb-6">
              <ResponsiveContainer width="100%" height="100%">
                <ComposedChart data={contributorActivityData}>
                  <defs>
                    <linearGradient id="contributorBarGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="0%" stopColor="#c9983a" stopOpacity={0.8} />
                      <stop offset="100%" stopColor="#d4af37" stopOpacity={0.4} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(122, 107, 90, 0.1)" />
                  <XAxis
                    dataKey="month"
                    stroke="#7a6b5a"
                    tick={{ fill: '#7a6b5a', fontSize: 10, fontWeight: 600 }}
                    angle={-45}
                    textAnchor="end"
                    height={50}
                  />
                  <Tooltip content={<CustomTooltip />} />
                  <Bar
                    dataKey="value"
                    fill="url(#contributorBarGradient)"
                    radius={[4, 4, 0, 0]}
                    maxBarSize={30}
                  />
                  <RechartsLine
                    type="monotone"
                    dataKey="trend"
                    stroke="#2d2820"
                    strokeWidth={2}
                    dot={false}
                  />
                </ComposedChart>
              </ResponsiveContainer>
            </div>

            {/* Filters */}
            <div className="flex flex-wrap items-center gap-1.5">
              <button
                onClick={() => toggleContributorFilter('new')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${contributorFilters.new
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                New
              </button>
              <button
                onClick={() => toggleContributorFilter('reactivated')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${contributorFilters.reactivated
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                Reactivated
              </button>
              <button
                onClick={() => toggleContributorFilter('active')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${contributorFilters.active
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                Active
              </button>
              <button
                onClick={() => toggleContributorFilter('churned')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${contributorFilters.churned
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                Churned
              </button>
              <button
                onClick={() => toggleContributorFilter('prMerged')}
                className={`px-3 py-1.5 rounded-[8px] text-[12px] font-semibold transition-all ${contributorFilters.prMerged
                    ? 'bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]'
                    : 'backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]'
                  }`}
              >
                PR merged
              </button>
            </div>
          </div>
        </div>

        {/* Right Column - Contributors Map */}
        <div
          className={`backdrop-blur-[40px] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
            theme === "dark" ? "bg-white/[0.15]" : "bg-white/[0.12]"
          }`}
        >
            <h2
              className={`text-[18px] font-bold mb-6 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              Contributors map
            </h2>

            {/* World Map Visualization */}
            <div className="relative h-[200px] md:h-[280px] mb-4 md:mb-6 rounded-[12px] md:rounded-[16px] backdrop-blur-[20px] bg-gradient-to-br from-[#2d2820]/80 via-[#1a1410]/70 to-[#2d2820]/80 border border-white/10 overflow-hidden touch-pan-x touch-pan-y">
              {/* Map Background Pattern */}
              <div className="absolute inset-0 opacity-20">
                <svg
                  width="100%"
                  height="100%"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <defs>
                    <pattern
                      id="grid"
                      width="20"
                      height="20"
                      patternUnits="userSpaceOnUse"
                    >
                      <path
                        d="M 20 0 L 0 0 0 20"
                        fill="none"
                        stroke="rgba(201,152,58,0.2)"
                        strokeWidth="0.5"
                      />
                    </pattern>
                  </defs>
                  <rect width="100%" height="100%" fill="url(#grid)" />
                </svg>
              </div>

              {/* World Map SVG */}
              <div className="absolute inset-0 w-full h-full">
                <ComposableMap
                  projection="geoMercator"
                  projectionConfig={{
                    scale: 100,
                  }}
                  className="w-full h-full"
                >
                  <defs>
                    <linearGradient
                      id="mapGradient"
                      x1="0%"
                      y1="0%"
                      x2="100%"
                      y2="100%"
                    >
                      <stop offset="0%" stopColor="#c9983a" stopOpacity="0.3" />
                      <stop
                        offset="50%"
                        stopColor="#d4af37"
                        stopOpacity="0.25"
                      />
                      <stop
                        offset="100%"
                        stopColor="#c9983a"
                        stopOpacity="0.2"
                      />
                    </linearGradient>
                    <filter id="glow">
                      <feGaussianBlur stdDeviation="3" result="coloredBlur" />
                      <feMerge>
                        <feMergeNode in="coloredBlur" />
                        <feMergeNode in="SourceGraphic" />
                      </feMerge>
                    </filter>
                  </defs>
                  <ZoomableGroup
                    zoom={mapZoom}
                    center={mapCenter}
                    onMoveEnd={({ coordinates, zoom }: { coordinates: [number, number]; zoom: number }) => {
                      setMapCenter(coordinates);
                      setMapZoom(zoom);
                    }}
                  >
                    <Geographies geography={geoUrl}>
                      {({ geographies }: { geographies: any[] }) =>
                        geographies.map((geo: any) => {
                          const isHighlighted = Object.keys(
                            countryCoordinates,
                          ).some(
                            (country) =>
                              geo.properties.name === country ||
                              (country === "United Kingdom" &&
                                geo.properties.name === "United Kingdom") || // Add aliases if needed
                              (country === "United States" &&
                                geo.properties.name ===
                                  "United States of America"),
                          );
                          return (
                            <Geography
                              key={geo.rsmKey}
                              geography={geo}
                              fill={
                                isHighlighted
                                  ? "url(#mapGradient)"
                                  : "rgba(255,255,255,0.05)"
                              }
                              stroke="#c9983a"
                              strokeWidth={0.5}
                              style={{
                                default: { outline: "none" },
                                hover: {
                                  fill: "#d4af37",
                                  outline: "none",
                                  opacity: 0.8,
                                },
                                pressed: { outline: "none" },
                              }}
                            />
                          );
                        })
                      }
                    </Geographies>

                    {/* Markers */}
                    {contributorsByRegion.map((region) => {
                      const coords = countryCoordinates[region.name];
                      if (!coords) return null;
                      return (
                        <Marker key={region.name} coordinates={coords}>
                          <circle
                            r={4}
                            fill="#c9983a"
                            stroke="#fff"
                            strokeWidth={1}
                            style={{ filter: "url(#glow)" }}
                          >
                            <animate
                              attributeName="opacity"
                              values="0.6;1;0.6"
                              dur="2s"
                              repeatCount="indefinite"
                            />
                          </circle>
                        </Marker>
                      );
                    })}

                    {/* Simple Connection Lines for visual effect */}
                    <MapLine
                      from={countryCoordinates["United Kingdom"]}
                      to={countryCoordinates["India"]}
                      stroke="#c9983a"
                      strokeWidth={0.5}
                      strokeDasharray="3,3"
                      className="opacity-30"
                    />
                    <MapLine
                      from={countryCoordinates["Canada"]}
                      to={countryCoordinates["Germany"]}
                      stroke="#d4af37"
                      strokeWidth={0.5}
                      strokeDasharray="3,3"
                      className="opacity-30"
                    />
                    <MapLine
                      from={countryCoordinates["Brazil"]}
                      to={countryCoordinates["Spain"]}
                      stroke="#c9983a"
                      strokeWidth={0.5}
                      strokeDasharray="3,3"
                      className="opacity-30"
                    />
                  </ZoomableGroup>
                </ComposableMap>
              </div>

              {/* Map Info Overlay */}
              <div className="absolute top-2 right-2 md:top-4 md:right-4 flex flex-col gap-1.5 md:gap-1">
                <button
                  onClick={() => setMapZoom((z) => Math.min(z * 1.5, 8))}
                  className="w-10 h-10 md:w-8 md:h-8 rounded-[8px] backdrop-blur-[25px] bg-white/[0.2] border border-white/30 flex items-center justify-center text-white font-bold text-[14px] md:text-[11px] hover:bg-white/[0.3] active:bg-white/[0.4] transition-all cursor-pointer touch-manipulation"
                >
                  +
                </button>
                <button
                  onClick={() => setMapZoom((z) => Math.max(z / 1.5, 1))}
                  className="w-10 h-10 md:w-8 md:h-8 rounded-[8px] backdrop-blur-[25px] bg-white/[0.2] border border-white/30 flex items-center justify-center text-white font-bold text-[14px] md:text-[11px] hover:bg-white/[0.3] active:bg-white/[0.4] transition-all cursor-pointer touch-manipulation"
                >
                  −
                </button>
              </div>
            </div>

            {/* Country Bars */}
            <div className="space-y-2 max-h-[250px] md:max-h-[300px] overflow-y-auto pr-2 custom-scrollbar">
              {contributorsByRegion.map((region) => (
                <div
                  key={region.name}
                  className="flex items-center gap-3 group"
                >
                  <div className="flex-1">
                    <div className="flex items-center justify-between mb-1.5">
                      <span
                        className={`text-[13px] font-semibold transition-colors ${
                          theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                        }`}
                      >
                        {region.name}
                      </span>
                      <span className="text-[12px] font-bold text-[#c9983a]">
                        {region.value}
                      </span>
                    </div>
                    <div className="h-6 rounded-[6px] backdrop-blur-[15px] bg-white/[0.08] border border-white/15 overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-[#c9983a] to-[#d4af37] rounded-[6px] transition-all duration-500 group-hover:shadow-[0_0_15px_rgba(201,152,58,0.5)]"
                        style={{ width: `${region.percentage}%` }}
                      />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Bottom Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 md:gap-6">
          {/* Contributor Activity */}
          <div
            className={`backdrop-blur-[40px] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
              theme === "dark" ? "bg-white/[0.15]" : "bg-white/[0.12]"
            }`}
          >
            <div className="flex items-center justify-between mb-6">
              <h2
                className={`text-[18px] font-bold transition-colors ${
                  theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                }`}
              >
                Contributor activity
              </h2>
              <div className="relative">
                <button
                  onClick={() =>
                    setShowContributorIntervalDropdown(
                      !showContributorIntervalDropdown,
                    )
                  }
                  className={`flex items-center gap-2 px-4 py-2 rounded-[10px] backdrop-blur-[20px] border transition-all ${
                    theme === "dark"
                      ? "bg-white/[0.08] border-white/20 hover:bg-white/[0.12] text-[#f5f5f5]"
                      : "bg-white/[0.15] border-white/25 hover:bg-white/[0.2] text-[#2d2820]"
                  }`}
                >
                  <span
                    className={`text-[13px] font-semibold transition-colors ${
                      theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                    }`}
                  >
                    {contributorIntervalLabels[contributorInterval]}
                  </span>
                  <ChevronDown
                    className={`w-4 h-4 transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  />
                </button>
                {showContributorIntervalDropdown && (
                  <div
                    className={`absolute right-0 sm:right-0 left-0 sm:left-auto mt-2 w-full sm:w-[180px] backdrop-blur-[30px] rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-50 ${
                      theme === "dark"
                        ? "bg-[#2d2820]/[0.95] border-2 border-[#c9983a]/30"
                        : "bg-white/[0.75] border-2 border-white/40"
                    }`}
                  >
                    <button
                      onClick={() => {
                        setContributorInterval("daily");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all touch-manipulation ${
                        theme === "dark"
                          ? "text-[#f5f5f5] hover:bg-white/[0.05]"
                          : "text-[#2d2820] hover:bg-white/[0.3]"
                      }`}
                    >
                      Daily interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("weekly");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all touch-manipulation ${
                        theme === "dark"
                          ? "text-[#f5f5f5] hover:bg-white/[0.05]"
                          : "text-[#2d2820] hover:bg-white/[0.3]"
                      }`}
                    >
                      Weekly interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("monthly");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all ${
                        contributorInterval === "monthly"
                          ? theme === "dark"
                            ? "bg-[#c9983a]/[0.25] text-[#f5f5f5] font-bold"
                            : "bg-[#c9983a]/[0.25] text-[#2d2820] font-bold"
                          : theme === "dark"
                            ? "text-[#f5f5f5] hover:bg-white/[0.05]"
                            : "text-[#2d2820] hover:bg-white/[0.3]"
                      }`}
                    >
                      Monthly interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("quarterly");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all touch-manipulation ${
                        theme === "dark"
                          ? "text-[#f5f5f5] hover:bg-white/[0.05]"
                          : "text-[#2d2820] hover:bg-white/[0.3]"
                      }`}
                    >
                      Quarterly interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("yearly");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className={`w-full px-4 py-3 text-left text-[13px] font-medium transition-all touch-manipulation ${
                        theme === "dark"
                          ? "text-[#f5f5f5] hover:bg-white/[0.05]"
                          : "text-[#2d2820] hover:bg-white/[0.3]"
                      }`}
                    >
                      Yearly interval
                    </button>
                  </div>
                )}
              </div>
            </div>

            {/* Chart */}
            <div className="h-[280px] mb-6">
              <ResponsiveContainer width="100%" height="100%">
                <ComposedChart
                  data={contributorChartData}
                  key={contributorInterval}
                >
                  <defs>
                    <linearGradient
                      id="contributorBarGradient"
                      x1="0"
                      y1="0"
                      x2="0"
                      y2="1"
                    >
                      <stop offset="0%" stopColor="#c9983a" stopOpacity={0.8} />
                      <stop
                        offset="100%"
                        stopColor="#d4af37"
                        stopOpacity={0.4}
                      />
                    </linearGradient>
                  </defs>
                  <CartesianGrid
                    strokeDasharray="3 3"
                    stroke="rgba(122, 107, 90, 0.1)"
                  />
                  <XAxis
                    dataKey="label"
                    stroke="#7a6b5a"
                    tick={{ fill: "#7a6b5a", fontSize: 11, fontWeight: 600 }}
                    angle={-45}
                    textAnchor="end"
                    height={80}
                  />
                  <YAxis
                    stroke="#7a6b5a"
                    tick={{ fill: "#7a6b5a", fontSize: 11, fontWeight: 600 }}
                  />
                  <Tooltip content={<CustomTooltip />} />
                  <Bar
                    dataKey="value"
                    fill="url(#contributorBarGradient)"
                    radius={[8, 8, 0, 0]}
                    maxBarSize={40}
                  />
                  <RechartsLine
                    type="monotone"
                    dataKey="trend"
                    stroke="#2d2820"
                    strokeWidth={3}
                    dot={false}
                  />
                </ComposedChart>
              </ResponsiveContainer>
            </div>

            {/* Filters */}
            <div className="flex flex-wrap items-center gap-2">
              <button
                onClick={() => toggleContributorFilter("new")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  contributorFilters.new
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                New
              </button>
              <button
                onClick={() => toggleContributorFilter("reactivated")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  contributorFilters.reactivated
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                Reactivated
              </button>
              <button
                onClick={() => toggleContributorFilter("active")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  contributorFilters.active
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                Active
              </button>
              <button
                onClick={() => toggleContributorFilter("churned")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  contributorFilters.churned
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                Churned
              </button>
              <button
                onClick={() => toggleContributorFilter("prMerged")}
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  contributorFilters.prMerged
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                PR merged
              </button>
            </div>
          </div>

          {/* Information Panel */}
          <div className="col-span-1 md:col-span-2 backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8">
            <h2
              className={`text-[18px] font-bold mb-6 transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              Information
            </h2>

            {/* Info Text */}
            <div className="mb-6 p-5 rounded-[16px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25">
              <div className="flex items-start gap-3">
                <Info className="w-5 h-5 text-[#c9983a] flex-shrink-0 mt-0.5" />
                <p
                  className={`text-[14px] leading-relaxed transition-colors ${
                    theme === "dark" ? "text-[#d4d4d4]" : "text-[#4a3f2f]"
                  }`}
                >
                  Only data from contributors who have completed a KYC are
                  included. Contributors without a completed KYC are excluded
                  from the map.
                </p>
              </div>
            </div>

            {/* Contributor Stats */}
            <div className="space-y-4">
              <div className="flex items-center justify-between p-6 rounded-[16px] backdrop-blur-[25px] bg-gradient-to-br from-white/[0.2] to-white/[0.12] border-2 border-white/30 shadow-[0_6px_24px_rgba(0,0,0,0.08)] hover:shadow-[0_8px_32px_rgba(201,152,58,0.15)] transition-all group">
                <div>
                  <h3
                    className={`text-[14px] font-bold uppercase tracking-wider mb-2 transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Contributors with billing profile
                  </h3>
                  <div
                    className={`text-[42px] font-black leading-none transition-colors ${
                      theme === "dark"
                        ? "text-[#f5f5f5]"
                        : "bg-gradient-to-r from-[#2d2820] to-[#c9983a] bg-clip-text text-transparent"
                    }`}
                  >
                    0 / 0
                  </div>
                </div>
                <div className="w-16 h-16 rounded-[16px] bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 flex items-center justify-center shadow-[0_4px_16px_rgba(201,152,58,0.25)] group-hover:scale-110 group-hover:shadow-[0_6px_24px_rgba(201,152,58,0.4)] transition-all duration-300">
                  <svg
                    className="w-8 h-8 text-[#c9983a]"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
                    />
                  </svg>
                </div>
              </div>

              {/* Additional Stats Placeholder */}
              <div className="grid grid-cols-2 gap-4">
                <div className="p-5 rounded-[14px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all group cursor-pointer">
                  <div
                    className={`text-[11px] font-bold uppercase tracking-wider mb-2 transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Active
                  </div>
                  <div
                    className={`text-[28px] font-black transition-colors ${
                      theme === "dark"
                        ? "text-[#f5f5f5] group-hover:text-[#c9983a]"
                        : "text-[#2d2820] group-hover:text-[#c9983a]"
                    }`}
                  >
                    0
                  </div>
                </div>
                <div className="p-5 rounded-[14px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all group cursor-pointer">
                  <div
                    className={`text-[11px] font-bold uppercase tracking-wider mb-2 transition-colors ${
                      theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
                    }`}
                  >
                    Total
                  </div>
                  <div
                    className={`text-[28px] font-black transition-colors ${
                      theme === "dark"
                        ? "text-[#f5f5f5] group-hover:text-[#c9983a]"
                        : "text-[#2d2820] group-hover:text-[#c9983a]"
                    }`}
                  >
                    0
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <style>{`
        .custom-scrollbar::-webkit-scrollbar {
          width: 6px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: rgba(255, 255, 255, 0.1);
          border-radius: 10px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: rgba(201, 152, 58, 0.5);
          border-radius: 10px;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: rgba(201, 152, 58, 0.7);
        }
        .scrollbar-hide {
          -ms-overflow-style: none;
          scrollbar-width: none;
        }
        .scrollbar-hide::-webkit-scrollbar {
          display: none;
        }
        .touch-manipulation {
          touch-action: manipulation;
        }
      `}</style>
      </div>
    
  );
}
