/**
 * ContributorSummarySheet — unit & accessibility tests
 *
 * Coverage:
 *  - Renders all required sections (identity, stats, heatmap, languages,
 *    ecosystems, certificates, footer)
 *  - Correct ARIA labels and landmark roles
 *  - Graceful empty states (no certificates, no languages, no ecosystems)
 *  - Heatmap generates 12 items regardless of input length
 *  - Avatar fallback renders initials when no URL provided
 *  - Certificates are typed (gold/blue/silver) with correct badge labels
 *  - PrintSummaryButton renders with correct aria-label and calls window.print
 *  - paper-size class applied correctly (a4 / letter)
 *  - "no-print" class on PrintSummaryButton (print CSS hides it)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  ContributorSummarySheet,
  PrintSummaryButton,
  type ContributorSummarySheetProps,
} from "../ContributorSummarySheet";

/* ── Minimal fixture ──────────────────────────────────────────────────────── */

const BASE_PROPS: ContributorSummarySheetProps = {
  displayName: "Amara Nwosu",
  username: "amara-nwosu",
  role: "Protocol Engineer",
  joinDate: "March 2025",
  topLanguages: ["TypeScript", "Rust", "Go", "Python", "Solidity"],
  ecosystems: ["Stellar", "Ethereum", "Cosmos"],
  totalBountiesWon: 12,
  totalEarned: "$8,400 USD",
  prsMerged: 47,
  issuesResolved: 31,
  contributionMonths: [3, 1, 5, 8, 12, 7, 4, 2, 9, 6, 11, 10],
  certificates: [
    { name: "Cairo Quests Q1 2026", variant: "gold", certId: "CERT-HK-2026-0628" },
    { name: "Soroban SDK Scholarship", variant: "blue", certId: "CERT-SC-2026-0301" },
  ],
  paperSize: "a4",
};

/* ═══════════════════════════════════════════════════════════════════════════ */
/*  ContributorSummarySheet                                                   */
/* ═══════════════════════════════════════════════════════════════════════════ */

