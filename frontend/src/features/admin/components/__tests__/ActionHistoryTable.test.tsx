import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ActionHistoryTable } from "../ActionHistoryTable";
import type { AuditEntry } from "../types";

const mockEntries: AuditEntry[] = [
  { id: "a1", programName: "alpha", action: "warn", performedBy: "admin1", performedAt: "2026-06-25T10:00:00Z", reason: "Policy violation", programId: "p1" },
  { id: "a2", programName: "beta", action: "pause", performedBy: "admin2", performedAt: "2026-06-24T10:00:00Z", reason: "Security issue", programId: "p2" },
  { id: "a3", programName: "gamma", action: "terminate", performedBy: "admin1", performedAt: "2026-06-23T10:00:00Z", reason: "Malicious activity", programId: "p3" },
  { id: "a4", programName: "delta", action: "resolve", performedBy: "admin2", performedAt: "2026-06-22T10:00:00Z", reason: "Issue fixed", programId: "p4" },
];

describe("ActionHistoryTable", () => {
  it("renders all audit entries", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    expect(screen.getByText("alpha")).toBeTruthy();
    expect(screen.getByText("beta")).toBeTruthy();
    expect(screen.getByText("gamma")).toBeTruthy();
    expect(screen.getByText("delta")).toBeTruthy();
  });

  it("filters by action type", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    fireEvent.click(screen.getByText("Warn"));
    expect(screen.getByText("alpha")).toBeTruthy();
    expect(screen.queryByText("beta")).toBeNull();
  });

  it("searches by program name", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    const search = screen.getByLabelText("Search audit log");
    fireEvent.change(search, { target: { value: "alpha" } });
    expect(screen.getByText("alpha")).toBeTruthy();
    expect(screen.queryByText("beta")).toBeNull();
  });

  it("searches by performed by", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    const search = screen.getByLabelText("Search audit log");
    fireEvent.change(search, { target: { value: "admin1" } });
    expect(screen.getByText("alpha")).toBeTruthy();
    expect(screen.getByText("gamma")).toBeTruthy();
    expect(screen.queryByText("beta")).toBeNull();
  });

  it("shows empty state when no entries match filters", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    const search = screen.getByLabelText("Search audit log");
    fireEvent.change(search, { target: { value: "nonexistent" } });
    expect(screen.getByText("No audit entries found")).toBeTruthy();
  });

  it("sorts entries by date descending", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    const rows = screen.getAllByRole("row");
    expect(rows[1].textContent).toContain("alpha");
    expect(rows[2].textContent).toContain("beta");
  });

  it("renders action pills with correct labels", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    expect(screen.getByText("Warning")).toBeTruthy();
    expect(screen.getByText("Paused")).toBeTruthy();
    expect(screen.getByText("Terminated")).toBeTruthy();
    expect(screen.getByText("Resolved")).toBeTruthy();
  });

  it("has search input accessible label", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    expect(screen.getByLabelText("Search audit log")).toBeTruthy();
  });

  it("allows switching back to All filter", () => {
    render(<ActionHistoryTable entries={mockEntries} />);
    fireEvent.click(screen.getByText("Warn"));
    expect(screen.queryByText("beta")).toBeNull();
    fireEvent.click(screen.getByText("All"));
    expect(screen.getByText("beta")).toBeTruthy();
  });
});
