import { useState, useEffect } from 'react';
import { ChevronDown, Info, Download, Camera, AlertCircle, Loader2 } from 'lucide-react';
import { BarChart, Bar, LineChart, Line as RechartsLine, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, ComposedChart } from 'recharts';
import { ComposableMap, Geographies, Geography, Marker, ZoomableGroup, Line as MapLine } from "react-simple-maps";
import { useTheme } from '../../../shared/contexts/ThemeContext';
import html2canvas from 'html2canvas';

export function DataPage() {
  const { theme } = useTheme();
  const [mapZoom, setMapZoom] = useState(1);
  const [mapCenter, setMapCenter] = useState<[number, number]>([0, 0]);
  const [hiddenSeries, setHiddenSeries] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState({ project: false, contributor: false });
  const [error, setError] = useState<{ project?: string; contributor?: string }>({});
  const [showExportOptions, setShowExportOptions] = useState<{ project: boolean; contributor: boolean }>({ project: false, contributor: false });
  const [hoveredCountry, setHoveredCountry] = useState<string | null>(null);
  const [mapTooltip, setMapTooltip] = useState<{ x: number; y: number; country: string; value: number; percentage: number } | null>(null);

  const toggleSeries = (key: string) => {
    setHiddenSeries(prev => ({ ...prev, [key]: !prev[key] }));
  };

  const geoUrl = "https://cdn.jsdelivr.net/npm/world-atlas@2/countries-110m.json";

  const countryCoordinates: Record<string, [number, number]> = {
    'United Kingdom': [-3.435973, 55.378051],
    'Germany': [10.451526, 51.165691],
    'Canada': [-106.346771, 56.130366],
    'India': [78.96288, 20.593684],
    'Brazil': [-51.92528, -14.235004],
    'Netherlands': [5.291266, 52.132633],
    'Australia': [133.775136, -25.274398],
    'Spain': [-3.74922, 40.463667],
    'Italy': [12.56738, 41.87194],
    'Poland': [19.145136, 51.919438],
    'Sweden': [18.643501, 60.128161],
    'Japan': [138.252924, 36.204824],
    'China': [104.195397, 35.86166],
  };

  const [activeTab, setActiveTab] = useState('overview');
  const [projectInterval, setProjectInterval] = useState('Monthly interval');
  const [contributorInterval, setContributorInterval] = useState('Monthly interval');
  const [showProjectIntervalDropdown, setShowProjectIntervalDropdown] = useState(false);
  const [showContributorIntervalDropdown, setShowContributorIntervalDropdown] = useState(false);
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

  const projectActivityData = [
    { month: 'January', value: 45, trend: 40, new: 12, reactivated: 5, active: 28, churned: -8, rewarded: 15420 },
    { month: 'February', value: 38, trend: 42, new: 8, reactivated: 4, active: 26, churned: -6, rewarded: 12300 },
    { month: 'March', value: 52, trend: 45, new: 15, reactivated: 7, active: 30, churned: -5, rewarded: 18650 },
    { month: 'April', value: 48, trend: 50, new: 11, reactivated: 6, active: 31, churned: -7, rewarded: 16800 },
    { month: 'May', value: 58, trend: 52, new: 18, reactivated: 8, active: 32, churned: -4, rewarded: 22100 },
    { month: 'June', value: 55, trend: 55, new: 14, reactivated: 6, active: 35, churned: -9, rewarded: 20500 },
    { month: 'July', value: 42, trend: 54, new: 9, reactivated: 5, active: 28, churned: -10, rewarded: 14200 },
    { month: 'August', value: 48, trend: 50, new: 12, reactivated: 7, active: 29, churned: -6, rewarded: 17300 },
    { month: 'September', value: 62, trend: 52, new: 20, reactivated: 9, active: 33, churned: -5, rewarded: 24800 },
    { month: 'October', value: 58, trend: 58, new: 16, reactivated: 8, active: 34, churned: -7, rewarded: 21900 },
    { month: 'November', value: 45, trend: 56, new: 10, reactivated: 6, active: 29, churned: -8, rewarded: 15600 },
    { month: 'December', value: 52, trend: 52, new: 13, reactivated: 7, active: 32, churned: -10, rewarded: 18900 },
  ];

  const contributorActivityData = [
    { month: 'January', value: 42, trend: 38, new: 10, reactivated: 4, active: 28, churned: -6, rewarded: 14200 },
    { month: 'February', value: 35, trend: 40, new: 7, reactivated: 3, active: 25, churned: -5, rewarded: 11800 },
    { month: 'March', value: 48, trend: 42, new: 13, reactivated: 6, active: 29, churned: -4, rewarded: 16900 },
    { month: 'April', value: 45, trend: 46, new: 11, reactivated: 5, active: 29, churned: -6, rewarded: 15300 },
    { month: 'May', value: 38, trend: 44, new: 8, reactivated: 4, active: 26, churned: -7, rewarded: 12700 },
    { month: 'June', value: 52, trend: 45, new: 15, reactivated: 7, active: 30, churned: -5, rewarded: 19100 },
    { month: 'July', value: 48, trend: 48, new: 12, reactivated: 6, active: 30, churned: -8, rewarded: 17400 },
    { month: 'August', value: 55, trend: 50, new: 17, reactivated: 8, active: 30, churned: -4, rewarded: 21300 },
    { month: 'September', value: 50, trend: 52, new: 14, reactivated: 7, active: 29, churned: -6, rewarded: 18600 },
    { month: 'October', value: 58, trend: 54, new: 19, reactivated: 9, active: 30, churned: -5, rewarded: 23800 },
    { month: 'November', value: 52, trend: 56, new: 15, reactivated: 7, active: 30, churned: -7, rewarded: 19500 },
    { month: 'December', value: 48, trend: 52, new: 12, reactivated: 6, active: 30, churned: -8, rewarded: 17200 },
  ];

  const contributorsByRegion = [
    { name: 'United Kingdom', value: 625, percentage: 45, color: '#c9983a' },
    { name: 'Germany', value: 720, percentage: 52, color: '#d4af37' },
    { name: 'Canada', value: 580, percentage: 42, color: '#c9983a' },
    { name: 'India', value: 560, percentage: 40, color: '#b8860b' },
    { name: 'Brazil', value: 490, percentage: 35, color: '#daa520' },
    { name: 'Netherlands', value: 300, percentage: 22, color: '#cd9b1d' },
    { name: 'Australia', value: 430, percentage: 31, color: '#c9983a' },
    { name: 'Spain', value: 280, percentage: 20, color: '#b8860b' },
    { name: 'Italy', value: 220, percentage: 16, color: '#daa520' },
    { name: 'Poland', value: 280, percentage: 20, color: '#cd9b1d' },
    { name: 'Sweden', value: 210, percentage: 15, color: '#c9983a' },
    { name: 'Japan', value: 240, percentage: 17, color: '#b8860b' },
    { name: 'China', value: 220, percentage: 16, color: '#daa520' },
  ];

  const toggleProjectFilter = (filter: keyof typeof projectFilters) => {
    setProjectFilters(prev => ({ ...prev, [filter]: !prev[filter] }));
  };

  const toggleContributorFilter = (filter: keyof typeof contributorFilters) => {
    setContributorFilters(prev => ({ ...prev, [filter]: !prev[filter] }));
  };

  // Enhanced export with format selection
  const exportChartData = async (data: any[], filename: string, chartName: string, chartElementId: string, format: 'csv' | 'png') => {
    if (format === 'csv') {
      setLoading(prev => ({ ...prev, [chartName === 'Project' ? 'project' : 'contributor']: true }));
      try {
        // Simulate loading delay
        await new Promise(resolve => setTimeout(resolve, 500));
        
        const csvContent = [
          Object.keys(data[0]).join(','),
          ...data.map(row => Object.values(row).join(','))
        ].join('\n');
        
        const blob = new Blob([csvContent], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${filename}_${new Date().toISOString().split('T')[0]}.csv`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        
        const announcement = document.createElement('div');
        announcement.setAttribute('role', 'status');
        announcement.setAttribute('aria-live', 'polite');
        announcement.className = 'sr-only';
        announcement.textContent = `${chartName} chart data exported as CSV successfully`;
        document.body.appendChild(announcement);
        setTimeout(() => document.body.removeChild(announcement), 3000);
      } catch (err) {
        setError(prev => ({ ...prev, [chartName === 'Project' ? 'project' : 'contributor']: 'Export failed. Please try again.' }));
      } finally {
        setLoading(prev => ({ ...prev, [chartName === 'Project' ? 'project' : 'contributor']: false }));
      }
    } else if (format === 'png') {
      setLoading(prev => ({ ...prev, [chartName === 'Project' ? 'project' : 'contributor']: true }));
      try {
        const chartElement = document.getElementById(chartElementId);
        if (chartElement) {
          const canvas = await html2canvas(chartElement);
          const link = document.createElement('a');
          link.download = `${filename}_${new Date().toISOString().split('T')[0]}.png`;
          link.href = canvas.toDataURL();
          link.click();
          
          const announcement = document.createElement('div');
          announcement.setAttribute('role', 'status');
          announcement.setAttribute('aria-live', 'polite');
          announcement.className = 'sr-only';
          announcement.textContent = `${chartName} chart exported as PNG successfully`;
          document.body.appendChild(announcement);
          setTimeout(() => document.body.removeChild(announcement), 3000);
        }
      } catch (err) {
        setError(prev => ({ ...prev, [chartName === 'Project' ? 'project' : 'contributor']: 'PNG export failed. Please try again.' }));
      } finally {
        setLoading(prev => ({ ...prev, [chartName === 'Project' ? 'project' : 'contributor']: false }));
      }
    }
  };

  const CustomTooltip = ({ active, payload, label }: any) => {
    if (active && payload && payload.length) {
      const data = payload[0].payload;
      return (
        <div role="tooltip" aria-live="polite" className="backdrop-blur-[30px] bg-[#1a1410]/95 border-2 border-white/20 rounded-[12px] px-5 py-4 min-w-[240px] shadow-xl">
          <p className="text-[13px] font-bold text-white mb-3">{data.month} 2025</p>
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#c9983a]" />
                <span className="text-[12px] text-white/80">New Contributors</span>
              </div>
              <span className="text-[13px] font-bold text-[#c9983a]">{data.new}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#d4af37]" />
                <span className="text-[12px] text-white/80">Reactivated</span>
              </div>
              <span className="text-[13px] font-bold text-[#d4af37]">{data.reactivated}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#c9983a]/70" />
                <span className="text-[12px] text-white/80">Active Contributors</span>
              </div>
              <span className="text-[13px] font-bold text-[#c9983a]/90">{data.active}</span>
            </div>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-[#ff6b6b]" />
                <span className="text-[12px] text-white/80">Churned</span>
              </div>
              <span className="text-[13px] font-bold text-[#ff6b6b]">{data.churned}</span>
            </div>
            <div className="h-px bg-white/10 my-2" />
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-gradient-to-r from-[#c9983a] to-[#d4af37]" />
                <span className="text-[12px] text-white/80">Total Rewarded</span>
              </div>
              <span className="text-[13px] font-bold text-white">{data.rewarded.toLocaleString()} USD</span>
            </div>
          </div>
          {/* Action link as required */}
          <button 
            onClick={() => console.log('View details for', data.month)}
            className="mt-3 text-[11px] text-[#c9983a] hover:text-[#d4af37] transition-colors w-full text-center border-t border-white/10 pt-2"
            aria-label={`View detailed report for ${data.month}`}
          >
            View detailed report →
          </button>
        </div>
      );
    }
    return null;
  };

  // Map tooltip component
  const MapTooltip = () => {
    if (!mapTooltip) return null;
    return (
      <div 
        className="fixed backdrop-blur-[30px] bg-[#1a1410]/95 border-2 border-[#c9983a]/50 rounded-[12px] px-4 py-3 min-w-[200px] z-50 pointer-events-none"
        style={{ left: mapTooltip.x + 10, top: mapTooltip.y - 10 }}
      >
        <p className="text-[13px] font-bold text-[#c9983a] mb-2">{mapTooltip.country}</p>
        <div className="space-y-1">
          <div className="flex justify-between">
            <span className="text-[11px] text-white/80">Contributors:</span>
            <span className="text-[12px] font-bold text-white">{mapTooltip.value}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-[11px] text-white/80">Percentage:</span>
            <span className="text-[12px] font-bold text-[#d4af37]">{mapTooltip.percentage}%</span>
          </div>
        </div>
      </div>
    );
  };

  // Loading state component
  const LoadingState = () => (
    <div className="flex items-center justify-center h-[280px]">
      <div className="text-center">
        <Loader2 className="w-8 h-8 text-[#c9983a] animate-spin mx-auto mb-3" />
        <p className="text-[13px] text-white/60">Loading chart data...</p>
      </div>
    </div>
  );

  // Empty state component
  const EmptyState = ({ message }: { message: string }) => (
    <div className="flex items-center justify-center h-[280px]">
      <div className="text-center">
        <AlertCircle className="w-8 h-8 text-white/40 mx-auto mb-3" />
        <p className="text-[13px] text-white/60">{message}</p>
      </div>
    </div>
  );

  // Error state component
  const ErrorState = ({ message, onRetry }: { message: string; onRetry: () => void }) => (
    <div className="flex items-center justify-center h-[280px]">
      <div className="text-center">
        <AlertCircle className="w-8 h-8 text-[#ff6b6b] mx-auto mb-3" />
        <p className="text-[13px] text-[#ff6b6b] mb-3">{message}</p>
        <button 
          onClick={onRetry}
          className="px-3 py-1.5 rounded-[8px] bg-[#c9983a]/20 text-[#c9983a] text-[12px] font-semibold hover:bg-[#c9983a]/30 transition-colors"
        >
          Retry
        </button>
      </div>
    </div>
  );

  return (
    <div className="space-y-6">
      <MapTooltip />

      {/* Header Tabs */}
      <div
        className={`backdrop-blur-[40px] rounded-[24px] border p-2 transition-colors ${
          theme === "dark"
            ? "bg-white/[0.12] border-white/20"
            : "bg-white/[0.12] border-white/20"
        }`}
      >
        <div className="flex items-center gap-2">
          <button
            onClick={() => setActiveTab("overview")}
            className={`px-6 py-3 rounded-[16px] font-bold text-[14px] transition-all duration-300 ${
              activeTab === "overview"
                ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${
                    theme === "dark" ? "text-[#f5c563]" : "text-[#2d2820]"
                  }`
                : `${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"} hover:bg-white/[0.08]`
            }`}
          >
            Overview
          </button>
          <button
            onClick={() => setActiveTab("projects")}
            className={`px-6 py-3 rounded-[16px] font-bold text-[14px] transition-all duration-300 ${
              activeTab === "projects"
                ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${
                    theme === "dark" ? "text-[#f5c563]" : "text-[#2d2820]"
                  }`
                : `${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"} hover:bg-white/[0.08]`
            }`}
          >
            Projects
          </button>
          <button
            onClick={() => setActiveTab("contributions")}
            className={`px-6 py-3 rounded-[16px] font-bold text-[14px] transition-all duration-300 ${
              activeTab === "contributions"
                ? `bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/20 border-2 border-[#c9983a]/50 ${
                    theme === "dark" ? "text-[#f5c563]" : "text-[#2d2820]"
                  }`
                : `${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"} hover:bg-white/[0.08]`
            }`}
          >
            Contributions
          </button>
        </div>
      </div>

      {/* Main Content Grid */}
      <div className="grid grid-cols-2 gap-6">
        {/* Left Column - Project Activity */}
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 p-8">
          <div className="flex items-center justify-between mb-6">
            <h2
              className={`text-[18px] font-bold transition-colors ${
                theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              Project activity
            </h2>
            <div className="flex items-center gap-2">
              {/* Enhanced Export with format selection */}
              <div className="relative">
                <button
                  aria-label="Export chart data"
                  aria-expanded={showExportOptions.project}
                  onClick={() =>
                    setShowExportOptions((prev) => ({
                      ...prev,
                      project: !prev.project,
                    }))
                  }
                  className="flex items-center gap-2 px-4 py-2 rounded-[10px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all focus-visible:outline-2 focus-visible:outline-[#c9983a] focus-visible:outline-offset-2"
                >
                  <Download className="w-4 h-4" />
                  <span
                    className={`text-[13px] font-semibold ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
                  >
                    Export
                  </span>
                </button>
                {showExportOptions.project && (
                  <div className="absolute right-0 mt-2 w-[160px] backdrop-blur-[30px] bg-white/[0.55] border-2 border-white/30 rounded-[12px] shadow-lg overflow-hidden z-50">
                    <button
                      onClick={() => {
                        exportChartData(
                          projectActivityData,
                          "project_activity",
                          "Project",
                          "project-chart",
                          "csv",
                        );
                        setShowExportOptions((prev) => ({
                          ...prev,
                          project: false,
                        }));
                      }}
                      className="w-full px-4 py-2.5 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] flex items-center gap-2"
                    >
                      <Download className="w-3.5 h-3.5" />
                      CSV Format
                    </button>
                    <button
                      onClick={() => {
                        exportChartData(
                          projectActivityData,
                          "project_activity",
                          "Project",
                          "project-chart",
                          "png",
                        );
                        setShowExportOptions((prev) => ({
                          ...prev,
                          project: false,
                        }));
                      }}
                      className="w-full px-4 py-2.5 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] flex items-center gap-2"
                    >
                      <Camera className="w-3.5 h-3.5" />
                      PNG Format
                    </button>
                  </div>
                )}
              </div>
              <div className="relative">
                <button
                  onClick={() =>
                    setShowProjectIntervalDropdown(!showProjectIntervalDropdown)
                  }
                  className="flex items-center gap-2 px-4 py-2 rounded-[10px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all"
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
                  <div className="absolute right-0 mt-2 w-[180px] backdrop-blur-[30px] bg-white/[0.55] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-50">
                    <button
                      onClick={() => {
                        setProjectInterval("Daily interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Daily interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Weekly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Weekly interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Monthly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium bg-white/[0.35] text-[#2d2820] font-bold"
                    >
                      Monthly interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Quarterly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Quarterly interval
                    </button>
                    <button
                      onClick={() => {
                        setProjectInterval("Yearly interval");
                        setShowProjectIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Yearly interval
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Legend */}
          <div
            className="flex flex-wrap gap-2 mb-4"
            role="group"
            aria-label="Chart series toggle"
          >
            {[
              { key: "new", label: "New", color: "#c9983a" },
              { key: "reactivated", label: "Reactivated", color: "#d4af37" },
              { key: "active", label: "Active", color: "#c9983a" },
              { key: "churned", label: "Churned", color: "#ff6b6b" },
              { key: "trend", label: "Trend", color: "#2d2820" },
            ].map(({ key, label, color }) => (
              <button
                key={key}
                role="button"
                tabIndex={0}
                aria-pressed={!hiddenSeries[key]}
                aria-label={`Toggle ${label} series`}
                onClick={() => toggleSeries(key)}
                onKeyDown={(e) =>
                  e.key === "Enter" || e.key === " " ? toggleSeries(key) : null
                }
                className={`flex items-center gap-2 px-3 py-1.5 rounded-[10px] border text-[13px] font-semibold transition-all focus-visible:outline-2 focus-visible:outline-[#c9983a] focus-visible:outline-offset-2 ${
                  hiddenSeries[key]
                    ? "opacity-40 border-white/15 bg-transparent"
                    : "border-white/25 bg-white/[0.15]"
                }`}
              >
                <span
                  className="w-3 h-3 rounded-[3px] flex-shrink-0"
                  style={{
                    background: color,
                    filter: hiddenSeries[key] ? "grayscale(1)" : "none",
                  }}
                />
                <span
                  className={
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }
                >
                  {label}
                </span>
              </button>
            ))}
          </div>

          {/* Chart with loading/empty/error states */}
          <div id="project-chart">
            {loading.project ? (
              <LoadingState />
            ) : error.project ? (
              <ErrorState
                message={error.project}
                onRetry={() =>
                  setError((prev) => ({ ...prev, project: undefined }))
                }
              />
            ) : projectActivityData.length === 0 ? (
              <EmptyState message="No data available for the selected period" />
            ) : (
              <div
                role="img"
                aria-label="Project activity bar chart showing monthly trends with new, reactivated, active, churned, and trend data"
                tabIndex={0}
                className="h-[280px] mb-6 focus-visible:outline-2 focus-visible:outline-[#c9983a] focus-visible:outline-offset-2 rounded-[8px]"
              >
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
                        <stop
                          offset="0%"
                          stopColor="#c9983a"
                          stopOpacity={0.8}
                        />
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
                    <Tooltip
                      content={<CustomTooltip />}
                      cursor={{ fill: "rgba(201, 152, 58, 0.1)" }}
                    />
                    <Bar
                      dataKey="new"
                      hide={hiddenSeries["new"]}
                      fill="#c9983a"
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <Bar
                      dataKey="reactivated"
                      hide={hiddenSeries["reactivated"]}
                      fill="#d4af37"
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <Bar
                      dataKey="active"
                      hide={hiddenSeries["active"]}
                      fill="#c9983a"
                      fillOpacity={0.7}
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <Bar
                      dataKey="churned"
                      hide={hiddenSeries["churned"]}
                      fill="#ff6b6b"
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <RechartsLine
                      type="monotone"
                      dataKey="trend"
                      hide={hiddenSeries["trend"]}
                      stroke="#2d2820"
                      strokeWidth={3}
                      dot={false}
                    />
                  </ComposedChart>
                </ResponsiveContainer>
              </div>
            )}
          </div>
          {/* Screen reader fallback — chart data as table for accessibility */}
          <table className="sr-only">
            <caption>Project activity data by month</caption>
            <thead>
              <tr>
                <th scope="col">Month</th>
                <th scope="col">New</th>
                <th scope="col">Reactivated</th>
                <th scope="col">Active</th>
                <th scope="col">Churned</th>
                <th scope="col">Rewarded (USD)</th>
              </tr>
            </thead>
            <tbody>
              {projectActivityData.map((row) => (
                <tr key={row.month}>
                  <td>{row.month}</td>
                  <td>{row.new}</td>
                  <td>{row.reactivated}</td>
                  <td>{row.active}</td>
                  <td>{row.churned}</td>
                  <td>{row.rewarded.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>

          {/* Filters */}
          <div className="flex flex-wrap items-center gap-2">
            {Object.keys(projectFilters).map((filter) => (
              <button
                key={filter}
                onClick={() =>
                  toggleProjectFilter(filter as keyof typeof projectFilters)
                }
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${
                  projectFilters[filter as keyof typeof projectFilters]
                    ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]"
                    : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"
                }`}
              >
                {filter.charAt(0).toUpperCase() +
                  filter.slice(1).replace("prMerged", "PR merged")}
              </button>
            ))}
          </div>
        </div>

        {/* Right Column - Contributors Map */}
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8">
          <h2
            className={`text-[18px] font-bold mb-6 transition-colors ${
              theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
            }`}
          >
            Contributors map
          </h2>

          {/* Color scale legend for map */}
          <div className="mb-4 p-3 rounded-[12px] backdrop-blur-[15px] bg-white/[0.08] border border-white/15">
            <p className="text-[11px] font-semibold text-white/60 mb-2">
              Contributor Density
            </p>
            <div className="flex items-center gap-2">
              <div className="flex-1 h-2 rounded-full bg-gradient-to-r from-[#b8860b] via-[#c9983a] to-[#d4af37]" />
              <div className="flex justify-between text-[10px] text-white/40 w-full mt-1">
                <span>Low</span>
                <span>Medium</span>
                <span>High</span>
              </div>
            </div>
          </div>

          {/* World Map Visualization with hover state */}
          <div className="relative h-[280px] mb-6 rounded-[16px] backdrop-blur-[20px] bg-gradient-to-br from-[#2d2820]/80 via-[#1a1410]/70 to-[#2d2820]/80 border border-white/10 overflow-hidden">
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

            <div className="absolute inset-0 w-full h-full">
              <ComposableMap
                role="img"
                aria-label="World map showing contributor distribution by country"
                projection="geoMercator"
                projectionConfig={{ scale: 100 }}
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
                    <stop offset="50%" stopColor="#d4af37" stopOpacity="0.25" />
                    <stop offset="100%" stopColor="#c9983a" stopOpacity="0.2" />
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
                  onMoveEnd={({ coordinates, zoom }) => {
                    setMapCenter(coordinates as [number, number]);
                    setMapZoom(zoom);
                  }}
                >
                  <Geographies geography={geoUrl}>
                    {({ geographies }) =>
                      geographies.map((geo) => {
                        const countryData = contributorsByRegion.find(
                          (c) => c.name === geo.properties.name,
                        );
                        const isHighlighted = !!countryData;
                        return (
                          <Geography
                            key={geo.rsmKey}
                            geography={geo}
                            fill={
                              isHighlighted
                                ? "url(#mapGradient)"
                                : "rgba(255,255,255,0.05)"
                            }
                            stroke={
                              hoveredCountry === geo.properties.name
                                ? "#d4af37"
                                : "#c9983a"
                            }
                            strokeWidth={
                              hoveredCountry === geo.properties.name ? 2 : 0.5
                            }
                            style={{
                              default: {
                                outline: "none",
                                transition: "all 0.2s ease",
                              },
                              hover: {
                                fill: "#d4af37",
                                outline: "none",
                                opacity: 0.8,
                                stroke: "#fff",
                                strokeWidth: 2,
                              },
                              pressed: { outline: "none" },
                            }}
                            onMouseEnter={(e) => {
                              setHoveredCountry(geo.properties.name);
                              if (countryData) {
                                setMapTooltip({
                                  x: e.clientX,
                                  y: e.clientY,
                                  country: countryData.name,
                                  value: countryData.value,
                                  percentage: countryData.percentage,
                                });
                              }
                            }}
                            onMouseLeave={() => {
                              setHoveredCountry(null);
                              setMapTooltip(null);
                            }}
                          />
                        );
                      })
                    }
                  </Geographies>

                  {contributorsByRegion.map((region) => {
                    const coords = countryCoordinates[region.name];
                    if (!coords) return null;
                    return (
                      <Marker key={region.name} coordinates={coords}>
                        <circle
                          r={hoveredCountry === region.name ? 6 : 4}
                          fill="#c9983a"
                          stroke="#fff"
                          strokeWidth={hoveredCountry === region.name ? 2 : 1}
                          style={{
                            filter: "url(#glow)",
                            transition: "all 0.2s ease",
                          }}
                          onMouseEnter={(e) => {
                            setHoveredCountry(region.name);
                            setMapTooltip({
                              x: e.clientX,
                              y: e.clientY,
                              country: region.name,
                              value: region.value,
                              percentage: region.percentage,
                            });
                          }}
                          onMouseLeave={() => {
                            setHoveredCountry(null);
                            setMapTooltip(null);
                          }}
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
                </ZoomableGroup>
              </ComposableMap>
            </div>

            <div className="absolute top-4 right-4 flex flex-col gap-1">
              <button
                onClick={() => setMapZoom((z) => Math.min(z * 1.5, 8))}
                className="w-8 h-8 rounded-[8px] backdrop-blur-[25px] bg-white/[0.2] border border-white/30 flex items-center justify-center text-white font-bold text-[11px] hover:bg-white/[0.3] transition-all cursor-pointer"
                aria-label="Zoom in on map"
              >
                +
              </button>
              <button
                onClick={() => setMapZoom((z) => Math.max(z / 1.5, 1))}
                className="w-8 h-8 rounded-[8px] backdrop-blur-[25px] bg-white/[0.2] border border-white/30 flex items-center justify-center text-white font-bold text-[11px] hover:bg-white/[0.3] transition-all cursor-pointer"
                aria-label="Zoom out on map"
              >
                −
              </button>
            </div>
          </div>

          {/* Screen reader fallback */}
          <table className="sr-only">
            <caption>Contributors by country</caption>
            <thead>
              <tr>
                <th scope="col">Country</th>
                <th scope="col">Contributors</th>
                <th scope="col">Percentage</th>
              </tr>
            </thead>
            <tbody>
              {contributorsByRegion.map((r) => (
                <tr key={r.name}>
                  <td>{r.name}</td>
                  <td>{r.value}</td>
                  <td>{r.percentage}%</td>
                </tr>
              ))}
            </tbody>
          </table>

          {/* Country Bars */}
          <div className="space-y-2 max-h-[300px] overflow-y-auto pr-2 custom-scrollbar">
            {contributorsByRegion.map((region) => (
              <div
                key={region.name}
                className="flex items-center gap-3 group cursor-pointer"
                onMouseEnter={() => setHoveredCountry(region.name)}
                onMouseLeave={() => setHoveredCountry(null)}
              >
                <div className="flex-1">
                  <div className="flex items-center justify-between mb-1.5">
                    <span
                      className={`text-[13px] font-semibold transition-colors ${hoveredCountry === region.name ? "text-[#d4af37]" : theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
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
      <div className="grid grid-cols-2 gap-6">
        {/* Contributor Activity */}
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8">
          <div className="flex items-center justify-between mb-6">
            <h2
              className={`text-[18px] font-bold transition-colors ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
            >
              Contributor activity
            </h2>
            <div className="flex items-center gap-2">
              <div className="relative">
                <button
                  aria-label="Export chart data"
                  aria-expanded={showExportOptions.contributor}
                  onClick={() =>
                    setShowExportOptions((prev) => ({
                      ...prev,
                      contributor: !prev.contributor,
                    }))
                  }
                  className="flex items-center gap-2 px-4 py-2 rounded-[10px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all"
                >
                  <Download className="w-4 h-4" />
                  <span
                    className={`text-[13px] font-semibold ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
                  >
                    Export
                  </span>
                </button>
                {showExportOptions.contributor && (
                  <div className="absolute right-0 mt-2 w-[160px] backdrop-blur-[30px] bg-white/[0.55] border-2 border-white/30 rounded-[12px] shadow-lg overflow-hidden z-50">
                    <button
                      onClick={() => {
                        exportChartData(
                          contributorActivityData,
                          "contributor_activity",
                          "Contributor",
                          "contributor-chart",
                          "csv",
                        );
                        setShowExportOptions((prev) => ({
                          ...prev,
                          contributor: false,
                        }));
                      }}
                      className="w-full px-4 py-2.5 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] flex items-center gap-2"
                    >
                      <Download className="w-3.5 h-3.5" />
                      CSV Format
                    </button>
                    <button
                      onClick={() => {
                        exportChartData(
                          contributorActivityData,
                          "contributor_activity",
                          "Contributor",
                          "contributor-chart",
                          "png",
                        );
                        setShowExportOptions((prev) => ({
                          ...prev,
                          contributor: false,
                        }));
                      }}
                      className="w-full px-4 py-2.5 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3] flex items-center gap-2"
                    >
                      <Camera className="w-3.5 h-3.5" />
                      PNG Format
                    </button>
                  </div>
                )}
              </div>
              <div className="relative">
                <button
                  onClick={() =>
                    setShowContributorIntervalDropdown(
                      !showContributorIntervalDropdown,
                    )
                  }
                  className="flex items-center gap-2 px-4 py-2 rounded-[10px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all"
                >
                  <span
                    className={`text-[13px] font-semibold ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
                  >
                    {contributorInterval}
                  </span>
                  <ChevronDown
                    className={`w-4 h-4 ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
                  />
                </button>
                {showContributorIntervalDropdown && (
                  <div className="absolute right-0 mt-2 w-[180px] backdrop-blur-[30px] bg-white/[0.55] border-2 border-white/30 rounded-[12px] shadow-[0_8px_32px_rgba(0,0,0,0.15)] overflow-hidden z-50">
                    <button
                      onClick={() => {
                        setContributorInterval("Daily interval");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Daily interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("Weekly interval");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Weekly interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("Monthly interval");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium bg-white/[0.35] text-[#2d2820] font-bold"
                    >
                      Monthly interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("Quarterly interval");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Quarterly interval
                    </button>
                    <button
                      onClick={() => {
                        setContributorInterval("Yearly interval");
                        setShowContributorIntervalDropdown(false);
                      }}
                      className="w-full px-4 py-3 text-left text-[13px] font-medium text-[#2d2820] hover:bg-white/[0.3]"
                    >
                      Yearly interval
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Legend */}
          <div
            className="flex flex-wrap gap-2 mb-4"
            role="group"
            aria-label="Chart series toggle"
          >
            {[
              { key: "new", label: "New", color: "#c9983a" },
              { key: "reactivated", label: "Reactivated", color: "#d4af37" },
              { key: "active", label: "Active", color: "#c9983a" },
              { key: "churned", label: "Churned", color: "#ff6b6b" },
              { key: "trend", label: "Trend", color: "#2d2820" },
            ].map(({ key, label, color }) => (
              <button
                key={key}
                role="button"
                tabIndex={0}
                aria-pressed={!hiddenSeries[key]}
                aria-label={`Toggle ${label} series`}
                onClick={() => toggleSeries(key)}
                onKeyDown={(e) =>
                  e.key === "Enter" || e.key === " " ? toggleSeries(key) : null
                }
                className={`flex items-center gap-2 px-3 py-1.5 rounded-[10px] border text-[13px] font-semibold transition-all focus-visible:outline-2 focus-visible:outline-[#c9983a] focus-visible:outline-offset-2 ${hiddenSeries[key] ? "opacity-40 border-white/15 bg-transparent" : "border-white/25 bg-white/[0.15]"}`}
              >
                <span
                  className="w-3 h-3 rounded-[3px] flex-shrink-0"
                  style={{
                    background: color,
                    filter: hiddenSeries[key] ? "grayscale(1)" : "none",
                  }}
                />
                <span
                  className={
                    theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                  }
                >
                  {label}
                </span>
              </button>
            ))}
          </div>

          <div id="contributor-chart">
            {loading.contributor ? (
              <LoadingState />
            ) : error.contributor ? (
              <ErrorState
                message={error.contributor}
                onRetry={() =>
                  setError((prev) => ({ ...prev, contributor: undefined }))
                }
              />
            ) : contributorActivityData.length === 0 ? (
              <EmptyState message="No data available for the selected period" />
            ) : (
              <div
                role="img"
                aria-label="Contributor activity bar chart showing monthly trends"
                tabIndex={0}
                className="h-[280px] mb-6 focus-visible:outline-2 focus-visible:outline-[#c9983a] focus-visible:outline-offset-2 rounded-[8px]"
              >
                <ResponsiveContainer width="100%" height="100%">
                  <ComposedChart data={contributorActivityData}>
                    <defs>
                      <linearGradient
                        id="contributorBarGradient"
                        x1="0"
                        y1="0"
                        x2="0"
                        y2="1"
                      >
                        <stop
                          offset="0%"
                          stopColor="#c9983a"
                          stopOpacity={0.8}
                        />
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
                    <Tooltip
                      content={<CustomTooltip />}
                      cursor={{ fill: "rgba(201, 152, 58, 0.1)" }}
                    />
                    <Bar
                      dataKey="new"
                      hide={hiddenSeries["new"]}
                      fill="#c9983a"
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <Bar
                      dataKey="reactivated"
                      hide={hiddenSeries["reactivated"]}
                      fill="#d4af37"
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <Bar
                      dataKey="active"
                      hide={hiddenSeries["active"]}
                      fill="#c9983a"
                      fillOpacity={0.7}
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <Bar
                      dataKey="churned"
                      hide={hiddenSeries["churned"]}
                      fill="#ff6b6b"
                      radius={[8, 8, 0, 0]}
                      maxBarSize={40}
                    />
                    <RechartsLine
                      type="monotone"
                      dataKey="trend"
                      hide={hiddenSeries["trend"]}
                      stroke="#2d2820"
                      strokeWidth={3}
                      dot={false}
                    />
                  </ComposedChart>
                </ResponsiveContainer>
              </div>
            )}
          </div>
          {/* Screen reader fallback — chart data as table for accessibility */}
          <table className="sr-only">
            <caption>Contributor activity data by month</caption>
            <thead>
              <tr>
                <th scope="col">Month</th>
                <th scope="col">New</th>
                <th scope="col">Reactivated</th>
                <th scope="col">Active</th>
                <th scope="col">Churned</th>
                <th scope="col">Rewarded (USD)</th>
              </tr>
            </thead>
            <tbody>
              {contributorActivityData.map((row) => (
                <tr key={row.month}>
                  <td>{row.month}</td>
                  <td>{row.new}</td>
                  <td>{row.reactivated}</td>
                  <td>{row.active}</td>
                  <td>{row.churned}</td>
                  <td>{row.rewarded.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <div className="flex flex-wrap items-center gap-2">
            {Object.keys(contributorFilters).map((filter) => (
              <button
                key={filter}
                onClick={() =>
                  toggleContributorFilter(
                    filter as keyof typeof contributorFilters,
                  )
                }
                className={`px-4 py-2 rounded-[10px] text-[13px] font-semibold transition-all ${contributorFilters[filter as keyof typeof contributorFilters] ? "bg-[#c9983a] text-white shadow-[0_3px_12px_rgba(201,152,58,0.3)]" : "backdrop-blur-[20px] bg-white/[0.15] border border-white/25 text-[#2d2820] hover:bg-white/[0.2]"}`}
              >
                {filter.charAt(0).toUpperCase() +
                  filter.slice(1).replace("prMerged", "PR merged")}
              </button>
            ))}
          </div>
        </div>

        {/* Information Panel */}
        <div className="backdrop-blur-[40px] bg-white/[0.12] rounded-[24px] border border-white/20 shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8">
          <h2
            className={`text-[18px] font-bold mb-6 transition-colors ${theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"}`}
          >
            Information
          </h2>
          <div className="mb-6 p-5 rounded-[16px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25">
            <div className="flex items-start gap-3">
              <Info className="w-5 h-5 text-[#c9983a] flex-shrink-0 mt-0.5" />
              <p
                className={`text-[14px] leading-relaxed transition-colors ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#4a3f2f]"}`}
              >
                Only data from contributors who have completed a KYC are
                included. Contributors without a completed KYC are excluded from
                the map.
              </p>
            </div>
          </div>
          <div className="space-y-4">
            <div className="flex items-center justify-between p-6 rounded-[16px] backdrop-blur-[25px] bg-gradient-to-br from-white/[0.2] to-white/[0.12] border-2 border-white/30 shadow-[0_6px_24px_rgba(0,0,0,0.08)] hover:shadow-[0_8px_32px_rgba(201,152,58,0.15)] transition-all group">
              <div>
                <h3
                  className={`text-[14px] font-bold uppercase tracking-wider mb-2 transition-colors ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
                >
                  Contributors with billing profile
                </h3>
                <div
                  className={`text-[42px] font-black leading-none transition-colors ${theme === "dark" ? "text-[#f5f5f5]" : "bg-gradient-to-r from-[#2d2820] to-[#c9983a] bg-clip-text text-transparent"}`}
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
            <div className="grid grid-cols-2 gap-4">
              <div className="p-5 rounded-[14px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all group cursor-pointer">
                <div
                  className={`text-[11px] font-bold uppercase tracking-wider mb-2 transition-colors ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
                >
                  Active
                </div>
                <div
                  className={`text-[28px] font-black transition-colors ${theme === "dark" ? "text-[#f5f5f5] group-hover:text-[#c9983a]" : "text-[#2d2820] group-hover:text-[#c9983a]"}`}
                >
                  0
                </div>
              </div>
              <div className="p-5 rounded-[14px] backdrop-blur-[20px] bg-white/[0.15] border border-white/25 hover:bg-white/[0.2] transition-all group cursor-pointer">
                <div
                  className={`text-[11px] font-bold uppercase tracking-wider mb-2 transition-colors ${theme === "dark" ? "text-[#d4d4d4]" : "text-[#7a6b5a]"}`}
                >
                  Total
                </div>
                <div
                  className={`text-[28px] font-black transition-colors ${theme === "dark" ? "text-[#f5f5f5] group-hover:text-[#c9983a]" : "text-[#2d2820] group-hover:text-[#c9983a]"}`}
                >
                  0
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <style>{`
  .custom-scrollbar::-webkit-scrollbar { width: 6px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: rgba(255, 255, 255, 0.1); border-radius: 10px; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(201, 152, 58, 0.5); border-radius: 10px; }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(201, 152, 58, 0.7); }
  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after {
      animation-duration: 0.01ms !important;
      transition-duration: 0.01ms !important;
    }
  }
`}</style>
    </div>
  );
}