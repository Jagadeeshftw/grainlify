import { useState } from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActiveFilterChips } from "../ActiveFilterChips";

describe("ActiveFilterChips", () => {
  it("renders nothing when there are no active filters", () => {
    const { container } = render(
      <ActiveFilterChips filters={[]} onRemove={vi.fn()} isDark={false} />
    );
    expect(container.innerHTML).toBe("");
  });

  it("renders a chip per active filter", () => {
    render(
      <ActiveFilterChips filters={["TypeScript", "Rust"]} onRemove={vi.fn()} isDark={false} />
    );
    expect(screen.getByText("TypeScript")).toBeTruthy();
    expect(screen.getByText("Rust")).toBeTruthy();
  });

  it("renders the row as an accessible list", () => {
    render(
      <ActiveFilterChips
        filters={["TypeScript"]}
        onRemove={vi.fn()}
        isDark={false}
        ariaLabel="Active language filters"
      />
    );
    expect(screen.getByRole("list", { name: "Active language filters" })).toBeTruthy();
  });

  it("calls onRemove with the removed filter", async () => {
    const onRemove = vi.fn();
    const user = userEvent.setup();
    render(
      <ActiveFilterChips filters={["TypeScript", "Rust"]} onRemove={onRemove} isDark={false} />
    );

    await user.click(screen.getByRole("button", { name: "Remove TypeScript filter" }));
    expect(onRemove).toHaveBeenCalledWith("TypeScript");
  });

  it("collapses into a '+N more' chip beyond maxVisible", () => {
    render(
      <ActiveFilterChips
        filters={["A", "B", "C", "D"]}
        onRemove={vi.fn()}
        isDark={false}
        maxVisible={2}
      />
    );
    expect(screen.getByText("A")).toBeTruthy();
    expect(screen.getByText("B")).toBeTruthy();
    expect(screen.queryByText("C")).toBeNull();
    expect(screen.getByRole("button", { name: "Show 2 more active filters" })).toBeTruthy();
  });

  it("expands to show all chips and offers 'Show less'", async () => {
    const user = userEvent.setup();
    render(
      <ActiveFilterChips
        filters={["A", "B", "C", "D"]}
        onRemove={vi.fn()}
        isDark={false}
        maxVisible={2}
      />
    );

    await user.click(screen.getByRole("button", { name: "Show 2 more active filters" }));
    expect(screen.getByText("C")).toBeTruthy();
    expect(screen.getByText("D")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Show less" })).toBeTruthy();
  });

  it("moves focus to the chip now at the removed index after removal", async () => {
    const user = userEvent.setup();
    const filters = ["A", "B", "C"];

    function Wrapper() {
      const [active, setActive] = useState<string[]>(filters);
      return (
        <ActiveFilterChips
          filters={active}
          onRemove={(f: string) => setActive((prev: string[]) => prev.filter((v: string) => v !== f))}
          isDark={false}
        />
      );
    }

    render(<Wrapper />);
    await user.click(screen.getByRole("button", { name: "Remove B filter" }));

    expect(screen.getByRole("button", { name: "Remove C filter" })).toBe(document.activeElement);
  });

  it("calls onAllRemoved when the last chip is removed", async () => {
    const onAllRemoved = vi.fn();
    const user = userEvent.setup();

    function Wrapper() {
      const [active, setActive] = useState<string[]>(["A"]);
      return (
        <ActiveFilterChips
          filters={active}
          onRemove={(f: string) => setActive((prev: string[]) => prev.filter((v: string) => v !== f))}
          isDark={false}
          onAllRemoved={onAllRemoved}
        />
      );
    }

    render(<Wrapper />);
    await user.click(screen.getByRole("button", { name: "Remove A filter" }));

    expect(onAllRemoved).toHaveBeenCalledTimes(1);
  });
});