describe("ContributorSummarySheet", () => {
  /* ── Root element ─────────────────────────────────────────────────────── */

  it("renders the sheet root with data-testid", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByTestId("contributor-summary-sheet")).toBeInTheDocument();
  });

  it("applies cs-sheet--a4 class by default", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByTestId("contributor-summary-sheet")).toHaveClass("cs-sheet--a4");
  });

  it("applies cs-sheet--letter class when paperSize=letter", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} paperSize="letter" />);
    expect(screen.getByTestId("contributor-summary-sheet")).toHaveClass("cs-sheet--letter");
  });

  it("has accessible aria-label on root", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(
      screen.getByLabelText("Contributor summary for Amara Nwosu")
    ).toBeInTheDocument();
  });

  /* ── Identity ─────────────────────────────────────────────────────────── */

  it("renders the contributor display name", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByTestId("cs-name")).toHaveTextContent("Amara Nwosu");
  });

  it("renders the contributor role when provided", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByTestId("cs-role")).toHaveTextContent("Protocol Engineer");
  });

  it("does not render role element when role is omitted", () => {
    const { role: _, ...noRole } = BASE_PROPS;
    render(<ContributorSummarySheet {...noRole} />);
    expect(screen.queryByTestId("cs-role")).toBeNull();
  });

  it("renders the username in the meta line", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText(/@amara-nwosu/)).toBeInTheDocument();
  });

  it("renders the join date in the meta line", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText(/March 2025/)).toBeInTheDocument();
  });

  it("renders avatar img with alt text when avatarUrl is provided", () => {
    render(
      <ContributorSummarySheet
        {...BASE_PROPS}
        avatarUrl="https://avatars.githubusercontent.com/u/1"
      />
    );
    expect(
      screen.getByRole("img", { name: "Amara Nwosu avatar" })
    ).toBeInTheDocument();
  });

  it("renders initials fallback when no avatarUrl is provided", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} avatarUrl={undefined} />);
    // The fallback div has aria-hidden; verify initials text exists in DOM
    const fallback = document.querySelector(".cs-avatar--fallback");
    expect(fallback).toBeInTheDocument();
    expect(fallback?.textContent).toBe("AM");
  });

  /* ── Branding ─────────────────────────────────────────────────────────── */

  it("renders Verified Contributor branding", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(
      screen.getByLabelText("Verified by Grainlify")
    ).toBeInTheDocument();
  });

  /* ── Stats row ────────────────────────────────────────────────────────── */

  it("renders the Contribution statistics section", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(
      screen.getByRole("region", { name: "Contribution statistics" })
    ).toBeInTheDocument();
  });

  it("renders total bounties won", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("Bounties Won")).toBeInTheDocument();
  });

  it("renders total earned", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("$8,400 USD")).toBeInTheDocument();
    expect(screen.getByText("Total Earned")).toBeInTheDocument();
  });

  it("renders PRs merged count", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("47")).toBeInTheDocument();
    expect(screen.getByText("PRs Merged")).toBeInTheDocument();
  });

  it("renders issues resolved count", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("31")).toBeInTheDocument();
    expect(screen.getByText("Issues Resolved")).toBeInTheDocument();
  });

  /* ── Heatmap ──────────────────────────────────────────────────────────── */

  it("renders the Contribution Activity section heading", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("Contribution Activity")).toBeInTheDocument();
  });

  it("renders exactly 12 heatmap cells regardless of input length", () => {
    const shortInput = { ...BASE_PROPS, contributionMonths: [5, 10] };
    render(<ContributorSummarySheet {...shortInput} />);
    const cells = document.querySelectorAll(".cs-heatmap-cell");
    expect(cells.length).toBe(12);
  });

  it("heatmap has accessible aria-label", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(
      screen.getByLabelText("Contribution activity heatmap by month")
    ).toBeInTheDocument();
  });

  it("each heatmap cell has an aria-label with month and count", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    // Jan is index 0, value 3 from fixture
    expect(screen.getByLabelText("Jan: 3 contributions")).toBeInTheDocument();
    // Jun is index 5, value 7
    expect(screen.getByLabelText("Jun: 7 contributions")).toBeInTheDocument();
  });

  /* ── Languages ────────────────────────────────────────────────────────── */

  it("renders the Top Languages section", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("Top Languages")).toBeInTheDocument();
  });

  it("renders up to 5 language chips", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("TypeScript")).toBeInTheDocument();
    expect(screen.getByText("Rust")).toBeInTheDocument();
    expect(screen.getByText("Go")).toBeInTheDocument();
    expect(screen.getByText("Python")).toBeInTheDocument();
    expect(screen.getByText("Solidity")).toBeInTheDocument();
  });

  it("does not render languages section when topLanguages is empty", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} topLanguages={[]} />);
    expect(screen.queryByText("Top Languages")).toBeNull();
  });

  it("language list has accessible aria-label", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(
      screen.getByRole("list", { name: "Top programming languages" })
    ).toBeInTheDocument();
  });

  /* ── Ecosystems ───────────────────────────────────────────────────────── */

  it("renders the Ecosystems section", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("Ecosystems")).toBeInTheDocument();
  });

  it("renders ecosystem chips", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("Stellar")).toBeInTheDocument();
    expect(screen.getByText("Ethereum")).toBeInTheDocument();
    expect(screen.getByText("Cosmos")).toBeInTheDocument();
  });

  it("does not render ecosystems section when ecosystems is empty", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} ecosystems={[]} />);
    expect(screen.queryByText("Ecosystems")).toBeNull();
  });

  /* ── Certificates ─────────────────────────────────────────────────────── */

  it("renders the Program Certificates section heading", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("Program Certificates")).toBeInTheDocument();
  });

  it("renders certificate names", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByText("Cairo Quests Q1 2026")).toBeInTheDocument();
    expect(screen.getByText("Soroban SDK Scholarship")).toBeInTheDocument();
  });

  it("renders certificate IDs", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByLabelText("Certificate ID CERT-HK-2026-0628")).toBeInTheDocument();
    expect(screen.getByLabelText("Certificate ID CERT-SC-2026-0301")).toBeInTheDocument();
  });

  it('renders "Hackathon" badge for gold variant', () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByLabelText("Hackathon certificate")).toBeInTheDocument();
  });

  it('renders "Scholarship" badge for blue variant', () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByLabelText("Scholarship certificate")).toBeInTheDocument();
  });

  it('renders "Bounty" badge for silver variant', () => {
    const withSilver = {
      ...BASE_PROPS,
      certificates: [
        { name: "Bug Bounty Sprint", variant: "silver" as const, certId: "CERT-BN-2026-0101" },
      ],
    };
    render(<ContributorSummarySheet {...withSilver} />);
    expect(screen.getByLabelText("Bounty certificate")).toBeInTheDocument();
  });

  it("renders empty state when certificates array is empty", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} certificates={[]} />);
    expect(screen.getByText("No certificates issued yet.")).toBeInTheDocument();
  });

  it("certificate list has accessible aria-label", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(
      screen.getByRole("list", { name: "Issued certificates" })
    ).toBeInTheDocument();
  });

  /* ── Footer ───────────────────────────────────────────────────────────── */

  it("renders Grainlify branding text in footer", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(
      screen.getByText(/Generated by Grainlify/)
    ).toBeInTheDocument();
  });

  it("renders a generated date in the footer", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    const date = screen.getByLabelText("Document generated date");
    expect(date.textContent).toBeTruthy();
  });

  /* ── Semantic structure ───────────────────────────────────────────────── */

  it("has a main landmark", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByRole("main")).toBeInTheDocument();
  });

  it("has a contentinfo (footer) landmark", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    expect(screen.getByRole("contentinfo")).toBeInTheDocument();
  });

  it("section headings are h2 elements", () => {
    render(<ContributorSummarySheet {...BASE_PROPS} />);
    const headings = screen.getAllByRole("heading", { level: 2 });
    const texts = headings.map((h) => h.textContent?.trim());
    expect(texts).toContain("Contribution Activity");
    expect(texts).toContain("Top Languages");
    expect(texts).toContain("Program Certificates");
  });
});

