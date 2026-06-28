import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ModerationQueue } from "../ModerationQueue";
import type { FlaggedProgram } from "../types";

const mockPrograms: FlaggedProgram[] = [
  {
    id: "1", name: "alpha-project", description: "First test project",
    severity: "critical", flagCount: 10, reason: "Suspicious activity",
    reportedBy: "bot", reportedAt: "2026-06-25T10:00:00Z", riskScore: 95,
    status: "pending", programUrl: "https://example.com/alpha",
    owner: "user1", createdAt: "2026-01-01T00:00:00Z",
    flagHistory: [],
  },
  {
    id: "2", name: "beta-project", description: "Second test project",
    severity: "low", flagCount: 1, reason: "Minor issue",
    reportedBy: "user2", reportedAt: "2026-06-20T10:00:00Z", riskScore: 10,
    status: "pending", programUrl: "https://example.com/beta",
    owner: "user2", createdAt: "2026-02-01T00:00:00Z",
    flagHistory: [],
  },
  {
    id: "3", name: "gamma-project", description: "Third test project",
    severity: "high", flagCount: 5, reason: "Security concern",
    reportedBy: "security-bot", reportedAt: "2026-06-22T10:00:00Z", riskScore: 72,
    status: "pending", programUrl: "https://example.com/gamma",
    owner: "user3", createdAt: "2026-03-01T00:00:00Z",
    flagHistory: [],
  },
];

describe("ModerationQueue", () => {
  const defaultProps = {
    programs: mockPrograms,
    onSelect: vi.fn(),
    onSelectMultiple: vi.fn(),
    selectedIds: [] as string[],
  };

  it("renders all programs", () => {
    render(<ModerationQueue {...defaultProps} />);
    expect(screen.getByText("alpha-project")).toBeTruthy();
    expect(screen.getByText("beta-project")).toBeTruthy();
    expect(screen.getByText("gamma-project")).toBeTruthy();
  });

  it("filters by search text", () => {
    render(<ModerationQueue {...defaultProps} />);
    const search = screen.getByLabelText("Search flagged programs");
    fireEvent.change(search, { target: { value: "alpha" } });
    expect(screen.getByText("alpha-project")).toBeTruthy();
    expect(screen.queryByText("beta-project")).toBeNull();
  });

  it("filters by severity", () => {
    render(<ModerationQueue {...defaultProps} />);
    const criticalButton = screen.getByText("Critical");
    fireEvent.click(criticalButton);
    expect(screen.getByText("alpha-project")).toBeTruthy();
    expect(screen.queryByText("beta-project")).toBeNull();
  });

  it("sorts by severity by default with critical first", () => {
    render(<ModerationQueue {...defaultProps} />);
    const items = screen.getAllByRole("listitem");
    expect(items[0].textContent).toContain("alpha-project");
    expect(items[1].textContent).toContain("gamma-project");
    expect(items[2].textContent).toContain("beta-project");
  });

  it("calls onSelect when a program row is clicked", () => {
    const onSelect = vi.fn();
    render(<ModerationQueue {...defaultProps} onSelect={onSelect} />);
    const items = screen.getAllByRole("listitem");
    fireEvent.click(items[0]);
    expect(onSelect).toHaveBeenCalledWith("1");
  });

  it("shows empty state when no programs match filters", () => {
    render(<ModerationQueue {...defaultProps} />);
    const search = screen.getByLabelText("Search flagged programs");
    fireEvent.change(search, { target: { value: "nonexistent" } });
    expect(screen.getByText("No flagged programs found")).toBeTruthy();
  });

  it("shows severity badges for each program", () => {
    render(<ModerationQueue {...defaultProps} />);
    const badges = screen.getAllByRole("status");
    expect(badges.length).toBe(3);
  });

  it("handles checkbox selection", () => {
    const onSelectMultiple = vi.fn();
    render(<ModerationQueue {...defaultProps} onSelectMultiple={onSelectMultiple} />);
    const checkboxes = screen.getAllByRole("checkbox");
    fireEvent.click(checkboxes[0]);
    expect(onSelectMultiple).toHaveBeenCalledWith(["1"]);
  });
});
