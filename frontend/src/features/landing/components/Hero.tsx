/**
 * Hero — Landing Page Hero Section
 *
 * Animation choreography (issue #1511):
 *   - Staggered entry: badge → headline → description → CTA group → stat cards
 *   - Timing tokens sourced exclusively from motionConfig.ts
 *   - Full prefers-reduced-motion fallback: instant opacity only, no transforms, no stagger
 *   - CTA primary: scale + gold glow on hover/focus; tap scales down
 *   - WCAG 2.1 AA — focus-visible available immediately (not animation-gated)
 *
 * @see design/specs/landing-hero-animation-choreography.md
 * @see frontend/src/shared/config/motionConfig.ts
 * @see frontend/src/shared/hooks/useReducedMotion.ts
 */

import { motion } from "motion/react";
import { Link } from "react-router-dom";
import { ArrowRight, Sparkles } from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { useLandingStats } from "../../../shared/hooks/useLandingStats";
import { useReducedMotion } from "../../../shared/hooks/useReducedMotion";
import { motionConfig } from "../../../shared/config/motionConfig";

// ---------------------------------------------------------------------------
// Animation variant factories — all values sourced from motionConfig tokens
// ---------------------------------------------------------------------------

/**
 * Outer container variant. Drives staggerChildren so each child fires in order.
 * When reduced-motion is requested the stagger and delay collapse to 0.
 */
function heroContainerVariants(prefersReduced: boolean) {
  if (prefersReduced) {
    return {
      hidden: { opacity: 0 },
      visible: {
        opacity: 1,
        transition: {
          staggerChildren: 0,
          delayChildren: 0,
        },
      },
    };
  }

  return {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        // staggerChildren fires each direct child variant in sequence
        staggerChildren: motionConfig.list.staggerDelay / 1000, // 0.05 s
        delayChildren: motionConfig.list.initialDelay / 1000,   // 0.1 s
      },
    },
  };
}

/**
 * Individual item variant for badge, headline, description, and CTA group.
 * Reduced-motion: opacity only, duration 0 (instant).
 */
function heroItemVariants(prefersReduced: boolean, yOffset: number = 24) {
  if (prefersReduced) {
    return {
      hidden: { opacity: 0 },
      visible: {
        opacity: 1,
        transition: { duration: motionConfig.durations.instant },
      },
    };
  }

  return {
    hidden: { opacity: 0, y: yOffset },
    visible: {
      opacity: 1,
      y: 0,
      transition: {
        duration: motionConfig.durations.normal / 1000, // 0.3 s
        ease: motionConfig.easing.easeOut,
      },
    },
  };
}

/**
 * Stat card variant — slightly longer entrance to give the data strip a
 * distinct "landing" feel separate from the upper content.
 */
function statCardVariants(prefersReduced: boolean) {
  if (prefersReduced) {
    return {
      hidden: { opacity: 0 },
      visible: {
        opacity: 1,
        transition: { duration: motionConfig.durations.instant },
      },
    };
  }

  return {
    hidden: { opacity: 0, y: 16 },
    visible: {
      opacity: 1,
      y: 0,
      transition: {
        duration: motionConfig.durations.slow / 1000, // 0.5 s
        ease: motionConfig.easing.easeOut,
      },
    },
  };
}

/**
 * Stat strip container — fires its own stagger 300 ms after the CTA group
 * has begun its animation, creating the natural reading-order cascade.
 */
function statContainerVariants(prefersReduced: boolean) {
  if (prefersReduced) {
    return {
      hidden: { opacity: 0 },
      visible: {
        opacity: 1,
        transition: { staggerChildren: 0, delayChildren: 0 },
      },
    };
  }

  return {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        // Extra delay so stat cards enter after the CTA group settles
        delayChildren: 0.2, // seconds — adds to parent stagger context
        staggerChildren: motionConfig.list.staggerDelay / 1000, // 0.05 s
      },
    },
  };
}

/**
 * Primary CTA (motion.a wrapper around Link) — scale + gold glow on hover.
 * Reduced-motion: hover/tap effects disabled.
 */
function primaryCtaVariants(prefersReduced: boolean) {
  if (prefersReduced) {
    return {
      initial: { scale: 1 },
      whileHover: {},
      whileTap: {},
    };
  }

  return {
    initial: { scale: 1 },
    whileHover: {
      scale: motionConfig.interactions.buttonHover.scale, // 1.02
      boxShadow: "0 0 24px rgba(201,152,58,0.55)",
      transition: {
        duration: motionConfig.interactions.buttonHover.duration / 1000, // 0.15 s
        ease: motionConfig.easing.easeOut,
      },
    },
    whileTap: {
      scale: 0.97,
      transition: {
        duration: motionConfig.interactions.buttonTap.duration / 1000, // 0.1 s
      },
    },
  };
}

/**
 * Secondary CTA — subtle scale + border accent on hover.
 */
