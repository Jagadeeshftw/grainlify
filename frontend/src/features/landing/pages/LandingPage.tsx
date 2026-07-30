import { Navbar } from "../components/Navbar";
import { Hero } from "../components/Hero";
import {
  Code,
  GitBranch,
  Award,
  Shield,
  Zap,
  Users,
  TrendingUp,
  CheckCircle,
  Star,
  Quote,
  Pause,
  Play,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { useTheme } from "../../../shared/contexts/ThemeContext";
import { useLandingStats } from "../../../shared/hooks/useLandingStats";
import { useEffect, useCallback, useRef, useState } from "react";
import {
  Carousel,
  CarouselContent,
  CarouselItem,
  type CarouselApi,
} from "../../../app/components/ui/carousel";
import { useNavigate } from "react-router-dom";

// ---------------------------------------------------------------------------
// Hook: detects prefers-reduced-motion media query
// ---------------------------------------------------------------------------
function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(
    () =>
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  useEffect(() => {
    if (typeof window === "undefined") return;
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handler = (e: MediaQueryListEvent) => setReduced(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  return reduced;
}

export function LandingPage() {
  const { theme } = useTheme();
  const navigate = useNavigate();

  // Check for OAuth callback token in URL (fallback for wrong redirect URL)
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const token = params.get("token");
    const github = params.get("github");

    console.log("LandingPage - Checking for token in URL");
    console.log("LandingPage - Current URL:", window.location.href);
    console.log("LandingPage - Token found:", token ? "Yes" : "No");
    console.log("LandingPage - GitHub username:", github);

    if (token) {
      console.log("LandingPage - Redirecting to /auth/callback with token");
      // If there's a token in the URL, redirect to the proper callback handler
      navigate(`/auth/callback?token=${token}`, { replace: true });
    }
  }, [navigate]);

  return (
    <div
      className={`min-h-screen transition-colors ${
        theme === "dark"
          ? "bg-gradient-to-br from-[#1a1512] via-[#231c17] to-[#2d241d]"
          : "bg-gradient-to-br from-[#e8dfd0] via-[#d4c5b0] to-[#c9b89a]"
      }`}
    >
      <Navbar />
      <Hero />
      <Features />
      <HowItWorks />
      <WhyChooseUs />
      <Testimonials />
      <Footer />
    </div>
  );
}

function Features() {
  const { theme } = useTheme();

  const features = [
    {
      icon: Code,
      title: "Smart Matching",
      description:
        "AI-powered algorithm matches contributors with projects that fit their skills and interests.",
    },
    {
      icon: GitBranch,
      title: "Seamless Integration",
      description:
        "Connect your GitHub, track contributions, and manage everything in one place.",
    },
    {
      icon: Award,
      title: "Rewards & Recognition",
      description:
        "Get compensated for your contributions with transparent grant distribution.",
    },
    {
      icon: Shield,
      title: "Secure & Transparent",
      description:
        "Built on blockchain technology ensuring secure, transparent transactions.",
    },
    {
      icon: Zap,
      title: "Real-time Updates",
      description:
        "Stay informed with instant notifications about project updates and opportunities.",
    },
    {
      icon: Users,
      title: "Community Driven",
      description:
        "Join a thriving community of developers, maintainers, and open-source enthusiasts.",
    },
  ];

  return (
    <section
      id="features"
      className="relative py-20 sm:py-24 md:py-32 px-4 sm:px-6"
    >
      <div className="max-w-7xl mx-auto">
        {/* Section Header */}
        <div className="text-center mb-16">
          <h2
            className={`text-4xl md:text-5xl font-bold mb-6 transition-colors ${
              theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
            }`}
          >
            Everything You Need to Succeed
          </h2>
          <p
            className={`text-xl max-w-2xl mx-auto transition-colors ${
              theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
            }`}
          >
            Powerful features designed to streamline your open-source journey
          </p>
        </div>

        {/* Features Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((feature, index) => (
            <div
              key={index}
              className={`group backdrop-blur-[40px] border rounded-[24px] p-8 transition-all hover:scale-105 hover:border-[#c9983a]/30 hover:shadow-[0_12px_36px_rgba(201,152,58,0.15)] ${
                theme === "dark"
                  ? "bg-white/[0.08] border-white/15 hover:bg-white/[0.12]"
                  : "bg-white/[0.15] border-white/25 hover:bg-white/[0.2]"
              }`}
            >
              <div className="w-14 h-14 rounded-[14px] bg-gradient-to-br from-[#c9983a]/25 to-[#d4af37]/15 border border-[#c9983a]/30 flex items-center justify-center mb-6 group-hover:scale-110 transition-transform shadow-[0_4px_12px_rgba(201,152,58,0.15)]">
                <feature.icon className="w-7 h-7 text-[#c9983a]" />
              </div>
              <h3
                className={`text-xl font-semibold mb-3 transition-colors ${
                  theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                }`}
              >
                {feature.title}
              </h3>
              <p
                className={`transition-colors ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                {feature.description}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function HowItWorks() {
  const { theme } = useTheme();

  const steps = [
    {
      number: "01",
      title: "Create Your Profile",
      description:
        "Sign up and showcase your skills, interests, and open-source experience.",
    },
    {
      number: "02",
      title: "Discover Projects",
      description:
        "Browse through curated projects or get matched with opportunities that fit you.",
    },
    {
      number: "03",
      title: "Start Contributing",
      description:
        "Connect with maintainers, pick up tasks, and start making an impact.",
    },
    {
      number: "04",
      title: "Earn Rewards",
      description:
        "Receive grants and recognition for your valuable contributions.",
    },
  ];

  return (
    <section
      id="how-it-works"
      className="relative py-20 sm:py-24 md:py-32 px-4 sm:px-6"
    >
      <div className="max-w-7xl mx-auto">
        {/* Section Header */}
        <div className="text-center mb-16">
          <h2
            className={`text-4xl md:text-5xl font-bold mb-6 transition-colors ${
              theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
            }`}
          >
            How It Works
          </h2>
          <p
            className={`text-xl max-w-2xl mx-auto transition-colors ${
              theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
            }`}
          >
            Get started in four simple steps
          </p>
        </div>

        {/* Steps */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
          {steps.map((step, index) => (
            <div key={index} className="relative">
              {/* Connector Line (desktop) */}
              {index < steps.length - 1 && (
                <div className="hidden lg:block absolute top-16 left-full w-full h-0.5 bg-gradient-to-r from-[#c9983a]/50 to-transparent" />
              )}

              <div
                className={`backdrop-blur-[40px] border rounded-[24px] p-8 transition-all hover:border-[#c9983a]/30 hover:shadow-[0_12px_36px_rgba(201,152,58,0.15)] ${
                  theme === "dark"
                    ? "bg-white/[0.08] border-white/15 hover:bg-white/[0.12]"
                    : "bg-white/[0.15] border-white/25 hover:bg-white/[0.2]"
                }`}
              >
                <div className="text-6xl font-bold bg-gradient-to-r from-[#c9983a] to-[#d4af37] bg-clip-text text-transparent mb-6">
                  {step.number}
                </div>
                <h3
                  className={`text-2xl font-semibold mb-4 transition-colors ${
                    theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                  }`}
                >
                  {step.title}
                </h3>
                <p
                  className={`transition-colors ${
                    theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                  }`}
                >
                  {step.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function WhyChooseUs() {
  const { theme } = useTheme();
  const { display } = useLandingStats();

  const benefits = [
    "Verified and vetted projects from trusted organizations",
    "Fair compensation with transparent grant distribution",
    "Comprehensive skill development and mentorship",
    "Active community support and collaboration",
    "Real-time project tracking and analytics",
    "Secure blockchain-based transactions",
  ];

  return (
    <section
      id="why-choose-us"
      className="relative py-20 sm:py-24 md:py-32 px-4 sm:px-6"
    >
      <div className="max-w-7xl mx-auto">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
          {/* Left: Benefits */}
          <div>
            <h2
              className={`text-4xl md:text-5xl font-bold mb-6 transition-colors ${
                theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
              }`}
            >
              Why Choose Grainlify?
            </h2>
            <p
              className={`text-xl mb-10 transition-colors ${
                theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
              }`}
            >
              We're more than just a platform – we're your partner in
              open-source success.
            </p>

            <div className="space-y-4">
              {benefits.map((benefit, index) => (
                <div key={index} className="flex items-start space-x-4">
                  <div className="flex-shrink-0 w-6 h-6 rounded-full bg-gradient-to-br from-[#c9983a] to-[#d4af37] flex items-center justify-center mt-1 shadow-[0_2px_8px_rgba(201,152,58,0.4)]">
                    <CheckCircle className="w-4 h-4 text-white" />
                  </div>
                  <p
                    className={`transition-colors ${
                      theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                    }`}
                  >
                    {benefit}
                  </p>
                </div>
              ))}
            </div>

            <div className="mt-10 flex items-center space-x-8">
              <div>
                <div
                  className={`text-4xl font-bold transition-colors ${
                    theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                  }`}
                >
                  98%
                </div>
                <div
                  className={`transition-colors ${
                    theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                  }`}
                >
                  Satisfaction Rate
                </div>
              </div>
              <div>
                <div
                  className={`text-4xl font-bold transition-colors ${
                    theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                  }`}
                >
                  24/7
                </div>
                <div
                  className={`transition-colors ${
                    theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                  }`}
                >
                  Support Available
                </div>
              </div>
            </div>
          </div>

          {/* Right: Visual Element */}
          <div className="relative">
            <div
              className={`backdrop-blur-[40px] border rounded-[28px] p-8 relative overflow-hidden shadow-[0_8px_32px_rgba(0,0,0,0.08)] ${
                theme === "dark"
                  ? "bg-white/[0.08] border-white/15"
                  : "bg-white/[0.15] border-white/25"
              }`}
            >
              <div className="absolute top-0 right-0 w-64 h-64 bg-[#c9983a]/20 rounded-full blur-3xl" />
              <div className="relative space-y-6">
                {[
                  {
                    icon: TrendingUp,
                    label: "Growing Ecosystem",
                    value: "+45%",
                  },
                  {
                    icon: Users,
                    label: "Active Users",
                    value: display.contributors,
                  },
                  {
                    icon: Award,
                    label: "Projects Funded",
                    value: display.activeProjects,
                  },
                ].map((item, index) => (
                  <div
                    key={index}
                    className={`flex items-center space-x-4 backdrop-blur-[25px] border rounded-[16px] p-4 transition-all hover:border-[#c9983a]/30 ${
                      theme === "dark"
                        ? "bg-white/[0.06] border-white/10 hover:bg-white/[0.1]"
                        : "bg-white/[0.12] border-white/20 hover:bg-white/[0.18]"
                    }`}
                  >
                    <div className="w-12 h-12 rounded-[12px] bg-gradient-to-br from-[#c9983a]/25 to-[#d4af37]/15 border border-[#c9983a]/30 flex items-center justify-center shadow-[0_4px_12px_rgba(201,152,58,0.15)]">
                      <item.icon className="w-6 h-6 text-[#c9983a]" />
                    </div>
                    <div className="flex-1">
                      <div
                        className={`text-sm transition-colors ${
                          theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                        }`}
                      >
                        {item.label}
                      </div>
                      <div
                        className={`text-xl font-semibold transition-colors ${
                          theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                        }`}
                      >
                        {item.value}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

function Testimonials() {
  const { theme } = useTheme();
  const prefersReducedMotion = useReducedMotion();

  // ---------------------------------------------------------------------------
  // Data
  // ---------------------------------------------------------------------------
  const testimonials = [
    {
      name: "Sarah Chen",
      role: "Full Stack Developer",
      avatar:
        "https://images.unsplash.com/photo-1655249481446-25d575f1c054?w=400",
      content:
        "Grainlify helped me find amazing projects that align with my interests. The grant system is transparent and fair!",
      rating: 5,
    },
    {
      name: "Marcus Johnson",
      role: "Project Maintainer",
      avatar:
        "https://images.unsplash.com/photo-1655249481446-25d575f1c054?w=400",
      content:
        "As a maintainer, this platform has been incredible for finding talented contributors. Highly recommend!",
      rating: 5,
    },
    {
      name: "Emily Rodriguez",
      role: "Open Source Contributor",
      avatar:
        "https://images.unsplash.com/photo-1655249481446-25d575f1c054?w=400",
      content:
        "The community here is amazing. I've learned so much and made great connections through Grainlify.",
      rating: 5,
    },
  ];

  // ---------------------------------------------------------------------------
  // Carousel API state
  // ---------------------------------------------------------------------------
  const [api, setApi] = useState<CarouselApi>();
  const [current, setCurrent] = useState(0);
  const [count, setCount] = useState(0);

  useEffect(() => {
    if (!api) return;
    setCount(api.scrollSnapList().length);
    setCurrent(api.selectedScrollSnap());
    const onSelect = () => setCurrent(api.selectedScrollSnap());
    api.on("select", onSelect);
    return () => {
      api.off("select", onSelect);
    };
  }, [api]);

  // ---------------------------------------------------------------------------
  // Autoplay state
  // ---------------------------------------------------------------------------
  const AUTOPLAY_INTERVAL = 5000; // ms
  const [isManuallyPaused, setIsManuallyPaused] = useState(false);
  const [isHoverPaused, setIsHoverPaused] = useState(false);
  const [isFocusPaused, setIsFocusPaused] = useState(false);
  const [isTouchPaused, setIsTouchPaused] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Derived: autoplay should run when nothing pauses it and reduced-motion is off
  const isPlaying =
    !prefersReducedMotion &&
    !isManuallyPaused &&
    !isHoverPaused &&
    !isFocusPaused &&
    !isTouchPaused;

  // Advance to next slide
  const advance = useCallback(() => {
    api?.scrollNext();
  }, [api]);

  // Start / stop interval based on isPlaying
  useEffect(() => {
    if (timerRef.current) clearInterval(timerRef.current);
    if (isPlaying) {
      timerRef.current = setInterval(advance, AUTOPLAY_INTERVAL);
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [isPlaying, advance]);

  // Reset timer after manual navigation
  const resetTimer = useCallback(() => {
    if (timerRef.current) clearInterval(timerRef.current);
    if (isPlaying) {
      timerRef.current = setInterval(advance, AUTOPLAY_INTERVAL);
    }
  }, [isPlaying, advance]);

  // ---------------------------------------------------------------------------
  // Event handlers
  // ---------------------------------------------------------------------------
  const handleMouseEnter = () => setIsHoverPaused(true);
  const handleMouseLeave = () => setIsHoverPaused(false);

  // Use focusin/focusout so any descendant focus is detected
  const handleFocusIn = () => setIsFocusPaused(true);
  const handleFocusOut = (e: React.FocusEvent<HTMLElement>) => {
    // Only resume if focus leaves the carousel entirely
    const sectionEl = e.currentTarget;
    if (!sectionEl.contains(e.relatedTarget as Node | null)) {
      setIsFocusPaused(false);
    }
  };

  const handleTouchStart = () => setIsTouchPaused(true);

  const handlePlayPauseToggle = () => {
    setIsManuallyPaused((prev) => !prev);
    // Touch users already set isTouchPaused; clicking play should also clear it
    setIsTouchPaused(false);
  };

  const handleDotClick = (index: number) => {
    api?.scrollTo(index);
    resetTimer();
  };

  const handlePrevClick = () => {
    api?.scrollPrev();
    resetTimer();
  };

  const handleNextClick = () => {
    api?.scrollNext();
    resetTimer();
  };

  // ---------------------------------------------------------------------------
  // aria-live: "off" during autoplay, "polite" when paused so screen readers
  // announce the slide content without being interrupted by auto-advances.
  // ---------------------------------------------------------------------------
  const ariaLive: React.AriaAttributes["aria-live"] =
    isPlaying && !prefersReducedMotion ? "off" : "polite";

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------
  return (
    <section
      id="testimonials"
      aria-label="Testimonials"
      aria-roledescription="carousel"
      className="relative py-20 sm:py-24 md:py-32 px-4 sm:px-6"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onFocusCapture={handleFocusIn}
      onBlurCapture={handleFocusOut}
      onTouchStart={handleTouchStart}
    >
      <div className="max-w-7xl mx-auto">
        {/* Section Header */}
        <div className="text-center mb-16">
          <h2
            className={`text-4xl md:text-5xl font-bold mb-6 transition-colors ${
              theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
            }`}
          >
            What Builders Say
          </h2>
          <p
            className={`text-xl max-w-2xl mx-auto transition-colors ${
              theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
            }`}
          >
            Hear from our community of contributors and maintainers
          </p>
        </div>

        {/* Carousel */}
        <Carousel
          opts={{
            loop: true,
            align: "start",
          }}
          setApi={setApi}
          className="w-full"
        >
          {/* Slide list — aria-live region */}
          <div aria-live={ariaLive} aria-atomic="false" aria-relevant="additions">
            <CarouselContent className="-ml-4">
              {testimonials.map((testimonial, index) => (
                <CarouselItem
                  key={index}
                  aria-label={`Testimonial ${index + 1} of ${testimonials.length}`}
                  className="pl-4 md:basis-1/2 lg:basis-1/3"
                >
                  <div
                    className={`h-full backdrop-blur-[40px] border rounded-[24px] p-8 transition-all hover:border-[#c9983a]/30 hover:shadow-[0_12px_36px_rgba(201,152,58,0.15)] ${
                      theme === "dark"
                        ? "bg-white/[0.08] border-white/15 hover:bg-white/[0.12]"
                        : "bg-white/[0.15] border-white/25 hover:bg-white/[0.2]"
                    }`}
                  >
                    <Quote
                      className="w-10 h-10 text-[#c9983a]/30 mb-6"
                      aria-hidden="true"
                    />

                    {/* Star Rating */}
                    <div
                      role="img"
                      aria-label={`${testimonial.rating} out of 5 stars`}
                      className="flex space-x-1 mb-6"
                    >
                      {[...Array(testimonial.rating)].map((_, i) => (
                        <Star
                          key={i}
                          className="w-5 h-5 fill-[#c9983a] text-[#c9983a]"
                          aria-hidden="true"
                        />
                      ))}
                    </div>

                    {/* Quote content */}
                    <blockquote
                      aria-label={`Quote from ${testimonial.name}`}
                      className={`mb-6 transition-colors ${
                        theme === "dark" ? "text-[#f5f5f5]" : "text-[#2d2820]"
                      }`}
                    >
                      {testimonial.content}
                    </blockquote>

                    {/* Author */}
                    <div className="flex items-center space-x-4">
                      <img
                        src={testimonial.avatar}
                        alt={`${testimonial.name}, ${testimonial.role}`}
                        className="w-12 h-12 rounded-full object-cover border-2 border-[#c9983a]/30"
                      />
                      <div>
                        <div
                          className={`font-semibold transition-colors ${
                            theme === "dark"
                              ? "text-[#e8dfd0]"
                              : "text-[#2d2820]"
                          }`}
                        >
                          {testimonial.name}
                        </div>
                        <div
                          className={`text-sm transition-colors ${
                            theme === "dark"
                              ? "text-[#d4d4d4]"
                              : "text-[#7a6b5a]"
                          }`}
                        >
                          {testimonial.role}
                        </div>
                      </div>
                    </div>
                  </div>
                </CarouselItem>
              ))}
            </CarouselContent>
          </div>

          {/* Controls bar: play/pause · dot indicators · prev/next */}
          <div className="mt-8 flex items-center justify-center gap-4">
            {/* Play / Pause toggle — hidden for reduced-motion users */}
            {!prefersReducedMotion && (
              <button
                type="button"
                aria-label={
                  isManuallyPaused
                    ? "Play testimonial carousel"
                    : "Pause testimonial carousel"
                }
                aria-pressed={isManuallyPaused}
                onClick={handlePlayPauseToggle}
                className={`flex items-center justify-center w-8 h-8 rounded-full border transition-colors
                  focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]
                  ${
                    theme === "dark"
                      ? "border-white/20 text-[#c9983a] hover:border-[#c9983a]/50 hover:bg-white/10"
                      : "border-black/20 text-[#c9983a] hover:border-[#c9983a]/50 hover:bg-black/5"
                  }`}
              >
                {isManuallyPaused ? (
                  <Play className="w-4 h-4" aria-hidden="true" />
                ) : (
                  <Pause className="w-4 h-4" aria-hidden="true" />
                )}
              </button>
            )}

            {/* Prev arrow */}
            <button
              type="button"
              aria-label="Previous testimonial"
              onClick={handlePrevClick}
              className={`flex items-center justify-center w-8 h-8 rounded-full border transition-colors
                focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]
                ${
                  theme === "dark"
                    ? "border-white/20 text-[#c9983a] hover:border-[#c9983a]/50 hover:bg-white/10"
                    : "border-black/20 text-[#c9983a] hover:border-[#c9983a]/50 hover:bg-black/5"
                }`}
            >
              <ChevronLeft className="w-4 h-4" aria-hidden="true" />
            </button>

            {/* Dot indicators */}
            <div
              role="group"
              aria-label="Carousel slide indicators"
              className="flex items-center gap-2"
            >
              {Array.from({ length: count }).map((_, index) => (
                <button
                  key={index}
                  type="button"
                  aria-label={`Go to testimonial ${index + 1} of ${count}`}
                  aria-current={index === current ? "true" : undefined}
                  onClick={() => handleDotClick(index)}
                  className={`rounded-full transition-all
                    focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]
                    ${
                      index === current
                        ? "w-2.5 h-2.5 bg-[#c9983a]"
                        : `w-2 h-2 ${
                            theme === "dark"
                              ? "bg-white/30 hover:bg-[#e8c77f]"
                              : "bg-black/20 hover:bg-[#c9983a]"
                          }`
                    }`}
                  style={{ minWidth: "24px", minHeight: "24px", padding: "7px" }}
                />
              ))}
            </div>

            {/* Next arrow */}
            <button
              type="button"
              aria-label="Next testimonial"
              onClick={handleNextClick}
              className={`flex items-center justify-center w-8 h-8 rounded-full border transition-colors
                focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#f1b400]
                ${
                  theme === "dark"
                    ? "border-white/20 text-[#c9983a] hover:border-[#c9983a]/50 hover:bg-white/10"
                    : "border-black/20 text-[#c9983a] hover:border-[#c9983a]/50 hover:bg-black/5"
                }`}
            >
              <ChevronRight className="w-4 h-4" aria-hidden="true" />
            </button>
          </div>
        </Carousel>
      </div>
    </section>
  );
}

function Footer() {
  const { theme } = useTheme();

  return (
    <footer className="relative py-16 px-6 border-t border-white/20">
      <div className="max-w-7xl mx-auto">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-8 mb-12">
          {/* Brand */}
          <div className="col-span-1">
            <div className="flex items-center space-x-2 mb-4">
              <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-[#c9983a] to-[#d4af37] shadow-[0_2px_8px_rgba(201,152,58,0.4)]" />
              <span
                className={`text-xl font-semibold transition-colors ${
                  theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
                }`}
              >
                Grainlify
              </span>
            </div>
            <p
              className={`transition-colors ${
                theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
              }`}
            >
              Connecting talent with opportunity in the open-source ecosystem.
            </p>
          </div>

          {/* Links */}
          <div>
            <h4
              className={`font-semibold mb-4 transition-colors ${
                theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
              }`}
            >
              Product
            </h4>
            <div className="space-y-2">
              <a
                href="#features"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                Features
              </a>
              <a
                href="#how-it-works"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                How it Works
              </a>
              <a
                href="#"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                Pricing
              </a>
            </div>
          </div>

          <div>
            <h4
              className={`font-semibold mb-4 transition-colors ${
                theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
              }`}
            >
              Company
            </h4>
            <div className="space-y-2">
              <a
                href="#"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                About
              </a>
              <a
                href="#"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                Blog
              </a>
              <a
                href="#"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                Careers
              </a>
            </div>
          </div>

          <div>
            <h4
              className={`font-semibold mb-4 transition-colors ${
                theme === "dark" ? "text-[#e8dfd0]" : "text-[#2d2820]"
              }`}
            >
              Resources
            </h4>
            <div className="space-y-2">
              <a
                href="#"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                Documentation
              </a>
              <a
                href="#"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                Support
              </a>
              <a
                href="#"
                className={`block transition-colors hover:text-[#c9983a] ${
                  theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
                }`}
              >
                Terms
              </a>
            </div>
          </div>
        </div>

        {/* Bottom */}
        <div
          className={`pt-8 border-t border-white/20 text-center transition-colors ${
            theme === "dark" ? "text-[#b8a898]" : "text-[#7a6b5a]"
          }`}
        >
          <p>&copy; 2024 Grainlify. All rights reserved.</p>
        </div>
      </div>
    </footer>
  );
}
