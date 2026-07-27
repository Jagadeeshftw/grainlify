import { useState, type ReactElement } from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ThemeProvider } from "../../contexts/ThemeContext";
import { FilterDropdown } from "../FilterDropdown";

beforeEach(() => {
  localStorage.clear();
  document.documentElement.className = "";
});

afterEach(() => {
  localStorage.clear();
  document.documentElement.className = "";
});

function renderWithTheme(ui: ReactElement) {
  return render(<ThemeProvider>{ui}</ThemeProvider>);
}

describe("FilterDropdown — single-select (default)", () => {
  it("shows the label as the display value when value is 'all'", () => {
    renderWithTheme(
      <FilterDropdown label="Sort" options={["All", "Newest", "Oldest"]} value="all" onChange={vi.fn()} />
    );
    expect(screen.getByRole("button", { name: "Sort" })).toBeTruthy();
  });

  it("selects a single option and closes the dropdown", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    renderWithTheme(
      <FilterDropdown label="Sort" options={["Newest", "Oldest"]} value="Newest" onChange={onChange} />
    );

    await user.click(screen.getByRole("button", { name: /Newest/ }));
    await user.click(screen.getByRole("option", { name: "Oldest" }));

    expect(onChange).toHaveBeenCalledWith("Oldest");
  });
});

describe("FilterDropdown — multi-select", () => {
  function MultiSelectHarness() {
    const [values, setValues] = useState<string[]>([]);
    return (
      <FilterDropdown
        label="Languages"
        options={["TypeScript", "Rust", "Go"]}
        multiple
        value={values}
        onChange={setValues}
      />
    );
  }

  it("shows the selection count in the trigger once values are active", async () => {
    const user = userEvent.setup();
    renderWithTheme(<MultiSelectHarness />);

    await user.click(screen.getByRole("button", { name: "Languages" }));
    await user.click(screen.getByRole("option", { name: "TypeScript" }));

    expect(screen.getByRole("button", { name: "Languages (1)" })).toBeTruthy();
  });

  it("renders a removable chip for each selected value", async () => {
    const user = userEvent.setup();
    renderWithTheme(<MultiSelectHarness />);

    await user.click(screen.getByRole("button", { name: "Languages" }));
    await user.click(screen.getByRole("option", { name: "TypeScript" }));
    await user.click(screen.getByRole("option", { name: "Rust" }));

    expect(screen.getByRole("button", { name: "Remove TypeScript filter" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Remove Rust filter" })).toBeTruthy();
  });

  it("keeps the dropdown open after selecting an option", async () => {
    const user = userEvent.setup();
    renderWithTheme(<MultiSelectHarness />);

    await user.click(screen.getByRole("button", { name: "Languages" }));
    await user.click(screen.getByRole("option", { name: "TypeScript" }));

    expect(screen.getByRole("option", { name: "Rust" })).toBeTruthy();
  });

  it("removing the chip deselects the option", async () => {
    const user = userEvent.setup();
    renderWithTheme(<MultiSelectHarness />);

    await user.click(screen.getByRole("button", { name: "Languages" }));
    await user.click(screen.getByRole("option", { name: "TypeScript" }));
    await user.click(screen.getByRole("button", { name: "Remove TypeScript filter" }));

    expect(screen.getByRole("button", { name: "Languages" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Remove TypeScript filter" })).toBeNull();
  });
});
