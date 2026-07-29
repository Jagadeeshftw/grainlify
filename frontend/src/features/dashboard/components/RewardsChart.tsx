/**
 * RewardsChart Component
 * 
 * An enhanced rewards distribution chart with:
 * - Responsive donut chart (recharts)
 * - Legend with amounts and percentages
 * - Annotated milestones for significant achievements
 * - Accessible with keyboard navigation and screen reader support
 * - Mobile-first responsive layout (stacked on sm/md, side-by-side on lg+)
 * - WCAG 2.1 AA compliant color contrast
 * 
 * Design Spec: design/profilepage-visualizations.md
 */

import { useState, useMemo } from 'react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts';
import { Trophy, TrendingUp, Target } from 'lucide-react';
import { useTheme } from '../../../shared/contexts/ThemeContext';

interface RewardItem {
  name: string;
  value: number;
  amount: number;
  color: string;
}

interface Milestone {
  icon: React.ReactNode;
  title: string;
  description: string;
}

interface RewardsChartProps {
  data: RewardItem[];
  totalRewards?: number;
  isLoading?: boolean;
}

export function RewardsChart({ 
  data, 
  totalRewards = 0,
  isLoading = false 
}: RewardsChartProps) {
  const { theme } = useTheme();
  const [selectedSegment, setSelectedSegment] = useState<string | null>(null);
  const [focusedIndex, setFocusedIndex] = useState<number | null>(null);

  const isDarkTheme = theme === 'dark';

  // Calculate milestones based on data
  const milestones = useMemo((): Milestone[] => {
    if (data.length === 0) return [];

    const sortedByAmount = [...data].sort((a, b) => b.amount - a.amount);
    const topCategory = sortedByAmount[0];
    const totalValue = data.reduce((sum, item) => sum + item.value, 0);

    return [
      {
        icon: <Trophy className="w-4 h-4 sm:w-5 sm:h-5" />,
        title: 'Highest Earning Category',
        description: `${topCategory.name} contributed $${topCategory.amount.toLocaleString()} to your rewards`
      },
      {
        icon: <TrendingUp className="w-4 h-4 sm:w-5 sm:h-5" />,
        title: 'Total Rewards Earned',
        description: `You've earned $${totalRewards.toLocaleString()} in total rewards this year`
      },
      {
        icon: <Target className="w-4 h-4 sm:w-5 sm:h-5" />,
        title: 'Diversity of Income',
        description: `Rewards distributed across ${data.length} different categories`
      }
    ];
  }, [data, totalRewards]);

  const handleSegmentHover = (index: number) => {
    setFocusedIndex(index);
  };

  const handleSegmentClick = (index: number, name: string) => {
    setSelectedSegment(selectedSegment === name ? null : name);
  };

  const handleKeyDown = (index: number, name: string, event: React.KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      handleSegmentClick(index, name);
    } else if (event.key === 'Escape') {
      setSelectedSegment(null);
    }
  };

  if (data.length === 0) {
    return (
      <div
        className={`text-center py-12 ${
          isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
        }`}
      >
        <Trophy className="w-16 h-16 mx-auto mb-4 opacity-50" />
        <p className="text-base sm:text-lg font-medium">No rewards yet</p>
        <p className="text-sm sm:text-base mt-2">Start contributing to earn rewards!</p>
      </div>
    );
  }

  return (
    <div className="w-full space-y-6 sm:space-y-8">
      {/* Header */}
      <div className="flex flex-col gap-2">
        <h2
          className={`text-lg sm:text-xl font-bold ${
            isDarkTheme ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
          }`}
          id="rewards-title"
        >
          Rewards Distribution 2025
        </h2>
        <div
          className={`text-sm ${isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}
        >
          Total: <span className="font-bold text-lg">${totalRewards.toLocaleString()}</span> USD
        </div>
      </div>

      {/* Main Content: Chart + Legend */}
      <div
        className="flex flex-col lg:flex-row gap-6 lg:gap-8"
        role="region"
        aria-labelledby="rewards-title"
        aria-describedby="rewards-desc"
      >
        <p id="rewards-desc" className="sr-only">
          A rewards distribution chart showing earnings across different categories. 
          Use Tab to navigate through categories, Enter to view details, and Escape to close details.
        </p>

        {/* Chart Container */}
        {isLoading ? (
          <div className="flex-shrink-0 w-full lg:w-1/2 flex items-center justify-center h-[300px] sm:h-[350px] lg:h-[400px]">
            <div className="w-48 h-48 sm:w-56 sm:h-56 bg-neutral-300 rounded-full animate-pulse" />
          </div>
        ) : (
          <div className="flex-shrink-0 w-full lg:w-1/2">
            <div className="relative group/chart h-[300px] sm:h-[350px] lg:h-[400px]">
              {/* Pulsing Glow Behind Chart */}
              <div className="absolute inset-0 bg-gradient-to-br from-[#c9983a]/20 to-[#d4af37]/15 rounded-full blur-2xl group-hover/chart:scale-110 transition-transform duration-500" />

              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={data}
                    cx="50%"
                    cy="50%"
                    innerRadius={60}
                    outerRadius={90}
                    paddingAngle={3}
                    dataKey="value"
                    animationBegin={0}
                    animationDuration={800}
                    animationEasing="ease-out"
                  >
                    {data.map((entry, index) => (
                      <Cell
                        key={`cell-${index}`}
                        fill={entry.color}
                        className="hover:opacity-80 transition-opacity cursor-pointer focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]"
                        opacity={
                          focusedIndex === null || focusedIndex === index ? 1 : 0.6
                        }
                      />
                    ))}
                  </Pie>
                  <Tooltip
                    content={({ active, payload }) => {
                      if (active && payload && payload[0]) {
                        const item = payload[0].payload as RewardItem;
                        return (
                          <div
                            className={`
                              backdrop-blur-[40px] rounded-lg border border-white/25 
                              shadow-lg px-3 sm:px-4 py-2 sm:py-3 text-xs sm:text-sm
                              ${isDarkTheme
                                ? 'bg-[#2d2820]/95 text-[#f5f5f5]'
                                : 'bg-[#e8dfd0]/95 text-[#2d2820]'
                              }
                            `}
                          >
                            <div className="font-bold">{item.name}</div>
                            <div className="text-xs sm:text-sm">
                              ${item.amount.toLocaleString()}
                            </div>
                            <div className={`text-xs ${isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}>
                              {item.value}% of total
                            </div>
                          </div>
                        );
                      }
                      return null;
                    }}
                    offset={50}
                    position={{ y: -80 }}
                    wrapperStyle={{ zIndex: 1000 }}
                    cursor={false}
                  />
                </PieChart>
              </ResponsiveContainer>

              {/* Center Total Display */}
              <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
                <div
                  className={`text-xs sm:text-sm font-bold uppercase tracking-wider mb-1 ${
                    isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}
                >
                  Total
                </div>
                <div className="text-2xl sm:text-3xl lg:text-4xl font-black bg-gradient-to-b from-[#2d2820] to-[#c9983a] bg-clip-text text-transparent leading-none">
                  ${(totalRewards / 1000).toFixed(1)}K
                </div>
                <div
                  className={`text-xs sm:text-sm font-semibold mt-1 ${
                    isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                  }`}
                >
                  USD Earned
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Legend + Details */}
        <div className="w-full lg:w-1/2 space-y-4 sm:space-y-6">
          {/* Legend Items */}
          <div className="space-y-3 sm:space-y-4">
            <h3
              className={`text-sm sm:text-base font-semibold ${
                isDarkTheme ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
              }`}
            >
              Categories
            </h3>
            <div className="space-y-2 sm:space-y-3">
              {data.map((item, index) => (
                <button
                  key={item.name}
                  onClick={() => handleSegmentClick(index, item.name)}
                  onKeyDown={(e) => handleKeyDown(index, item.name, e)}
                  onMouseEnter={() => handleSegmentHover(index)}
                  onMouseLeave={() => setFocusedIndex(null)}
                  aria-label={`${item.name}: $${item.amount.toLocaleString()} (${item.value}% of total rewards)`}
                  className={`
                    w-full backdrop-blur-[20px] rounded-lg border transition-all duration-300
                    p-3 sm:p-4 hover:scale-105 focus-visible:outline-2 focus-visible:outline-offset-2 
                    focus-visible:outline-[#f1b400] cursor-pointer group/card
                    ${selectedSegment === item.name
                      ? 'bg-white/[0.25] border-white/40 shadow-lg'
                      : 'bg-white/[0.15] border-white/25 hover:bg-white/[0.2] hover:border-white/40'
                    }
                  `}
                >
                  <div className="flex items-center gap-3 sm:gap-4">
                    <div
                      className="flex-shrink-0 w-3 h-3 sm:w-4 sm:h-4 rounded-full shadow-md group-hover/card:scale-150 transition-all"
                      style={{ backgroundColor: item.color }}
                    />
                    <div className="flex-1 text-left">
                      <div
                        className={`text-sm sm:text-base font-semibold transition-colors ${
                          isDarkTheme ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                        }`}
                      >
                        {item.name}
                      </div>
                      <div
                        className={`text-xs mt-1 ${
                          isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                        }`}
                      >
                        ${item.amount.toLocaleString()}
                      </div>
                    </div>
                    <div
                      className={`flex-shrink-0 text-sm sm:text-base font-bold transition-all ${
                        selectedSegment === item.name
                          ? 'text-[#f1b400]'
                          : isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#c9983a]'
                      }`}
                    >
                      {item.value}%
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* Milestones Section */}
          {milestones.length > 0 && (
            <div className="space-y-3 sm:space-y-4 pt-4 sm:pt-6 border-t border-white/20">
              <h3
                className={`text-sm sm:text-base font-semibold ${
                  isDarkTheme ? 'text-[#f5f5f5]' : 'text-[#2d2820]'
                }`}
              >
                Milestones
              </h3>
              {milestones.map((milestone, idx) => (
                <div
                  key={idx}
                  className={`
                    p-3 sm:p-4 rounded-lg border-l-3 backdrop-blur-[20px]
                    border-l-[#f1b400]
                    ${isDarkTheme
                      ? 'bg-[#f1b400]/10 text-[#f5f5f5]'
                      : 'bg-[#f1b400]/8 text-[#2d2820]'
                    }
                  `}
                >
                  <div className="flex items-start gap-2 sm:gap-3">
                    <div className="flex-shrink-0 mt-0.5 text-[#f1b400]">
                      {milestone.icon}
                    </div>
                    <div>
                      <div className="text-xs sm:text-sm font-bold">{milestone.title}</div>
                      <div
                        className={`text-xs sm:text-sm mt-1 ${
                          isDarkTheme ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'
                        }`}
                      >
                        {milestone.description}
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Data Table Alternative for Screen Readers */}
      <table className="sr-only" aria-label="Rewards Distribution Details">
        <thead>
          <tr>
            <th>Category</th>
            <th>Amount (USD)</th>
            <th>Percentage</th>
          </tr>
        </thead>
        <tbody>
          {data.map((item) => (
            <tr key={item.name}>
              <td>{item.name}</td>
              <td>${item.amount.toLocaleString()}</td>
              <td>{item.value}%</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