/* ═══════════════════════════════════════════════════════════════════════════ */
/*  PrintSummaryButton                                                        */
/* ═══════════════════════════════════════════════════════════════════════════ */

describe("PrintSummaryButton", () => {
  let printSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    printSpy = vi.fn();
    vi.stubGlobal("print", printSpy);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders with default label", () => {
    render(<PrintSummaryButton />);
    expect(
      screen.getByRole("button", { name: "Print / Save as PDF" })
    ).toBeInTheDocument();
  });

  it("renders with custom label", () => {
    render(<PrintSummaryButton label="Save summary as PDF" />);
    expect(
      screen.getByRole("button", { name: "Save summary as PDF" })
    ).toBeInTheDocument();
  });

  it("calls window.print when clicked", () => {
    render(<PrintSummaryButton />);
    fireEvent.click(screen.getByRole("button", { name: "Print / Save as PDF" }));
    expect(printSpy).toHaveBeenCalledTimes(1);
  });

  it("has no-print class so it is hidden in print output", () => {
    render(<PrintSummaryButton />);
    const btn = screen.getByRole("button", { name: "Print / Save as PDF" });
    expect(btn).toHaveClass("no-print");
  });

  it("has cs-print-btn class", () => {
    render(<PrintSummaryButton />);
    const btn = screen.getByRole("button", { name: "Print / Save as PDF" });
    expect(btn).toHaveClass("cs-print-btn");
  });

  it("button is keyboard accessible (has type=button)", () => {
    render(<PrintSummaryButton />);
    const btn = screen.getByRole("button", { name: "Print / Save as PDF" });
    expect(btn).toHaveAttribute("type", "button");
  });

  it("applies extra className when provided", () => {
    render(<PrintSummaryButton className="my-custom-class" />);
    const btn = screen.getByRole("button", { name: "Print / Save as PDF" });
    expect(btn).toHaveClass("my-custom-class");
  });
});
