import { useId } from "react";
import { Sparkles, Users, ArrowUpRight, Package } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";

export type RecommendationCardVariant = "project-pick" | "contributor-pick";

interface RecommendationCardProps {
  title: string;
  description: string;
  rationale: string;
  eyebrow: string;
  variant: RecommendationCardVariant;
  icon?: string;
  accentClass?: string;
  tags?: string[];
  stats?: Array<{ label: string; value: string }>;
  onClick?: () => void;
}

export function RecommendationCard({
  title,
  description,
  rationale,
  eyebrow,
  variant,
  icon,
  accentClass = "from-[#c9983a] to-[#a67c2e]",
  tags = [],
  stats = [],
  onClick,
}: RecommendationCardProps) {
  const { theme } = useTheme();
  const rationaleId = useId();
  const isDark = theme === "dark";
  const variantLabel = variant === "project-pick" ? "recommended project" : "recommended contributor";
  const isInteractive = Boolean(onClick);

  const rootClasses = [
    "group relative w-full rounded-[22px] border p-5 text-left transition-all duration-200",
    isInteractive ? "cursor-pointer" : "cursor-default",
    isDark
      ? "bg-white/[0.08] border-white/12 shadow-[0_10px_30px_rgba(0,0,0,0.14)]"
      : "bg-white/[0.16] border-white/25 shadow-[0_10px_30px_rgba(15,23,42,0.08)]",
    variant === "project-pick"
      ? "hover:-translate-y-1 hover:border-[#c9983a]/40 hover:shadow-[0_16px_38px_rgba(201,152,58,0.18)]"
      : "hover:-translate-y-1 hover:border-[#e8c77f]/40 hover:shadow-[0_16px_38px_rgba(201,152,58,0.16)]",
    "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]",
  ].join(" ");

  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={`${variantLabel}: ${title}`}
      aria-describedby={rationaleId}
      className={`${rootClasses} focus-visible:ring-2 focus-visible:ring-[#c9983a]/25`}
    >
      <div
        className={`absolute inset-0 rounded-[22px] bg-gradient-to-br ${accentClass} opacity-0 transition-opacity duration-200 group-hover:opacity-[0.07] group-focus-visible:opacity-[0.1]`}
      />

      <div className="relative flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div
            className={`flex h-11 w-11 shrink-0 items-center justify-center rounded-[14px] border border-white/20 bg-gradient-to-br ${accentClass} text-white shadow-sm`}
          >
            {icon ? (
              icon.startsWith("http") ? (
                <img
                  src={icon}
                  alt=""
                  className="h-8 w-8 rounded-[10px] object-cover"
                />
              ) : (
                icon
              )
            ) : variant === "project-pick" ? (
              <Package className="h-5 w-5" />
            ) : (
              <Users className="h-5 w-5" />
            )}
          </div>
          <div>
            <p
              className={`text-[11px] font-semibold uppercase tracking-[0.24em] transition-colors ${
                isDark ? "text-[#e8c77f]" : "text-[#8b6527]"
              }`}
            >
              {eyebrow}
            </p>
            <h4
              className={`mt-1 text-[17px] font-semibold leading-6 transition-colors ${
                isDark ? "text-[#f5f5f5]" : "text-[#2d2820]"
              }`}
            >
              {title}
            </h4>
          </div>
        </div>
        <div className={`rounded-full p-2 ${isDark ? "bg-white/10" : "bg-white/20"}`}>
          <ArrowUpRight className="h-4 w-4 text-[#c9983a]" />
        </div>
      </div>

      <div className="relative mt-4 flex items-center gap-2 rounded-full border border-[#c9983a]/20 bg-[#c9983a]/10 px-3 py-1.5 text-[12px] font-semibold text-[#8b6527] backdrop-blur-sm dark:text-[#f5c563]">
        <Sparkles className="h-3.5 w-3.5" />
        <span className="truncate" id={rationaleId} title={rationale}>
          Why recommended: {rationale}
        </span>
      </div>

      <p
        className={`relative mt-3 text-[13px] leading-6 transition-colors ${
          isDark ? "text-[#d4d4d4]" : "text-[#7a6b5a]"
        }`}
      >
        {description}
      </p>

      {stats.length > 0 && (
        <div className="relative mt-4 flex flex-wrap gap-2">
          {stats.map((stat) => (
            <div
              key={stat.label}
              className={`rounded-full border px-3 py-1.5 text-[12px] ${
                isDark
                  ? "border-white/10 bg-white/10 text-[#f5f5f5]"
                  : "border-white/20 bg-white/30 text-[#2d2820]"
              }`}
            >
              <span className="font-semibold">{stat.value}</span>{" "}
              <span className="opacity-80">{stat.label}</span>
            </div>
          ))}
        </div>
      )}

      {tags.length > 0 && (
        <div className="relative mt-4 flex flex-wrap gap-2">
          {tags.map((tag) => (
            <span
              key={tag}
              className={`rounded-full border px-2.5 py-1 text-[11px] font-semibold ${
                isDark
                  ? "border-[#c9983a]/30 bg-[#c9983a]/15 text-[#f5c563]"
                  : "border-[#c9983a]/30 bg-[#c9983a]/15 text-[#8b6527]"
              }`}
            >
              {tag}
            </span>
          ))}
        </div>
      )}
    </button>
  );
}
