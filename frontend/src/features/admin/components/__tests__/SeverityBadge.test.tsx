import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SeverityBadge } from "../SeverityBadge";

describe("SeverityBadge", () => {
  it("renders low severity with correct label and icon", () => {
    render(<SeverityBadge severity="low" />);
    const badge = screen.getByRole("status");
    expect(badge).toBeTruthy();
    expect(badge.textContent).toContain("Low");
  });

  it("renders medium severity", () => {
    render(<SeverityBadge severity="medium" />);
    expect(screen.getByRole("status").textContent).toContain("Medium");
  });

  it("renders high severity", () => {
    render(<SeverityBadge severity="high" />);
    expect(screen.getByRole("status").textContent).toContain("High");
  });

  it("renders critical severity", () => {
    render(<SeverityBadge severity="critical" />);
    expect(screen.getByRole("status").textContent).toContain("Critical");
  });

  it("displays flag count when provided", () => {
    render(<SeverityBadge severity="high" count={5} />);
    const badge = screen.getByRole("status");
    expect(badge.textContent).toContain("5");
  });

  it("does not display count when not provided", () => {
    render(<SeverityBadge severity="low" />);
    const badge = screen.getByRole("status");
    expect(badge.textContent).not.toMatch(/\d+/);
  });

  it("has accessible aria-label with severity and count", () => {
    render(<SeverityBadge severity="critical" count={12} />);
    const badge = screen.getByRole("status");
    expect(badge.getAttribute("aria-label")).toContain("Critical");
    expect(badge.getAttribute("aria-label")).toContain("12");
  });

  it("applies custom className", () => {
    render(<SeverityBadge severity="medium" className="custom-class" />);
    const badge = screen.getByRole("status");
    expect(badge.className).toContain("custom-class");
  });
});
