import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProgramModerationDrawer } from "../ProgramModerationDrawer";
import type { FlaggedProgram } from "../types";

const mockProgram: FlaggedProgram = {
  id: "prog-1",
  name: "test-program",
  description: "A test program for moderation",
  severity: "high",
  flagCount: 5,
  riskScore: 74,
  status: "pending",
  programUrl: "https://github.com/example/test",
  owner: "test_owner",
  createdAt: "2026-01-15T08:00:00Z",
  reportedBy: "security-bot",
  reportedAt: "2026-06-25T06:00:00Z",
  reason: "Security vulnerability detected",
  flagHistory: [
    {
      id: "flag-1",
      reason: "Critical vulnerability in dependency",
      reportedBy: "dependabot",
      reportedAt: "2026-06-25T06:00:00Z",
      severity: "high",
      automated: true,
    },
    {
      id: "flag-2",
      reason: "Community report of suspicious behavior",
      reportedBy: "trusted_user",
      reportedAt: "2026-06-24T10:00:00Z",
      severity: "medium",
      evidenceUrl: "https://example.com/evidence",
      automated: false,
    },
  ],
};

describe("ProgramModerationDrawer", () => {
  it("renders program details when open", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    expect(screen.getByText("test-program")).toBeTruthy();
    expect(screen.getByText("A test program for moderation")).toBeTruthy();
  });

  it("returns null when not open", () => {
    const { container } = render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={false}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    expect(container.innerHTML).toBe("");
  });

  it("returns null when program is null", () => {
    const { container } = render(
      <ProgramModerationDrawer
        program={null}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    expect(container.innerHTML).toBe("");
  });

  it("renders flag history entries", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    expect(screen.getByText("Flag History (2)")).toBeTruthy();
    expect(screen.getByText("Critical vulnerability in dependency")).toBeTruthy();
    expect(screen.getByText("Community report of suspicious behavior")).toBeTruthy();
  });

  it("shows evidence link when available", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    const evidenceLinks = screen.getAllByText("Evidence");
    expect(evidenceLinks.length).toBeGreaterThanOrEqual(1);
  });

  it("renders three action buttons", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    expect(screen.getByText("Send Warning")).toBeTruthy();
    expect(screen.getByText("Pause Program")).toBeTruthy();
    expect(screen.getByText("Terminate Program")).toBeTruthy();
  });

  it("shows inline confirmation when warning action is clicked", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    fireEvent.click(screen.getByText("Send Warning"));
    expect(screen.getByText("Confirm Send Warning")).toBeTruthy();
    expect(screen.getByText("Cancel")).toBeTruthy();
  });

  it("calls onAction when action is confirmed", () => {
    const onAction = vi.fn();
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={onAction}
      />
    );
    fireEvent.click(screen.getByText("Send Warning"));
    fireEvent.click(screen.getByText("Confirm Send Warning"));
    expect(onAction).toHaveBeenCalledWith("warn", "prog-1");
  });

  it("has accessible dialog attributes", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-label")).toContain("test-program");
  });

  it("calls onClose when close button is clicked", () => {
    const onClose = vi.fn();
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={onClose}
        onAction={vi.fn()}
      />
    );
    fireEvent.click(screen.getByLabelText("Close drawer"));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("shows automated flag badge", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    const automatedBadges = screen.getAllByText("Automated");
    expect(automatedBadges.length).toBeGreaterThanOrEqual(1);
  });

  it("renders view program link", () => {
    render(
      <ProgramModerationDrawer
        program={mockProgram}
        open={true}
        onClose={vi.fn()}
        onAction={vi.fn()}
      />
    );
    const link = screen.getByText("View Program");
    expect(link.getAttribute("href")).toBe("https://github.com/example/test");
    expect(link.getAttribute("target")).toBe("_blank");
    expect(link.getAttribute("rel")).toBe("noopener noreferrer");
  });
});
