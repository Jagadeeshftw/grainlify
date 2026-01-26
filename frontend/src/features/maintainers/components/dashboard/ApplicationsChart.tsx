import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { ChartDataPoint } from '../../types';

interface ApplicationsChartProps {
  data: ChartDataPoint[];
}

export function ApplicationsChart({ data }: ApplicationsChartProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  const tooltipBg = isDark
    ? 'bg-neutral-900/80 border-white/10'
    : 'bg-[#e8dfd0]/95 border-white/25';

  const tooltipTitleText = isDark
    ? 'text-neutral-300'
    : 'text-[#7a6b5a]';

  const tooltipLabelText = isDark
    ? 'text-neutral-400'
    : 'text-[#7a6b5a]';

  const tooltipValueText = isDark
    ? 'text-neutral-100'
    : 'text-[#2d2820]';

  return (
    <div className={`backdrop-blur-[40px] rounded-[24px] border p-8 relative overflow-hidden group/chart transition-colors ${theme === 'dark'
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/20'
      }`}>
      {/* Background Glow */}
      <div className="absolute top-0 right-0 w-80 h-80 bg-gradient-to-bl from-[#c9983a]/8 to-transparent rounded-full blur-3xl pointer-events-none group-hover/chart:scale-125 transition-transform duration-1000" />

      <div className="relative">
        <div className="mb-6">
          <h2 className={`text-[20px] font-bold mb-1 transition-colors ${theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
            }`}>Applications History</h2>
          <p className={`text-[12px] font-medium transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`}>Last 6 months overview</p>
        </div>

        {/* Bar Chart */}
        <div className={`h-[320px] backdrop-blur-[25px] rounded-[16px] border p-6 transition-colors ${theme === 'dark'
            ? 'bg-white/[0.05] border-white/10'
            : 'bg-white/[0.08] border-white/20'
          }`}>
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={data} barGap={4}>
              <CartesianGrid
                strokeDasharray="3 3"
                stroke={theme === 'dark' ? 'rgba(184, 168, 152, 0.12)' : 'rgba(122, 107, 90, 0.15)'}
                vertical={false}
              />
              <XAxis
                dataKey="month"
                stroke={theme === 'dark' ? '#b8a898' : '#7a6b5a'}
                tick={{ fill: theme === 'dark' ? '#b8a898' : '#7a6b5a', fontSize: 12, fontWeight: 600 }}
                axisLine={{ stroke: theme === 'dark' ? 'rgba(184, 168, 152, 0.2)' : 'rgba(122, 107, 90, 0.2)' }}
              />
              <YAxis
                stroke={theme === 'dark' ? '#b8a898' : '#7a6b5a'}
                tick={{ fill: theme === 'dark' ? '#b8a898' : '#7a6b5a', fontSize: 12, fontWeight: 600 }}
                axisLine={{ stroke: theme === 'dark' ? 'rgba(184, 168, 152, 0.2)' : 'rgba(122, 107, 90, 0.2)' }}
              />
              <Tooltip
                cursor={{ fill: 'rgba(201, 152, 58, 0.08)' }}
                content={({ active, payload }) => {
                  if (active && payload && payload.length) {
                    return (
                      <div
                        className={`backdrop-blur-[40px] rounded-[14px] border px-5 py-4 ${tooltipBg}`}
                      >
                        <div
                          className={`text-[13px] font-bold mb-2 ${tooltipTitleText}`}
                        >
                          {payload[0].payload.month}
                        </div>

                        {payload.map((entry: any, index: number) => (
                          <div
                            key={index}
                            className="flex items-center justify-between gap-4 mb-1"
                          >
                            <div className="flex items-center gap-2">
                              <div
                                className="w-3 h-3 rounded-full"
                                style={{ backgroundColor: entry.color }}
                              />
                              <span
                                className={`text-[12px] font-medium ${tooltipLabelText}`}
                              >
                                {entry.dataKey === 'applications'
                                  ? 'Applications'
                                  : 'Merged'}
                              </span>
                            </div>

                            <span
                              className={`text-[14px] font-bold ${tooltipValueText}`}
                            >
                              {entry.value}
                            </span>
                          </div>
                        ))}
                      </div>
                    );
                  }

                  return null;
                }}
              />

              <Bar
                dataKey="applications"
                fill="url(#applicationsGradient)"
                radius={[8, 8, 0, 0]}
                animationBegin={0}
                animationDuration={800}
              />
              <Bar
                dataKey="merged"
                fill="url(#mergedGradient)"
                radius={[8, 8, 0, 0]}
                animationBegin={100}
                animationDuration={800}
              />
              <defs>
                <linearGradient id="applicationsGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#c9983a" stopOpacity={0.9} />
                  <stop offset="100%" stopColor="#d4af37" stopOpacity={0.6} />
                </linearGradient>
                <linearGradient id="mergedGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#4fb37a" stopOpacity={0.9} />
                  <stop offset="100%" stopColor="#2e6947" stopOpacity={0.6} />
                </linearGradient>
              </defs>
            </BarChart>
          </ResponsiveContainer>
        </div>

        {/* Legend */}
        <div className="flex items-center justify-center gap-6 mt-5">
          <div className="flex items-center gap-2">
            <div className="w-4 h-4 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37]" />
            <span className={`text-[13px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>Applications</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-4 h-4 rounded-full bg-gradient-to-br from-[#4fb37a] to-[#2e6947]" />
            <span className={`text-[13px] font-semibold transition-colors ${theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
              }`}>Merged</span>
          </div>
        </div>
      </div>
    </div>
  );
}