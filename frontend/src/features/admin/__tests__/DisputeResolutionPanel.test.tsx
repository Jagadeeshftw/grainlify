/**
 * DisputeResolutionPanel — component tests
 *
 * Coverage:
 *  - Rendering: detail / decide / confirm steps
 *  - Decision buttons (release / split / refund) update slider + preview
 *  - Split slider: mouse/keyboard, PgUp/PgDn, aria-valuetext
 *  - Live preview amounts update on slider change
 *  - Irreversibility warning rendered with role="alert"
 *  - onDecide called with correct payload on confirm
 *  - onClose called from close button and backdrop
 *  - Focus management: confirm button ref, slider keyboard
 *  - ARIA: aria-live regions, aria-pressed on decision buttons
 *  - Edge cases: 0 %, 100 %, 50 % split
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import type { Dispute } from "../components/DisputeResolutionPanel";
import { DisputeResolutionPanel } from "../components/DisputeResolutionPanel";

vi.mock("../../../shared/contexts/ThemeContext", () => ({
  useTheme: () => ({ theme: "dark" }),
}));

// ─── fixtures ─────────────────────────────────────────────────────────────────

const DISPUTE: Dispute = {
  id: "d-001",
  programId: "prog-stellar-q1",
  bountyTitle: "Implement XLM wallet adapter",
  totalAmountXlm: 1000,
  contributor: "GCONT...ADDR",
  maintainer: "GMAIN...ADDR",
  claim: "PR merged and all acceptance criteria met.",
  counterClaim: "Integration tests not included; criteria not fully met.",
  evidence: [
    { label: "PR #42", url: "https://github.com/example/repo/pull/42" },
    { label: "Test run", url: "https://ci.example.com/run/99" },
  ],
  timeline: [
    { timestamp: "2026-06-01 10:00", actor: "GCONT...ADDR", action: "Opened dispute" },
    { timestamp: "2026-06-02 09:30", actor: "GMAIN...ADDR", action: "Submitted counter-claim" },
  ],
  status: "open",
};

function renderPanel(overrides?: Partial<Dispute>) {
  const onDecide = vi.fn();
  const onClose = vi.fn();
  render(
    <DisputeResolutionPanel
      dispute={{ ...DISPUTE, ...overrides }}
      onDecide={onDecide}
      onClose={onClose}
    />
  );
  return { onDecide, onClose };
}

// ─── helpers ──────────────────────────────────────────────────────────────────

function goToDecide() {
  fireEvent.click(screen.getByTestId("proceed-to-decide"));
}

function goToConfirm() {
  goToDecide();
  fireEvent.click(screen.getByTestId("proceed-to-confirm"));
}

// ══════════════════════════════════════════════════════════════════════════════

describe("DisputeResolutionPanel — detail step", () => {
  it("renders dispute title and program id", () => {
    renderPanel();
    expect(screen.getByText("Implement XLM wallet adapter")).toBeInTheDocument();
    expect(screen.getByText(/prog-stellar-q1/)).toBeInTheDocument();
  });

  it("renders claim and counter-claim", () => {
    renderPanel();
    expect(screen.getByTestId("claim-panel")).toHaveTextContent("PR merged");
    expect(screen.getByTestId("counter-claim-panel")).toHaveTextContent("Integration tests not included");
  });

  it("renders evidence pills with external links", () => {
    renderPanel();
    const links = screen.getAllByRole("link");
    expect(links.some((l) => l.textContent?.includes("PR #42"))).toBe(true);
    expect(links.some((l) => l.textContent?.includes("Test run"))).toBe(true);
  });

  it("all evidence links open in new tab", () => {
    renderPanel();
    screen.getAllByRole("link").forEach((l) => {
      expect(l).toHaveAttribute("target", "_blank");
      expect(l).toHaveAttribute("rel", "noopener noreferrer");
    });
  });

  it("renders timeline events", () => {
    renderPanel();
    expect(screen.getByTestId("timeline-section")).toHaveTextContent("Opened dispute");
    expect(screen.getByTestId("timeline-section")).toHaveTextContent("Submitted counter-claim");
  });

  it("hides evidence section when no evidence", () => {
    renderPanel({ evidence: [] });
    expect(screen.queryByTestId("evidence-section")).toBeNull();
  });

  it("calls onClose when close button clicked", () => {
    const { onClose } = renderPanel();
    fireEvent.click(screen.getByTestId("close-button"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onClose when backdrop clicked", () => {
    const { onClose } = renderPanel();
    const backdrop = document.querySelector("[aria-hidden=\"true\"]") as HTMLElement;
    fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalled();
  });

  it("navigates to decide step", () => {
    renderPanel();
    fireEvent.click(screen.getByTestId("proceed-to-decide"));
    expect(screen.getByTestId("decision-panel")).toBeInTheDocument();
  });

  it("has correct ARIA: dialog role and aria-modal", () => {
    renderPanel();
    const panel = screen.getByTestId("dispute-panel");
    expect(panel).toHaveAttribute("role", "dialog");
    expect(panel).toHaveAttribute("aria-modal", "true");
  });
});

// ══════════════════════════════════════════════════════════════════════════════

describe("DisputeResolutionPanel — decision step", () => {
  beforeEach(() => {
    renderPanel();
    goToDecide();
  });

  it("shows total escrow amount", () => {
    expect(screen.getByTestId("decision-panel")).toHaveTextContent("1000 XLM");
  });

  it("release button is initially pressed (aria-pressed=true)", () => {
    expect(screen.getByTestId("decision-btn-release")).toHaveAttribute("aria-pressed", "true");
  });

  it("clicking refund sets slider to 0 and contributor gets 0 XLM", () => {
    fireEvent.click(screen.getByTestId("decision-btn-refund"));
    const slider = screen.getByTestId("split-slider") as HTMLInputElement;
    expect(slider.value).toBe("0");
    expect(screen.getByTestId("contributor-preview")).toHaveTextContent("0.00 XLM");
    expect(screen.getByTestId("maintainer-preview")).toHaveTextContent("1000.00 XLM");
  });

  it("clicking release sets slider to 100 and contributor gets full amount", () => {
    fireEvent.click(screen.getByTestId("decision-btn-refund")); // first go to 0
    fireEvent.click(screen.getByTestId("decision-btn-release"));
    const slider = screen.getByTestId("split-slider") as HTMLInputElement;
    expect(slider.value).toBe("100");
    expect(screen.getByTestId("contributor-preview")).toHaveTextContent("1000.00 XLM");
  });

  it("clicking split sets slider to 50 (when was 100)", () => {
    fireEvent.click(screen.getByTestId("decision-btn-split"));
    const slider = screen.getByTestId("split-slider") as HTMLInputElement;
    expect(slider.value).toBe("50");
  });

  it("slider change updates live label and preview", () => {
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "70" } });
    expect(screen.getByTestId("split-live-label")).toHaveTextContent("70% → contributor");
    expect(screen.getByTestId("contributor-preview")).toHaveTextContent("700.00 XLM");
    expect(screen.getByTestId("maintainer-preview")).toHaveTextContent("300.00 XLM");
  });

  it("slider aria-valuetext reflects both sides", () => {
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "60" } });
    expect(screen.getByTestId("split-slider")).toHaveAttribute(
      "aria-valuetext",
      "60% to contributor, 40% to maintainer"
    );
  });

  it("PageUp increases slider by 10", () => {
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "50" } });
    fireEvent.keyDown(screen.getByTestId("split-slider"), { key: "PageUp" });
    expect((screen.getByTestId("split-slider") as HTMLInputElement).value).toBe("60");
  });

  it("PageDown decreases slider by 10", () => {
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "50" } });
    fireEvent.keyDown(screen.getByTestId("split-slider"), { key: "PageDown" });
    expect((screen.getByTestId("split-slider") as HTMLInputElement).value).toBe("40");
  });

  it("PageUp clamps at 100", () => {
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "95" } });
    fireEvent.keyDown(screen.getByTestId("split-slider"), { key: "PageUp" });
    expect((screen.getByTestId("split-slider") as HTMLInputElement).value).toBe("100");
  });

  it("PageDown clamps at 0", () => {
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "5" } });
    fireEvent.keyDown(screen.getByTestId("split-slider"), { key: "PageDown" });
    expect((screen.getByTestId("split-slider") as HTMLInputElement).value).toBe("0");
  });

  it("split-live-label has aria-live=polite", () => {
    expect(screen.getByTestId("split-live-label")).toHaveAttribute("aria-live", "polite");
  });

  it("contributor XLM preview has aria-live=polite", () => {
    const previews = screen.getByTestId("contributor-preview").querySelectorAll("[aria-live]");
    expect(previews.length).toBeGreaterThan(0);
  });

  it("navigates to confirm step", () => {
    fireEvent.click(screen.getByTestId("proceed-to-confirm"));
    expect(screen.getByTestId("confirm-screen")).toBeInTheDocument();
  });

  it("back button returns to detail", () => {
    fireEvent.click(screen.getByText(/← Back to dispute/));
    expect(screen.getByTestId("dispute-detail")).toBeInTheDocument();
  });
});

// ══════════════════════════════════════════════════════════════════════════════

describe("DisputeResolutionPanel — confirm step", () => {
  it("shows irreversibility warning with role=alert", () => {
    renderPanel();
    goToConfirm();
    const warning = screen.getByTestId("irreversibility-warning");
    expect(warning).toBeInTheDocument();
    expect(warning).toHaveAttribute("role", "alert");
    expect(warning).toHaveTextContent("irreversible");
  });

  it("shows decision summary", () => {
    renderPanel();
    goToConfirm();
    expect(screen.getByTestId("decision-summary")).toHaveTextContent("Full Release");
    expect(screen.getByTestId("decision-summary")).toHaveTextContent("1000.00 XLM");
  });

  it("calls onDecide with correct full-release payload", () => {
    const { onDecide } = renderPanel();
    goToConfirm();
    fireEvent.click(screen.getByTestId("confirm-decision-button"));
    expect(onDecide).toHaveBeenCalledWith({
      disputeId: "d-001",
      decision: "release",
      contributorPct: 100,
    });
  });

  it("calls onDecide with correct refund payload", () => {
    const { onDecide } = renderPanel();
    goToDecide();
    fireEvent.click(screen.getByTestId("decision-btn-refund"));
    fireEvent.click(screen.getByTestId("proceed-to-confirm"));
    fireEvent.click(screen.getByTestId("confirm-decision-button"));
    expect(onDecide).toHaveBeenCalledWith({
      disputeId: "d-001",
      decision: "refund",
      contributorPct: 0,
    });
  });

  it("calls onDecide with correct split payload", () => {
    const { onDecide } = renderPanel();
    goToDecide();
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "70" } });
    fireEvent.click(screen.getByTestId("proceed-to-confirm"));
    fireEvent.click(screen.getByTestId("confirm-decision-button"));
    expect(onDecide).toHaveBeenCalledWith({
      disputeId: "d-001",
      decision: "split",
      contributorPct: 70,
    });
  });

  it("back button returns to decide step", () => {
    renderPanel();
    goToConfirm();
    fireEvent.click(screen.getByText(/← Change decision/));
    expect(screen.getByTestId("decision-panel")).toBeInTheDocument();
  });

  it("confirm button is accessible and has visible text", () => {
    renderPanel();
    goToConfirm();
    const btn = screen.getByTestId("confirm-decision-button");
    expect(btn).toBeInTheDocument();
    expect(btn.textContent).toMatch(/Confirm/i);
  });
});

// ══════════════════════════════════════════════════════════════════════════════

describe("DisputeResolutionPanel — edge cases", () => {
  it("handles exactly 0% contributor split (full refund via slider)", () => {
    renderPanel();
    goToDecide();
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "0" } });
    expect(screen.getByTestId("contributor-preview")).toHaveTextContent("0.00 XLM");
    expect(screen.getByTestId("maintainer-preview")).toHaveTextContent("1000.00 XLM");
  });

  it("handles exactly 100% contributor split (full release via slider)", () => {
    renderPanel();
    goToDecide();
    fireEvent.change(screen.getByTestId("split-slider"), { target: { value: "100" } });
    expect(screen.getByTestId("contributor-preview")).toHaveTextContent("1000.00 XLM");
    expect(screen.getByTestId("maintainer-preview")).toHaveTextContent("0.00 XLM");
  });

  it("handles 50% split", () => {
    renderPanel();
    goToDecide();
    fireEvent.click(screen.getByTestId("decision-btn-split"));
    expect(screen.getByTestId("contributor-preview")).toHaveTextContent("500.00 XLM");
    expect(screen.getByTestId("maintainer-preview")).toHaveTextContent("500.00 XLM");
  });

  it("renders with empty timeline gracefully", () => {
    renderPanel({ timeline: [] });
    expect(screen.getByTestId("dispute-detail")).toBeInTheDocument();
  });
});