function secondaryCtaVariants(prefersReduced: boolean) {
  if (prefersReduced) {
    return {
      initial: { scale: 1 },
      whileHover: {},
      whileTap: {},
    };
  }

  return {
    initial: { scale: 1 },
    whileHover: {
      scale: 1.01,
      transition: {
        duration: motionConfig.interactions.buttonHover.duration / 1000,
        ease: motionConfig.easing.easeOut,
      },
    },
    whileTap: {
      scale: 0.98,
      transition: {
        duration: motionConfig.interactions.buttonTap.duration / 1000,
      },
    },
  };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Hero() {
  const { theme } = useTheme();
  const { display } = useLandingStats();
  const prefersReduced = useReducedMotion();

  const container = heroContainerVariants(prefersReduced);
  const item = heroItemVariants(prefersReduced);
  const statCard = statCardVariants(prefersReduced);
  const statContainer = statContainerVariants(prefersReduced);
  const primaryCta = primaryCtaVariants(prefersReduced);
  const secondaryCta = secondaryCtaVariants(prefersReduced);

  const stats = [
    { label: "Active Projects", value: display.activeProjects },
    { label: "Contributors", value: display.contributors },
    { label: "Grants Distributed", value: display.grantsDistributed },
  ];

  return (
    <section className="relative min-h-screen flex items-center justify-center px-4 sm:px-6 pt-20">
      {/*
       * Decorative background orbs — aria-hidden so screen readers skip them.
       * CSS animate-pulse is suppressed when prefers-reduced-motion is active
       * by conditionally applying the class.
       */}
      <div
        aria-hidden="true"
        className={`hidden sm:block absolute top-1/4 left-1/4 w-64 sm:w-96 h-64 sm:h-96 rounded-full bg-[#c9983a]/30 blur-3xl ${
          prefersReduced ? "" : "animate-pulse"
        }`}
      />
      <div
        aria-hidden="true"
        className={`hidden sm:block absolute bottom-1/4 right-1/4 w-64 sm:w-96 h-64 sm:h-96 rounded-full bg-[#d4af37]/20 blur-3xl ${
          prefersReduced ? "" : "animate-pulse delay-1000"
        }`}
      />

      {/* ----------------------------------------------------------------- */}
      {/* Animated content container                                         */}
      {/* ----------------------------------------------------------------- */}
      <motion.div
        className="relative z-10 max-w-6xl mx-auto text-center"
        variants={container}
        initial="hidden"
        animate="visible"
      >
        {/* Badge --------------------------------------------------------- */}
        <motion.div variants={item} className="inline-block mb-8">
          <div
            className={`inline-flex items-center space-x-2 px-4 py-2 rounded-full backdrop-blur-[30px] border transition-colors ${
              theme === "dark"
                ? "bg-white/[0.08] border-white/15"
                : "bg-white/[0.15] border-white/25"
            }`}
          >
            <Sparkles className="w-4 h-4 text-[#c9983a]" aria-hidden="true" />
            <span
              className={`text-sm font-medium transition-colors ${
                theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
              }`}
            >
              Web3 Contributors Platform
            </span>
          </div>
        </motion.div>

        {/* Headline ------------------------------------------------------- */}
        {/*
         * Accessibility note: the gradient <span> is purely visual.
         * The <h1> announces as one sentence: "Connect with Open Source
         * Opportunities" — no ARIA fragmentation.
         */}
        <motion.h1
          variants={item}
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
        </motion.h1>

        {/* Description ---------------------------------------------------- */}
        <motion.p
          variants={item}
          className={`text-base sm:text-lg max-w-2xl mx-auto mb-8 sm:mb-12 transition-colors ${
            theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
          }`}
        >
          Grainlify bridges the gap between talented contributors and innovative
          projects, making open-source collaboration seamless and rewarding.
        </motion.p>

        {/* CTA Buttons ---------------------------------------------------- */}
        {/*
         * Accessibility note: both CTAs are immediately focusable at T+0.
         * The motion.div wrapping them is opacity:0 in initial state, but
         * opacity does NOT remove elements from the tab order — keyboard
         * users can reach them before the animation completes.
         *
         * Focus ring: focus-visible:outline-[#c9983a] aligns with gold token.
         */}
        <motion.div
          variants={item}
          className="flex flex-col sm:flex-row items-center justify-center gap-4 w-full max-w-2xl mx-auto"
        >
          {/* Primary CTA */}
          <motion.div
            variants={primaryCta}
            initial="initial"
            whileHover="whileHover"
            whileTap="whileTap"
            className="w-full sm:w-auto"
          >
            <Link
              to="/signin"
              className="group w-full sm:w-auto px-6 sm:px-8 py-3 sm:py-4 rounded-[16px] bg-gradient-to-r from-[#c9983a] to-[#d4af37] text-white font-medium hover:shadow-2xl hover:shadow-[#c9983a]/50 transition-all flex items-center justify-center sm:inline-flex space-x-2 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#c9983a]"
            >
              <span>Get Started</span>
              <ArrowRight
                className="w-5 h-5 group-hover:translate-x-1 transition-transform"
                aria-hidden="true"
              />
            </Link>
          </motion.div>

          {/* Secondary CTA */}
          <motion.div
            variants={secondaryCta}
            initial="initial"
            whileHover="whileHover"
            whileTap="whileTap"
            className="w-full sm:w-auto"
          >
            <a
              href="https://grainlify-cuss.vercel.app/"
              target="_blank"
              rel="noopener noreferrer"
              className={`w-full sm:w-auto px-6 sm:px-8 py-3 sm:py-4 rounded-[16px] backdrop-blur-[30px] border font-medium transition-all inline-flex items-center justify-center focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#c9983a] ${
                theme === "dark"
                  ? "bg-white/[0.08] border-white/15 text-[#e8dfd0] hover:bg-white/[0.12] hover:border-[#c9983a]/30"
                  : "bg-white/[0.15] border-white/25 text-[#2d2820] hover:bg-white/[0.2] hover:border-[#c9983a]/30"
              }`}
            >
              Docs
            </a>
          </motion.div>
        </motion.div>

        {/* Stat Strip ------------------------------------------------------ */}
        {/*
         * Stats use a nested stagger container so they can fire with their own
         * sub-delay (200 ms after CTA) while remaining part of the parent cascade.
         */}
        <motion.div
          variants={statContainer}
          className="mt-12 sm:mt-16 grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-6 max-w-4xl mx-auto px-2"
        >
          {stats.map((stat) => (
            <motion.div
              key={stat.label}
              variants={statCard}
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
            </motion.div>
          ))}
        </motion.div>
      </motion.div>
    </section>
  );
}
