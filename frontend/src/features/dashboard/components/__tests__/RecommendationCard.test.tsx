import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RecommendationCard } from "../RecommendationCard";

vi.mock("../../../shared/contexts/ThemeContext", () => ({
  useTheme: () => ({ theme: "light" }),
}));

describe("RecommendationCard", () => {
  it("renders rationale copy and accessible description wiring", () => {
    render(
      <RecommendationCard
        title="Rust contributor hub"
        description="A community-led place to find multi-language contribution work."
        rationale="Matches your Rust activity"
        eyebrow="Recommended project"
        variant="project-pick"
        tags={["Rust", "Sovereign"]}
        stats={[{ label: "Stars", value: "2.4K" }]}
        onClick={() => undefined}
      />,
    );

    const button = screen.getByRole("button", { name: /recommended project: rust contributor hub/i });
    expect(button).toHaveAttribute("aria-describedby");
    expect(screen.getByText(/why recommended: matches your rust activity/i)).toBeInTheDocument();
    expect(screen.getByText("Rust")).toBeInTheDocument();
  });
});
