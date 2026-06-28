import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { BulkActionToolbar } from "../BulkActionToolbar";

describe("BulkActionToolbar", () => {
  const defaultProps = {
    selectedCount: 3,
    totalCount: 10,
    onSelectAll: vi.fn(),
    onClearSelection: vi.fn(),
    onBulkAction: vi.fn(),
  };

  it("renders when items are selected", () => {
    render(<BulkActionToolbar {...defaultProps} />);
    expect(screen.getByText("3 of 10 selected")).toBeTruthy();
  });

  it("does not render when no items are selected", () => {
    const { container } = render(
      <BulkActionToolbar {...defaultProps} selectedCount={0} />
    );
    expect(container.innerHTML).toBe("");
  });

  it("shows all three action buttons", () => {
    render(<BulkActionToolbar {...defaultProps} />);
    expect(screen.getByText("Warn Selected")).toBeTruthy();
    expect(screen.getByText("Pause Selected")).toBeTruthy();
    expect(screen.getByText("Terminate Selected")).toBeTruthy();
  });

  it("calls onSelectAll when select all is clicked", () => {
    const onSelectAll = vi.fn();
    render(
      <BulkActionToolbar {...defaultProps} onSelectAll={onSelectAll} />
    );
    fireEvent.click(screen.getByText("Select all"));
    expect(onSelectAll).toHaveBeenCalledOnce();
  });

  it("calls onClearSelection when clear is clicked", () => {
    const onClearSelection = vi.fn();
    render(
      <BulkActionToolbar {...defaultProps} onClearSelection={onClearSelection} />
    );
    fireEvent.click(screen.getByText("Clear"));
    expect(onClearSelection).toHaveBeenCalledOnce();
  });

  it("shows select all text when not all selected", () => {
    render(<BulkActionToolbar {...defaultProps} />);
    expect(screen.getByText("Select all")).toBeTruthy();
  });

  it("shows deselect all when all items selected", () => {
    render(
      <BulkActionToolbar {...defaultProps} selectedCount={10} totalCount={10} />
    );
    expect(screen.getByText("Deselect all")).toBeTruthy();
  });

  it("has toolbar role", () => {
    render(<BulkActionToolbar {...defaultProps} />);
    expect(screen.getByRole("toolbar")).toBeTruthy();
  });

  it("opens confirmation dialog for terminate action", () => {
    render(<BulkActionToolbar {...defaultProps} />);
    fireEvent.click(screen.getByText("Terminate Selected"));
    expect(screen.getByText("Yes, Terminate")).toBeTruthy();
    expect(screen.getByText("Cancel")).toBeTruthy();
  });

  it("calls onBulkAction when terminate is confirmed", () => {
    const onBulkAction = vi.fn();
    render(
      <BulkActionToolbar {...defaultProps} onBulkAction={onBulkAction} />
    );
    fireEvent.click(screen.getByText("Terminate Selected"));
    fireEvent.click(screen.getByText("Yes, Terminate"));
    expect(onBulkAction).toHaveBeenCalledWith("terminate");
  });
});
