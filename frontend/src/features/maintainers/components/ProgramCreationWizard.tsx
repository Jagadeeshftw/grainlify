import { useState, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { X, ChevronRight, ChevronLeft, Check, Loader2, AlertCircle, BookOpen, Coins, Calendar, Eye, Plus, Trash2 } from "lucide-react";
import * as Slider from "@radix-ui/react-slider";
import * as Switch from "@radix-ui/react-switch";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { getEcosystems } from "../../../shared/api/client";

// ─── Types ────────────────────────────────────────────────────────────────────

export type RecipeType = "hackathon" | "bounty" | "grant" | "ongoing";

export interface PayoutBracket {
  label: string; // e.g. "1st Place"
  percentage: number; // 0–100, sum across brackets must equal 100
}

export interface ScheduleEntry {
  milestone: string;
  releasePercentage: number; // 0–100
  unlockAfterDays: number;
}

export interface WizardFormState {
  // Step 1
  recipe: RecipeType | null;
  programName: string;
  ecosystemName: string;
  description: string;
  // Step 2
  fundingAmount: string;
  fundingToken: string;
  minBounty: string;
  maxBounty: string;
  // Step 3
  brackets: PayoutBracket[];
  scheduleEntries: ScheduleEntry[];
  useSchedule: boolean;
  // Step 4 — review only (no extra state)
}

const RECIPE_OPTIONS: { id: RecipeType; label: string; description: string; icon: string }[] = [
  { id: "hackathon", label: "Hackathon", description: "Time-boxed event with ranked prizes", icon: "🏆" },
  { id: "bounty", label: "Bounty", description: "Discrete tasks with fixed rewards", icon: "🎯" },
  { id: "grant", label: "Grant", description: "Milestone-based long-form funding", icon: "🌱" },
  { id: "ongoing", label: "Ongoing", description: "Continuous contribution rewards", icon: "♾️" },
];

const TOKEN_OPTIONS = ["XLM", "USDC", "EURC"];

const STEP_LABELS = ["Recipe", "Funding", "Schedule", "Review"];

// ─── Sub-components ───────────────────────────────────────────────────────────

function StepIndicator({ current, total, isDark }: { current: number; total: number; isDark: boolean }) {
  return (
    <div className="flex items-center gap-0" role="tablist" aria-label="Wizard steps">
      {STEP_LABELS.map((label, i) => {
        const stepNum = i + 1;
        const isComplete = stepNum < current;
        const isActive = stepNum === current;
        return (
          <div key={label} className="flex items-center">
            <div
              role="tab"
              aria-selected={isActive}
              aria-label={`Step ${stepNum}: ${label}${isComplete ? " (complete)" : ""}`}
              className={`flex items-center gap-2 px-3 py-1.5 rounded-full text-[12px] font-semibold transition-all ${
                isActive
                  ? isDark
                    ? "bg-[#c9983a]/30 border border-[#c9983a]/60 text-[#e8c77f]"
                    : "bg-[#c9983a]/20 border border-[#c9983a]/50 text-[#8b6f3a]"
                  : isComplete
                    ? isDark
                      ? "bg-green-500/20 border border-green-500/40 text-green-400"
                      : "bg-green-100 border border-green-300 text-green-700"
                    : isDark
                      ? "bg-white/[0.06] border border-white/10 text-[#b8a898]"
                      : "bg-white/20 border border-white/30 text-[#7a6b5a]"
              }`}
            >
              <span
                className={`w-4 h-4 rounded-full flex items-center justify-center text-[10px] font-bold flex-shrink-0 ${
                  isActive ? "bg-[#c9983a] text-white" : isComplete ? "bg-green-500 text-white" : isDark ? "bg-white/15 text-[#b8a898]" : "bg-white/40 text-[#7a6b5a]"
                }`}
              >
                {isComplete ? <Check className="w-2.5 h-2.5" /> : stepNum}
              </span>
              <span className="hidden sm:inline">{label}</span>
            </div>
            {i < total - 1 && <div className={`w-6 h-[1px] mx-0.5 ${isComplete ? "bg-green-500/50" : isDark ? "bg-white/10" : "bg-white/25"}`} aria-hidden />}
          </div>
        );
      })}
    </div>
  );
}

function SectionLabel({ children, isDark }: { children: React.ReactNode; isDark: boolean }) {
  return <p className={`text-[11px] font-bold uppercase tracking-widest mb-3 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>{children}</p>;
}

function InputField({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
  required,
  hint,
  isDark,
  disabled,
  error,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  required?: boolean;
  hint?: string;
  isDark: boolean;
  disabled?: boolean;
  error?: string | null;
}) {
  const id = label.toLowerCase().replace(/\s+/g, "-");
  return (
    <div>
      <label htmlFor={id} className={`block text-[13px] font-semibold mb-1.5 ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
        {label}
        {required && (
          <span className="text-red-500 ml-1" aria-hidden>
            *
          </span>
        )}
      </label>
      <input
        id={id}
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        required={required}
        disabled={disabled}
        aria-required={required}
        aria-invalid={!!error}
        aria-describedby={error ? `${id}-error` : hint ? `${id}-hint` : undefined}
        className={`w-full px-4 py-2.5 rounded-[12px] border-2 text-[14px] transition-all
          ${
            error
              ? isDark
                ? "bg-red-500/10 border-red-500/50 text-[#e8dfd0] placeholder:text-red-400/50"
                : "bg-red-50 border-red-400 text-[#2d2820]"
              : isDark
                ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a]/50 focus:bg-white/[0.12]"
                : "bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a]/50 focus:bg-white/60"
          }
          ${disabled ? "opacity-50 cursor-not-allowed" : ""}
          focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30`}
      />
      {error && (
        <p id={`${id}-error`} role="alert" aria-live="polite" className={`text-[12px] mt-1 flex items-center gap-1 ${isDark ? "text-red-400" : "text-red-600"}`}>
          <AlertCircle className="w-3 h-3 flex-shrink-0" /> {error}
        </p>
      )}
      {hint && !error && (
        <p id={`${id}-hint`} className={`text-[12px] mt-1 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
          {hint}
        </p>
      )}
    </div>
  );
}

// ─── Step 1: Recipe ───────────────────────────────────────────────────────────

function Step1Recipe({
  form,
  setForm,
  isDark,
  ecosystems,
  loadingEco,
}: {
  form: WizardFormState;
  setForm: React.Dispatch<React.SetStateAction<WizardFormState>>;
  isDark: boolean;
  ecosystems: { name: string; slug: string }[];
  loadingEco: boolean;
}) {
  return (
    <div className="space-y-6">
      <div>
        <SectionLabel isDark={isDark}>Program type</SectionLabel>
        <div className="grid grid-cols-2 gap-3" role="radiogroup" aria-label="Program type">
          {RECIPE_OPTIONS.map((opt) => {
            const selected = form.recipe === opt.id;
            return (
              <button
                key={opt.id}
                type="button"
                role="radio"
                aria-checked={selected}
                onClick={() => setForm((f) => ({ ...f, recipe: opt.id }))}
                className={`p-4 rounded-[14px] border-2 text-left transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
                  selected
                    ? isDark
                      ? "bg-[#c9983a]/20 border-[#c9983a]/60 shadow-[0_0_20px_rgba(201,152,58,0.2)]"
                      : "bg-[#c9983a]/15 border-[#c9983a]/50"
                    : isDark
                      ? "bg-white/[0.06] border-white/10 hover:bg-white/[0.1] hover:border-white/20"
                      : "bg-white/20 border-white/30 hover:bg-white/30 hover:border-white/40"
                }`}
              >
                <div className="text-2xl mb-2" aria-hidden>
                  {opt.icon}
                </div>
                <div className={`text-[14px] font-bold mb-0.5 ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>{opt.label}</div>
                <div className={`text-[12px] leading-snug ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>{opt.description}</div>
                {selected && (
                  <div className="mt-2 flex items-center gap-1">
                    <Check className="w-3.5 h-3.5 text-[#c9983a]" />
                    <span className={`text-[11px] font-semibold ${isDark ? "text-[#e8c77f]" : "text-[#8b6f3a]"}`}>Selected</span>
                  </div>
                )}
              </button>
            );
          })}
        </div>
      </div>

      <div className="space-y-4">
        <SectionLabel isDark={isDark}>Program details</SectionLabel>
        <InputField
          label="Program name"
          required
          value={form.programName}
          onChange={(v) => setForm((f) => ({ ...f, programName: v }))}
          placeholder="e.g. Stellar Q1 OSS Hackathon"
          isDark={isDark}
        />

        <div>
          <label className={`block text-[13px] font-semibold mb-1.5 ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
            Ecosystem{" "}
            <span className="text-red-500" aria-hidden>
              *
            </span>
          </label>
          {loadingEco ? (
            <div className={`h-11 w-full rounded-[12px] border-2 animate-pulse ${isDark ? "bg-white/[0.08] border-white/15" : "bg-white/40 border-white/50"}`} />
          ) : (
            <EcosystemSelect value={form.ecosystemName} onChange={(v) => setForm((f) => ({ ...f, ecosystemName: v }))} ecosystems={ecosystems} isDark={isDark} />
          )}
        </div>

        <div>
          <label htmlFor="description" className={`block text-[13px] font-semibold mb-1.5 ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
            Description
          </label>
          <textarea
            id="description"
            value={form.description}
            onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
            placeholder="What is this program funding? Who should apply?"
            rows={3}
            className={`w-full px-4 py-2.5 rounded-[12px] border-2 text-[14px] resize-none transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30
              ${
                isDark
                  ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a]/50"
                  : "bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a]/50"
              }`}
          />
        </div>
      </div>
    </div>
  );
}

// ─── Step 2: Funding ──────────────────────────────────────────────────────────

function Step2Funding({ form, setForm, isDark }: { form: WizardFormState; setForm: React.Dispatch<React.SetStateAction<WizardFormState>>; isDark: boolean }) {
  return (
    <div className="space-y-6">
      <div className="space-y-4">
        <SectionLabel isDark={isDark}>Prize pool</SectionLabel>

        <div className="flex gap-3">
          <div className="flex-1">
            <InputField
              label="Total amount"
              required
              type="number"
              value={form.fundingAmount}
              onChange={(v) => setForm((f) => ({ ...f, fundingAmount: v }))}
              placeholder="50000"
              isDark={isDark}
              hint="Funds locked into on-chain escrow before work begins"
            />
          </div>
          <div className="w-28">
            <label htmlFor="token-select" className={`block text-[13px] font-semibold mb-1.5 ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
              Token
            </label>
            <select
              id="token-select"
              value={form.fundingToken}
              onChange={(e) => setForm((f) => ({ ...f, fundingToken: e.target.value }))}
              className={`w-full px-3 py-2.5 rounded-[12px] border-2 text-[14px] transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30
                ${
                  isDark
                    ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] focus:border-[#c9983a]/50"
                    : "bg-white/40 border-white/50 text-[#2d2820] focus:border-[#c9983a]/50"
                }`}
            >
              {TOKEN_OPTIONS.map((t) => (
                <option key={t} value={t}>
                  {t}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Escrow callout */}
        <div
          className={`flex items-start gap-3 p-3 rounded-[12px] border ${isDark ? "bg-[#c9983a]/10 border-[#c9983a]/25 text-[#e8c77f]" : "bg-[#c9983a]/10 border-[#c9983a]/25 text-[#8b6f3a]"}`}
        >
          <Coins className="w-4 h-4 mt-0.5 flex-shrink-0" aria-hidden />
          <p className="text-[12px] leading-relaxed">
            Funds are locked in the on-chain Soroban escrow before the program goes live. Contributors are paid directly — the platform never holds funds.
          </p>
        </div>
      </div>

      <div className="space-y-4">
        <SectionLabel isDark={isDark}>Per-bounty limits (optional)</SectionLabel>
        <div className="grid grid-cols-2 gap-3">
          <InputField
            label="Min bounty"
            type="number"
            value={form.minBounty}
            onChange={(v) => setForm((f) => ({ ...f, minBounty: v }))}
            placeholder="100"
            isDark={isDark}
            hint={`${form.fundingToken}`}
          />
          <InputField
            label="Max bounty"
            type="number"
            value={form.maxBounty}
            onChange={(v) => setForm((f) => ({ ...f, maxBounty: v }))}
            placeholder="5000"
            isDark={isDark}
            hint={`${form.fundingToken}`}
          />
        </div>
      </div>

      {/* Live preview bar */}
      {form.fundingAmount && Number(form.fundingAmount) > 0 && (
        <div className={`p-4 rounded-[14px] border-2 space-y-2 ${isDark ? "bg-white/[0.04] border-white/10" : "bg-white/30 border-white/40"}`}>
          <p className={`text-[12px] font-semibold ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>Prize pool preview</p>
          <p className={`text-[28px] font-bold tabular-nums ${isDark ? "text-[#e8c77f]" : "text-[#8b6f3a]"}`}>
            {Number(form.fundingAmount).toLocaleString()} <span className="text-[16px] font-normal opacity-70">{form.fundingToken}</span>
          </p>
        </div>
      )}
    </div>
  );
}

// ─── Step 3: Schedule / Brackets ─────────────────────────────────────────────

function Step3Schedule({ form, setForm, isDark }: { form: WizardFormState; setForm: React.Dispatch<React.SetStateAction<WizardFormState>>; isDark: boolean }) {
  const bracketTotal = form.brackets.reduce((s, b) => s + b.percentage, 0);
  const bracketError = bracketTotal !== 100 && form.brackets.length > 0;

  const addBracket = () => {
    setForm((f) => ({
      ...f,
      brackets: [...f.brackets, { label: `Place ${f.brackets.length + 1}`, percentage: 0 }],
    }));
  };

  const removeBracket = (i: number) => {
    setForm((f) => ({ ...f, brackets: f.brackets.filter((_, idx) => idx !== i) }));
  };

  const addSchedule = () => {
    setForm((f) => ({
      ...f,
      scheduleEntries: [...f.scheduleEntries, { milestone: "", releasePercentage: 25, unlockAfterDays: 30 }],
    }));
  };

  const removeSchedule = (i: number) => {
    setForm((f) => ({ ...f, scheduleEntries: f.scheduleEntries.filter((_, idx) => idx !== i) }));
  };

  const inputCls = `w-full px-3 py-2 rounded-[10px] border-2 text-[13px] transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30 ${
    isDark
      ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] placeholder:text-[#b8a898] focus:border-[#c9983a]/50"
      : "bg-white/40 border-white/50 text-[#2d2820] placeholder:text-[#7a6b5a] focus:border-[#c9983a]/50"
  }`;

  return (
    <div className="space-y-6">
      {/* Payout brackets */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <SectionLabel isDark={isDark}>Payout brackets</SectionLabel>
          <button
            type="button"
            onClick={addBracket}
            className={`flex items-center gap-1 text-[12px] font-semibold px-3 py-1.5 rounded-[8px] border transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30 ${
              isDark
                ? "bg-[#c9983a]/15 border-[#c9983a]/40 text-[#e8c77f] hover:bg-[#c9983a]/25"
                : "bg-[#c9983a]/15 border-[#c9983a]/40 text-[#8b6f3a] hover:bg-[#c9983a]/25"
            }`}
            aria-label="Add payout bracket"
          >
            <Plus className="w-3.5 h-3.5" /> Add bracket
          </button>
        </div>

        {form.brackets.length === 0 && (
          <p className={`text-[13px] py-4 text-center ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
            No brackets yet. Add one above, or leave empty for flat payouts.
          </p>
        )}

        <div className="space-y-3">
          {form.brackets.map((b, i) => (
            <div
              key={i}
              className={`flex items-center sm:gap-3 gap-2 sm:p-3 p-2 rounded-[12px] border ${isDark ? "bg-white/[0.04] border-white/10" : "bg-white/20 border-white/30"}`}
            >
              <div className="flex-1">
                <label htmlFor={`bracket-label-${i}`} className="sr-only">
                  Bracket label
                </label>
                <input
                  id={`bracket-label-${i}`}
                  value={b.label}
                  onChange={(e) => {
                    const next = [...form.brackets];
                    next[i] = { ...next[i], label: e.target.value };
                    setForm((f) => ({ ...f, brackets: next }));
                  }}
                  placeholder="e.g. 1st Place"
                  className={inputCls}
                />
              </div>
              <div className="sm:w-28 w-24 flex-shrink-0">
                <label htmlFor={`bracket-pct-${i}`} className="sr-only">
                  Percentage
                </label>
                <div className="relative">
                  <input
                    id={`bracket-pct-${i}`}
                    type="number"
                    min={0}
                    max={100}
                    value={b.percentage === 0 ? "" : b.percentage}
                    onChange={(e) => {
                      const next = [...form.brackets];
                      next[i] = { ...next[i], percentage: e.target.value === "" ? 0 : Number(e.target.value) };
                      setForm((f) => ({ ...f, brackets: next }));
                    }}
                    placeholder="100"
                    className={inputCls + " pr-8"}
                  />
                  <span className={`absolute right-3 top-1/2 -translate-y-1/2 text-[12px] ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>%</span>
                </div>
              </div>
              {/* Slider */}
              <div className="sm:w-20 w-16 flex-shrink-0" aria-hidden>
                <Slider.Root
                  min={0}
                  max={100}
                  step={1}
                  value={[b.percentage]}
                  onValueChange={([val]) => {
                    const next = [...form.brackets];
                    next[i] = { ...next[i], percentage: val };
                    setForm((f) => ({ ...f, brackets: next }));
                  }}
                  className="relative flex items-center h-5 w-full cursor-pointer touch-none"
                >
                  <Slider.Track className={`relative h-1.5 flex-1 rounded-full overflow-hidden ${isDark ? "bg-white/10" : "bg-white/40"}`}>
                    <Slider.Range className="absolute h-full bg-[#c9983a] rounded-full" />
                  </Slider.Track>
                  <Slider.Thumb className="block w-4 h-4 rounded-full bg-[#c9983a] border-2 border-white shadow focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40" />
                </Slider.Root>
              </div>
              <button
                type="button"
                onClick={() => removeBracket(i)}
                aria-label={`Remove bracket ${b.label}`}
                className={`p-1.5 rounded-[8px] transition-all focus:outline-none focus:ring-2 focus:ring-red-400/40 ${isDark ? "hover:bg-red-500/20 text-[#b8a898] hover:text-red-400" : "hover:bg-red-100 text-[#7a6b5a] hover:text-red-600"}`}
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          ))}
        </div>

        {/* Bracket total */}
        {form.brackets.length > 0 && (
          <div
            className={`mt-2 flex items-center gap-2 text-[12px] font-semibold ${bracketError ? "text-red-500" : isDark ? "text-green-400" : "text-green-600"}`}
            role={bracketError ? "alert" : "status"}
            aria-live="polite"
          >
            {bracketError ? (
              <>
                <AlertCircle className="w-3.5 h-3.5" /> Brackets total {bracketTotal}% — must equal 100%
              </>
            ) : (
              <>
                <Check className="w-3.5 h-3.5" /> Brackets total 100%
              </>
            )}
          </div>
        )}
      </div>

      {/* Release schedule */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-3">
            <SectionLabel isDark={isDark}>Release schedule</SectionLabel>
            <Switch.Root
              checked={form.useSchedule}
              onCheckedChange={(v) => setForm((f) => ({ ...f, useSchedule: v }))}
              aria-label="Enable release schedule"
              className={`relative w-9 h-5 rounded-full outline-none transition-colors focus:ring-2 focus:ring-[#c9983a]/40 ${form.useSchedule ? "bg-[#c9983a]" : isDark ? "bg-white/15" : "bg-white/40"}`}
            >
              <Switch.Thumb className="block w-3.5 h-3.5 bg-white rounded-full shadow transition-transform data-[state=checked]:translate-x-[18px] translate-x-0.5" />
            </Switch.Root>
          </div>
          {form.useSchedule && (
            <button
              type="button"
              onClick={addSchedule}
              className={`flex items-center gap-1 text-[12px] font-semibold px-3 py-1.5 rounded-[8px] border transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30 ${
                isDark
                  ? "bg-[#c9983a]/15 border-[#c9983a]/40 text-[#e8c77f] hover:bg-[#c9983a]/25"
                  : "bg-[#c9983a]/15 border-[#c9983a]/40 text-[#8b6f3a] hover:bg-[#c9983a]/25"
              }`}
              aria-label="Add schedule entry"
            >
              <Plus className="w-3.5 h-3.5" /> Add milestone
            </button>
          )}
        </div>

        {form.useSchedule && (
          <div className="space-y-3">
            {form.scheduleEntries.length === 0 && (
              <p className={`text-[13px] py-4 text-center ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                Add milestones to lock portions of the prize pool to specific unlock times.
              </p>
            )}
            {form.scheduleEntries.map((s, i) => (
              <div key={i} className={`p-3 rounded-[12px] border space-y-2 ${isDark ? "bg-white/[0.04] border-white/10" : "bg-white/20 border-white/30"}`}>
                <div className="flex items-center gap-2">
                  <div className="flex-1">
                    <label htmlFor={`sched-label-${i}`} className="sr-only">
                      Milestone name
                    </label>
                    <input
                      id={`sched-label-${i}`}
                      value={s.milestone}
                      onChange={(e) => {
                        const next = [...form.scheduleEntries];
                        next[i] = { ...next[i], milestone: e.target.value };
                        setForm((f) => ({ ...f, scheduleEntries: next }));
                      }}
                      placeholder="e.g. Final submissions"
                      className={inputCls}
                    />
                  </div>
                  <button
                    type="button"
                    onClick={() => removeSchedule(i)}
                    aria-label={`Remove milestone ${s.milestone || i + 1}`}
                    className={`p-1.5 rounded-[8px] transition-all focus:outline-none focus:ring-2 focus:ring-red-400/40 ${isDark ? "hover:bg-red-500/20 text-[#b8a898] hover:text-red-400" : "hover:bg-red-100 text-[#7a6b5a] hover:text-red-600"}`}
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <label htmlFor={`sched-pct-${i}`} className={`block text-[11px] font-semibold mb-1 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                      Release %
                    </label>
                    <div className="relative">
                      <input
                        id={`sched-pct-${i}`}
                        type="number"
                        min={0}
                        max={100}
                        value={s.releasePercentage === 0 ? "" : s.releasePercentage}
                        onChange={(e) => {
                          const next = [...form.scheduleEntries];
                          next[i] = { ...next[i], releasePercentage: e.target.value === "" ? 0 : Number(e.target.value) };
                          setForm((f) => ({ ...f, scheduleEntries: next }));
                        }}
                        className={inputCls + " pr-6"}
                      />
                      <span className={`absolute right-2.5 top-1/2 -translate-y-1/2 text-[11px] ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>%</span>
                    </div>
                  </div>
                  <div>
                    <label htmlFor={`sched-days-${i}`} className={`block text-[11px] font-semibold mb-1 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                      Unlock after (days)
                    </label>
                    <input
                      id={`sched-days-${i}`}
                      type="number"
                      min={0}
                      value={s.unlockAfterDays === 0 ? "" : s.unlockAfterDays}
                      onChange={(e) => {
                        const next = [...form.scheduleEntries];
                        next[i] = { ...next[i], unlockAfterDays: e.target.value === "" ? 0 : Number(e.target.value) };
                        setForm((f) => ({ ...f, scheduleEntries: next }));
                      }}
                      className={inputCls}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Step 4: Review ───────────────────────────────────────────────────────────

function Step4Review({ form, isDark, onEditStep }: { form: WizardFormState; isDark: boolean; onEditStep: (step: number) => void }) {
  const recipe = RECIPE_OPTIONS.find((r) => r.id === form.recipe);

  const Row = ({ label, value }: { label: string; value: string }) => (
    <div className="flex items-start justify-between gap-3 py-2">
      <span className={`text-[12px] font-semibold flex-shrink-0 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>{label}</span>
      <span className={`text-[13px] text-right ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>{value || "—"}</span>
    </div>
  );

  const Section = ({ title, step, children }: { title: string; step: number; children: React.ReactNode }) => (
    <div className={`rounded-[14px] border-2 overflow-hidden ${isDark ? "bg-white/[0.04] border-white/10" : "bg-white/20 border-white/30"}`}>
      <div className={`flex items-center justify-between px-4 py-2.5 border-b ${isDark ? "border-white/10 bg-white/[0.03]" : "border-white/30 bg-white/20"}`}>
        <span className={`text-[12px] font-bold uppercase tracking-wider ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>{title}</span>
        <button
          type="button"
          onClick={() => onEditStep(step)}
          className={`text-[11px] font-semibold px-2 py-1 rounded-[6px] transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30 ${isDark ? "text-[#e8c77f] hover:bg-[#c9983a]/20" : "text-[#8b6f3a] hover:bg-[#c9983a]/15"}`}
        >
          Edit
        </button>
      </div>
      <div className="px-4 divide-y divide-white/5">{children}</div>
    </div>
  );

  return (
    <div className="space-y-4">
      <Section title="Recipe & details" step={1}>
        <Row label="Type" value={recipe ? `${recipe.icon} ${recipe.label}` : "—"} />
        <Row label="Name" value={form.programName} />
        <Row label="Ecosystem" value={form.ecosystemName} />
        {form.description && <Row label="Description" value={form.description} />}
      </Section>

      <Section title="Funding" step={2}>
        <Row label="Prize pool" value={form.fundingAmount ? `${Number(form.fundingAmount).toLocaleString()} ${form.fundingToken}` : "—"} />
        {form.minBounty && <Row label="Min bounty" value={`${form.minBounty} ${form.fundingToken}`} />}
        {form.maxBounty && <Row label="Max bounty" value={`${form.maxBounty} ${form.fundingToken}`} />}
      </Section>

      <Section title="Brackets & schedule" step={3}>
        {form.brackets.length === 0 && !form.useSchedule ? <Row label="Payout" value="Flat / no brackets configured" /> : null}
        {form.brackets.map((b, i) => (
          <Row key={i} label={b.label} value={`${b.percentage}%`} />
        ))}
        {form.useSchedule &&
          form.scheduleEntries.length > 0 &&
          form.scheduleEntries.map((s, i) => (
            <Row key={`s-${i}`} label={s.milestone || `Milestone ${i + 1}`} value={`${s.releasePercentage}% after ${s.unlockAfterDays}d`} />
          ))}
      </Section>

      {/* Publish notice */}
      <div
        className={`flex items-start gap-3 p-3 rounded-[12px] border ${isDark ? "bg-green-500/10 border-green-500/25 text-green-400" : "bg-green-50 border-green-300 text-green-700"}`}
      >
        <Eye className="w-4 h-4 mt-0.5 flex-shrink-0" aria-hidden />
        <p className="text-[12px] leading-relaxed">
          Publishing will create the program on-chain and lock your prize pool into escrow. Contributors won't be paid until you trigger payouts.
        </p>
      </div>
    </div>
  );
}

function EcosystemSelect({
  value,
  onChange,
  ecosystems,
  isDark,
}: {
  value: string;
  onChange: (v: string) => void;
  ecosystems: { name: string; slug: string }[];
  isDark: boolean;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const id = setTimeout(() => document.addEventListener("click", handler), 0);
    return () => {
      clearTimeout(id);
      document.removeEventListener("click", handler);
    };
  }, [open]);

  const selected = ecosystems.find((e) => e.name === value);

  return (
    <div ref={ref} className="relative">
      {/* Trigger */}
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
        className={`w-full px-4 py-2.5 rounded-[12px] border-2 text-[14px] text-left flex items-center justify-between transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30
          ${isDark ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] focus:border-[#c9983a]/50" : "bg-white/40 border-white/50 text-[#2d2820] focus:border-[#c9983a]/50"}
          ${open ? (isDark ? "border-[#c9983a]/50" : "border-[#c9983a]/50") : ""}
        `}
      >
        <span className={selected ? "" : isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}>{selected ? selected.name : "Select ecosystem…"}</span>
        <ChevronRight className={`w-4 h-4 flex-shrink-0 transition-transform ${open ? "rotate-90" : ""} ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`} />
      </button>

      {/* Dropdown */}
      {open && (
        <div
          role="listbox"
          aria-label="Ecosystem"
          className={`absolute z-50 mt-1.5 w-full rounded-[14px] border-2 overflow-hidden shadow-[0_8px_32px_rgba(0,0,0,0.35)] backdrop-blur-md
            ${isDark ? "bg-[#3a3228]/95 border-white/20" : "bg-[#d4c5b0]/95 border-white/40"}
          `}
        >
          <div className="max-h-48 overflow-y-auto custom-scrollbar py-1.5">
            {ecosystems.map((eco) => {
              const isSelected = eco.name === value;
              return (
                <button
                  key={eco.slug}
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  onClick={() => {
                    onChange(eco.name);
                    setOpen(false);
                  }}
                  className={`w-full px-4 py-2.5 text-left text-[14px] flex items-center justify-between transition-all
                    ${
                      isSelected
                        ? isDark
                          ? "bg-[#c9983a]/25 text-[#e8c77f]"
                          : "bg-[#c9983a]/20 text-[#8b6f3a]"
                        : isDark
                          ? "text-[#e8dfd0] hover:bg-white/[0.08]"
                          : "text-[#2d2820] hover:bg-white/30"
                    }
                  `}
                >
                  <span>{eco.name}</span>
                  {isSelected && <Check className="w-3.5 h-3.5 text-[#c9983a]" />}
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Wizard Shell ─────────────────────────────────────────────────────────────

export interface ProgramCreationWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

const EMPTY_FORM: WizardFormState = {
  recipe: null,
  programName: "",
  ecosystemName: "",
  description: "",
  fundingAmount: "",
  fundingToken: "XLM",
  minBounty: "",
  maxBounty: "",
  brackets: [],
  scheduleEntries: [],
  useSchedule: false,
};

export function ProgramCreationWizard({ isOpen, onClose, onSuccess }: ProgramCreationWizardProps) {
  const { theme } = useTheme();
  const isDark = theme === "dark";

  const [step, setStep] = useState(1);
  const [form, setForm] = useState<WizardFormState>(EMPTY_FORM);
  const [ecosystems, setEcosystems] = useState<{ name: string; slug: string }[]>([]);
  const [loadingEco, setLoadingEco] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [stepError, setStepError] = useState<string | null>(null);
  const [submitted, setSubmitted] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const lastFocusRef = useRef<HTMLElement | null>(null);
  const errorRef = useRef<HTMLDivElement>(null);

  // ── DEV MOCKS ── auto-disabled in production builds
  const IS_DEV = import.meta.env.DEV;

  const MOCK_ECOSYSTEMS = [
    { name: "Stellar", slug: "stellar" },
    { name: "Soroban", slug: "soroban" },
    { name: "Ethereum", slug: "ethereum" },
    { name: "Polkadot", slug: "polkadot" },
    { name: "Cosmos", slug: "cosmos" },
  ];

  const mockPublish = (): Promise<void> => new Promise((res) => setTimeout(res, 1000));

  // Focus trap
  useEffect(() => {
    if (!isOpen) return;
    lastFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => {
      lastFocusRef.current?.focus();
    };
  }, [isOpen]);

  // Reset on close
  useEffect(() => {
    if (!isOpen) {
      setStep(1);
      setForm(EMPTY_FORM);
      setStepError(null);
      setSubmitted(false);
    }
  }, [isOpen]);

  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [isOpen]);

  // Load ecosystems
  useEffect(() => {
    if (!isOpen) return;
    setLoadingEco(true);

    if (IS_DEV) {
      setTimeout(() => {
        setEcosystems(MOCK_ECOSYSTEMS);
        setLoadingEco(false);
      }, 500);
      return;
    }

    getEcosystems()
      .then((d) => setEcosystems(d.ecosystems.map((e) => ({ name: e.name, slug: e.slug }))))
      .catch(() => setEcosystems([]))
      .finally(() => setLoadingEco(false));
  }, [isOpen]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      handleClose();
    }
  };

  const handleClose = () => {
    if (isSubmitting) return;
    onClose();
  };

  // Per-step validation
  const validate = (): string | null => {
    if (step === 1) {
      if (!form.recipe) return "Select a program type to continue.";
      if (!form.programName.trim()) return "Program name is required.";
      if (!form.ecosystemName) return "Ecosystem is required.";
    }
    if (step === 2) {
      if (!form.fundingAmount || Number(form.fundingAmount) <= 0) return "Enter a valid funding amount.";
      if (form.minBounty && form.maxBounty && Number(form.minBounty) > Number(form.maxBounty)) return "Min bounty cannot exceed max bounty.";
    }
    if (step === 3) {
      const bracketTotal = form.brackets.reduce((s, b) => s + b.percentage, 0);
      if (form.brackets.length > 0 && bracketTotal !== 100) return `Bracket percentages must total 100% (currently ${bracketTotal}%).`;
    }
    return null;
  };

  const handleNext = () => {
    const err = validate();
    if (err) {
      setStepError(err);
      setTimeout(() => errorRef.current?.scrollIntoView({ behavior: "smooth", block: "start" }), 0);

      // Scroll the modal body to top so the error banner is visible
      containerRef.current?.querySelector(".overflow-y-auto")?.scrollTo({ top: 0, behavior: "smooth" });
      return;
    }
    setStepError(null);
    setStep((s) => s + 1);
  };

  const handleBack = () => {
    setStepError(null);
    setStep((s) => s - 1);
  };

  const handlePublish = async () => {
    setIsSubmitting(true);
    setStepError(null);
    try {
      if (IS_DEV) {
        await mockPublish();
      } else {
        // await createProgram(form); // real API call goes here
        await new Promise((r) => setTimeout(r, 1200));
      }
      setSubmitted(true);
      setTimeout(() => {
        onSuccess();
        onClose();
      }, 1000);
    } catch (e) {
      setStepError(e instanceof Error ? e.message : "Failed to create program. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  };

  const STEP_ICONS = [BookOpen, Coins, Calendar, Eye];
  const StepIcon = STEP_ICONS[step - 1];

  const stepTitles = ["Choose recipe & details", "Configure funding", "Set payout schedule", "Review & publish"];

  if (!isOpen) return null;

  return createPortal(
    <div className="fixed inset-0 z-[10000] flex items-center justify-center p-4" role="presentation" onKeyDown={handleKeyDown}>
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/55 backdrop-blur-sm" onClick={handleClose} aria-hidden />

      {/* Modal */}
      <div
        ref={containerRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="wizard-title"
        className={`relative w-full max-w-[560px] max-h-[90vh] flex flex-col rounded-[24px] border-2 shadow-[0_24px_80px_rgba(0,0,0,0.5)] transition-colors ${
          isDark ? "bg-[#3a3228] border-white/25" : "bg-[#d4c5b0] border-white/40"
        }`}
      >
        {/* Header */}
        <div className={`flex-shrink-0 sm:px-6 px-3 pt-5 pb-4 border-b-2 ${isDark ? "border-white/15" : "border-white/30"}`}>
          <div className="flex items-start justify-between gap-4 mb-4">
            <div className="flex items-center gap-3">
              <div
                className={`w-9 h-9 rounded-[12px] flex items-center justify-center border-2 flex-shrink-0 ${isDark ? "bg-[#c9983a]/25 border-[#c9983a]/50" : "bg-[#c9983a]/20 border-[#c9983a]/40"}`}
              >
                <StepIcon className="w-4 h-4 text-[#c9983a]" aria-hidden />
              </div>
              <div>
                <h2 id="wizard-title" className={`text-[18px] font-bold leading-tight ${isDark ? "text-[#e8dfd0]" : "text-[#2d2820]"}`}>
                  {stepTitles[step - 1]}
                </h2>
                <p className={`text-[12px] mt-0.5 ${isDark ? "text-[#b8a898]" : "text-[#7a6b5a]"}`}>
                  New program · Step {step} of {STEP_LABELS.length}
                </p>
              </div>
            </div>
            <button
              type="button"
              onClick={handleClose}
              disabled={isSubmitting}
              aria-label="Close wizard"
              className={`p-2 rounded-[10px] transition-all flex-shrink-0 focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
                isDark ? "hover:bg-white/10 text-[#b8a898] hover:text-[#e8dfd0]" : "hover:bg-white/20 text-[#7a6b5a] hover:text-[#2d2820]"
              } ${isSubmitting ? "opacity-40 cursor-not-allowed" : ""}`}
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          <StepIndicator current={step} total={STEP_LABELS.length} isDark={isDark} />
        </div>

        {/* Scrollable body */}
        <div className="flex-1 overflow-y-auto sm:px-6 px-3 py-5 custom-scrollbar">
          {/* Step error */}
          {stepError && (
            <div
              ref={errorRef}
              role="alert"
              aria-live="assertive"
              className={`flex items-center gap-3 p-3 rounded-[12px] border-2 mb-5 ${isDark ? "bg-red-500/10 border-red-500/30 text-red-400" : "bg-red-50 border-red-300 text-red-700"}`}
            >
              <AlertCircle className="w-4 h-4 flex-shrink-0" aria-hidden />
              <span className="text-[13px] font-medium">{stepError}</span>
            </div>
          )}

          {/* Step success */}
          {submitted && (
            <div
              className={`flex items-center gap-3 p-3 rounded-[12px] border-2 mb-5 ${isDark ? "bg-green-500/10 border-green-500/30 text-green-400" : "bg-green-50 border-green-300 text-green-700"}`}
            >
              <Check className="w-4 h-4 flex-shrink-0" />
              <span className="text-[13px] font-medium">Program created! Redirecting…</span>
            </div>
          )}

          {step === 1 && <Step1Recipe form={form} setForm={setForm} isDark={isDark} ecosystems={ecosystems} loadingEco={loadingEco} />}
          {step === 2 && <Step2Funding form={form} setForm={setForm} isDark={isDark} />}
          {step === 3 && <Step3Schedule form={form} setForm={setForm} isDark={isDark} />}
          {step === 4 && (
            <Step4Review
              form={form}
              isDark={isDark}
              onEditStep={(s) => {
                setStepError(null);
                setStep(s);
              }}
            />
          )}
        </div>

        {/* Footer nav */}
        <div className={`flex-shrink-0 sm:px-6 px-3 py-4 border-t-2 flex items-center gap-3 ${isDark ? "border-white/15" : "border-white/30"}`}>
          {step > 1 ? (
            <button
              type="button"
              onClick={handleBack}
              disabled={isSubmitting}
              className={`flex items-center gap-2 px-5 py-2.5 rounded-[12px] border-2 text-[14px] font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30 ${
                isDark ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]" : "bg-white/30 border-white/40 text-[#2d2820] hover:bg-white/50"
              } ${isSubmitting ? "opacity-50 cursor-not-allowed" : ""}`}
            >
              <ChevronLeft className="w-4 h-4" aria-hidden /> Back
            </button>
          ) : (
            <button
              type="button"
              onClick={handleClose}
              disabled={isSubmitting}
              className={`flex items-center gap-2 px-5 py-2.5 rounded-[12px] border-2 text-[14px] font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/30 ${
                isDark ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12]" : "bg-white/30 border-white/40 text-[#2d2820] hover:bg-white/50"
              }`}
            >
              Cancel
            </button>
          )}

          <div className="flex-1" />

          {step < 4 ? (
            <button
              type="button"
              onClick={handleNext}
              className={`flex items-center gap-2 px-5 py-2.5 rounded-[12px] border-2 text-[14px] font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
                isDark
                  ? "bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-[#c9983a]/60 text-[#fef5e7] hover:from-[#c9983a]/50 hover:to-[#d4af37]/40 shadow-[0_4px_16px_rgba(201,152,58,0.35)]"
                  : "bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border-[#c9983a]/50 text-[#2d2820] hover:from-[#c9983a]/40 hover:to-[#d4af37]/35 shadow-[0_4px_16px_rgba(201,152,58,0.2)]"
              }`}
            >
              Next <ChevronRight className="w-4 h-4" aria-hidden />
            </button>
          ) : (
            <button
              type="button"
              onClick={handlePublish}
              disabled={isSubmitting || submitted}
              className={`flex items-center gap-2 px-5 py-2.5 rounded-[12px] border-2 text-[14px] font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-[#c9983a]/40 ${
                isDark
                  ? "bg-gradient-to-br from-[#c9983a]/40 to-[#d4af37]/30 border-[#c9983a]/60 text-[#fef5e7] hover:from-[#c9983a]/50 shadow-[0_4px_16px_rgba(201,152,58,0.35)]"
                  : "bg-gradient-to-br from-[#c9983a]/30 to-[#d4af37]/25 border-[#c9983a]/50 text-[#2d2820] hover:from-[#c9983a]/40 shadow-[0_4px_16px_rgba(201,152,58,0.2)]"
              } ${isSubmitting || submitted ? "opacity-60 cursor-not-allowed" : ""}`}
            >
              {isSubmitting ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" aria-hidden /> Publishing…
                </>
              ) : submitted ? (
                <>
                  <Check className="w-4 h-4" aria-hidden /> Published!
                </>
              ) : (
                <>
                  Publish program <Check className="w-4 h-4" aria-hidden />
                </>
              )}
            </button>
          )}
        </div>
      </div>
    </div>,
    document.body,
  );
}
