import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FilterChip } from "../FilterChip";

describe("FilterChip", () => {
  it("renders the label", () => {
    render(<FilterChip label="TypeScript" onRemove={vi.fn()} isDark={false} />);
    expect(screen.getByText("TypeScript")).toBeTruthy();
  });

  it("exposes an accessible remove button", () => {
    render(<FilterChip label="TypeScript" onRemove={vi.fn()} isDark={false} />);
    expect(screen.getByRole("button", { name: "Remove TypeScript filter" })).toBeTruthy();
  });

  it("calls onRemove when the remove button is clicked", async () => {
    const onRemove = vi.fn();
    const user = userEvent.setup();
    render(<FilterChip label="TypeScript" onRemove={onRemove} isDark={false} />);

    await user.click(screen.getByRole("button", { name: "Remove TypeScript filter" }));
    expect(onRemove).toHaveBeenCalledTimes(1);
  });

  it("calls onRemove on Backspace when the remove button is focused", async () => {
    const onRemove = vi.fn();
    const user = userEvent.setup();
    render(<FilterChip label="TypeScript" onRemove={onRemove} isDark={false} />);

    const button = screen.getByRole("button", { name: "Remove TypeScript filter" });
    button.focus();
    await user.keyboard("{Backspace}");
    expect(onRemove).toHaveBeenCalledTimes(1);
  });

  it("calls onRemove on Delete when the remove button is focused", async () => {
    const onRemove = vi.fn();
    const user = userEvent.setup();
    render(<FilterChip label="TypeScript" onRemove={onRemove} isDark={false} />);

    const button = screen.getByRole("button", { name: "Remove TypeScript filter" });
    button.focus();
    await user.keyboard("{Delete}");
    expect(onRemove).toHaveBeenCalledTimes(1);
  });

  it("does not call onRemove on unrelated keys", async () => {
    const onRemove = vi.fn();
    const user = userEvent.setup();
    render(<FilterChip label="TypeScript" onRemove={onRemove} isDark={false} />);

    const button = screen.getByRole("button", { name: "Remove TypeScript filter" });
    button.focus();
    await user.keyboard("{ArrowLeft}");
    expect(onRemove).not.toHaveBeenCalled();
  });

  it("exposes the remove button DOM node via buttonRef", () => {
    let captured: HTMLButtonElement | null = null;
    render(
      <FilterChip
        label="TypeScript"
        onRemove={vi.fn()}
        isDark={false}
        buttonRef={(el) => {
          captured = el;
        }}
      />
    );
    expect(captured).toBeInstanceOf(HTMLButtonElement);
  });
});
