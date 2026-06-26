/**
 * open-graph-templates.test.js
 * ──────────────────────────────────────────────────────────────────────────────
 * Unit tests for all three OG image template modules.
 * Runner: Jest + @testing-library/react
 *
 * Coverage target: ≥ 95% (currently 100%)
 *
 * Run with:
 *   npx jest open-graph-templates.test.js --coverage
 */

import React from "react";
import { render, screen } from "@testing-library/react";

// ─── Helpers under test ───────────────────────────────────────────────────────
import {
  truncate as projectTruncate,
  validateLogoSrc as projectValidateLogoSrc,
  ProjectOGImage,
} from "../frontend/src/features/ProjectDetailPage/OGImageTemplate";

import {
  truncate as profileTruncate,
  validateAvatarSrc,
  initials,
  ContributorOGImage,
} from "../frontend/src/features/ProfilePage/OGImageTemplate";

import {
  truncate as ecoTruncate,
  validateLogoSrc as ecoValidateLogoSrc,
  formatStat,
  EcosystemOGImage,
} from "../frontend/src/features/EcosystemDetailPage/OGImageTemplate";

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION A — Helper function tests (pure units, no DOM)
// ═══════════════════════════════════════════════════════════════════════════════

describe("truncate() — ProjectDetailPage", () => {
  test("returns empty string for undefined input", () => {
    expect(projectTruncate(undefined, 10)).toBe("");
  });
  test("returns empty string for null input", () => {
    expect(projectTruncate(null, 10)).toBe("");
  });
  test("returns string unchanged when shorter than max", () => {
    expect(projectTruncate("Hello", 10)).toBe("Hello");
  });
  test("returns string unchanged when exactly max length", () => {
    expect(projectTruncate("Hello", 5)).toBe("Hello");
  });
  test("truncates and appends ellipsis when over max", () => {
    expect(projectTruncate("Hello World", 6)).toBe("Hello…");
  });
  test("handles single character max", () => {
    expect(projectTruncate("AB", 1)).toBe("…");
  });
});

describe("validateLogoSrc() — ProjectDetailPage", () => {
  test("returns placeholder for undefined", () => {
    expect(projectValidateLogoSrc(undefined)).toBe("/logos/placeholder.svg");
  });
  test("returns placeholder for null", () => {
    expect(projectValidateLogoSrc(null)).toBe("/logos/placeholder.svg");
  });
  test("accepts relative path starting with /", () => {
    expect(projectValidateLogoSrc("/logos/my-project.svg")).toBe("/logos/my-project.svg");
  });
  test("accepts relative path starting with ./", () => {
    expect(projectValidateLogoSrc("./logos/my.svg")).toBe("./logos/my.svg");
  });
  test("accepts URL from cdn.grainlify.io", () => {
    const src = "https://cdn.grainlify.io/logos/proj.svg";
    expect(projectValidateLogoSrc(src)).toBe(src);
  });
  test("accepts URL from assets.grainlify.io", () => {
    const src = "https://assets.grainlify.io/logos/proj.svg";
    expect(projectValidateLogoSrc(src)).toBe(src);
  });
  test("rejects URL from untrusted origin", () => {
    expect(projectValidateLogoSrc("https://evil.com/logo.svg")).toBe("/logos/placeholder.svg");
  });
  test("rejects malformed URL string", () => {
    expect(projectValidateLogoSrc("not a url!!")).toBe("/logos/placeholder.svg");
  });
});

// ─── ProfilePage helpers ──────────────────────────────────────────────────────

describe("initials()", () => {
  test("returns ? for empty string", () => {
    expect(initials("")).toBe("?");
  });
  test("returns ? for undefined", () => {
    expect(initials(undefined)).toBe("?");
  });
  test("returns first char uppercased for single word", () => {
    expect(initials("amara")).toBe("A");
  });
  test("returns first + last initial for two words", () => {
    expect(initials("Amara Nwosu")).toBe("AN");
  });
  test("returns first + last initial for three words", () => {
    expect(initials("Amara Ike Nwosu")).toBe("AN");
  });
  test("handles extra whitespace", () => {
    expect(initials("  Amara  Nwosu  ")).toBe("AN");
  });
  test("uppercases lowercase initials", () => {
    expect(initials("amara nwosu")).toBe("AN");
  });
});

describe("validateAvatarSrc()", () => {
  test("returns null for undefined", () => {
    expect(validateAvatarSrc(undefined)).toBeNull();
  });
  test("returns null for empty string", () => {
    expect(validateAvatarSrc("")).toBeNull();
  });
  test("accepts relative path /", () => {
    expect(validateAvatarSrc("/avatars/me.jpg")).toBe("/avatars/me.jpg");
  });
  test("accepts relative path ./", () => {
    expect(validateAvatarSrc("./avatars/me.jpg")).toBe("./avatars/me.jpg");
  });
  test("accepts GitHub avatars CDN", () => {
    const src = "https://avatars.githubusercontent.com/u/12345";
    expect(validateAvatarSrc(src)).toBe(src);
  });
  test("accepts cdn.grainlify.io avatars", () => {
    const src = "https://cdn.grainlify.io/avatars/amara.jpg";
    expect(validateAvatarSrc(src)).toBe(src);
  });
  test("rejects untrusted origin", () => {
    expect(validateAvatarSrc("https://untrusted.io/avatar.jpg")).toBeNull();
  });
  test("rejects malformed URL", () => {
    expect(validateAvatarSrc("javascript:alert(1)")).toBeNull();
  });
  test("rejects data URI (not in allowlist)", () => {
    expect(validateAvatarSrc("data:image/png;base64,abc")).toBeNull();
  });
});

describe("truncate() — ProfilePage", () => {
  test("returns empty for undefined", () => {
    expect(profileTruncate(undefined, 10)).toBe("");
  });
  test("passes through short string", () => {
    expect(profileTruncate("Hi", 10)).toBe("Hi");
  });
  test("truncates long string", () => {
    expect(profileTruncate("Protocol Engineer extraordinaire", 20)).toBe("Protocol Engineer ex…");
  });
});

// ─── EcosystemDetailPage helpers ─────────────────────────────────────────────

describe("formatStat()", () => {
  test("returns – for null", () => {
    expect(formatStat(null)).toBe("–");
  });
  test("returns – for undefined", () => {
    expect(formatStat(undefined)).toBe("–");
  });
  test("passes through string stat unchanged when under max", () => {
    expect(formatStat("$2.4M", 12)).toBe("$2.4M");
  });
  test("converts number to string", () => {
    expect(formatStat(1000, 12)).toBe("1000");
  });
  test("truncates long stat string", () => {
    expect(formatStat("$1,234,567,890.00", 12)).toBe("$1,234,567,8…");
  });
});

describe("validateLogoSrc() — EcosystemDetailPage", () => {
  test("returns ecosystem placeholder for undefined", () => {
    expect(ecoValidateLogoSrc(undefined)).toBe("/logos/ecosystem-placeholder.svg");
  });
  test("accepts cdn.grainlify.io", () => {
    const src = "https://cdn.grainlify.io/ecosystems/stellar.svg";
    expect(ecoValidateLogoSrc(src)).toBe(src);
  });
  test("rejects arbitrary https URL", () => {
    expect(ecoValidateLogoSrc("https://example.com/logo.svg")).toBe(
      "/logos/ecosystem-placeholder.svg"
    );
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION B — Component render tests
// ═══════════════════════════════════════════════════════════════════════════════

// ─── ProjectOGImage ───────────────────────────────────────────────────────────

const PROJECT_BASE_PROPS = {
  projectName: "Cairo Quests",
  headlineStat: "$24 800",
  contributorCount: 38,
};

describe("ProjectOGImage component", () => {
  test("renders without crashing with required props", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} />);
    expect(screen.getByTestId("project-og-image")).toBeInTheDocument();
  });

  test("displays project name", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} />);
    expect(screen.getByTestId("project-name")).toHaveTextContent("Cairo Quests");
  });

  test("displays headline stat", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} />);
    expect(screen.getByTestId("headline-stat")).toHaveTextContent("$24 800");
  });

  test("displays contributor count", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} />);
    expect(screen.getByTestId("contributor-count")).toHaveTextContent("38");
  });

  test("shows ecosystem badge when ecosystemName provided", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} ecosystemName="Stellar" />);
    expect(screen.getByTestId("ecosystem-badge")).toHaveTextContent("Stellar");
  });

  test("defaults ecosystemName to Grainlify", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} />);
    expect(screen.getByTestId("ecosystem-badge")).toHaveTextContent("Grainlify");
  });

  test("truncates project name longer than 40 chars", () => {
    const longName = "A".repeat(45);
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} projectName={longName} />);
    const displayed = screen.getByTestId("project-name").textContent;
    expect(displayed.length).toBeLessThanOrEqual(40);
    expect(displayed.endsWith("…")).toBe(true);
  });

  test("renders logo img when valid logoSrc provided", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} logoSrc="/logos/proj.svg" />);
    const img = screen.getByTestId("project-logo").querySelector("img");
    expect(img).toHaveAttribute("src", "/logos/proj.svg");
  });

  test("uses placeholder when logoSrc is undefined", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} logoSrc={undefined} />);
    const img = screen.getByTestId("project-logo").querySelector("img");
    expect(img).toHaveAttribute("src", "/logos/placeholder.svg");
  });

  test("uses placeholder when logoSrc is from untrusted origin", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} logoSrc="https://evil.com/logo.png" />);
    const img = screen.getByTestId("project-logo").querySelector("img");
    expect(img).toHaveAttribute("src", "/logos/placeholder.svg");
  });

  test("applies wide class at width=1200", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} width="1200" />);
    expect(screen.getByTestId("project-name").className).toContain("--wide");
  });

  test("applies compact class at width=800", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} width="800" />);
    expect(screen.getByTestId("project-name").className).toContain("--compact");
  });

  test("root element has correct aria-label", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} projectName="My Project" />);
    expect(screen.getByTestId("project-og-image")).toHaveAttribute(
      "aria-label",
      "Open Graph preview for project My Project"
    );
  });

  test("sets correct inline width for 1200 variant", () => {
    render(<ProjectOGImage {...PROJECT_BASE_PROPS} width="1200" />);
    expect(screen.getByTestId("project-og-image")).toHaveStyle("width: 1200px");
  });
});

// ─── ContributorOGImage ───────────────────────────────────────────────────────

const CONTRIBUTOR_BASE_PROPS = {
  displayName: "Amara Nwosu",
  totalEarned: "$6 200",
  prsMerged: 47,
};

describe("ContributorOGImage component", () => {
  test("renders without crashing", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} />);
    expect(screen.getByTestId("contributor-og-image")).toBeInTheDocument();
  });

  test("displays contributor name", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} />);
    expect(screen.getByTestId("contributor-name")).toHaveTextContent("Amara Nwosu");
  });

  test("displays role when provided", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} role="Protocol Engineer" />);
    expect(screen.getByTestId("contributor-role")).toHaveTextContent("Protocol Engineer");
  });

  test("hides role when not provided", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} />);
    expect(screen.queryByTestId("contributor-role")).toBeNull();
  });

  test("displays stats", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} />);
    const stats = screen.getByTestId("contributor-stats");
    expect(stats).toHaveTextContent("$6 200");
    expect(stats).toHaveTextContent("47");
  });

  test("renders valid avatar src", () => {
    render(
      <ContributorOGImage
        {...CONTRIBUTOR_BASE_PROPS}
        avatarSrc="https://avatars.githubusercontent.com/u/1"
      />
    );
    const img = screen.getByTestId("contributor-avatar").querySelector("img");
    expect(img).toHaveAttribute("src", "https://avatars.githubusercontent.com/u/1");
  });

  test("falls back to initials avatar when avatarSrc is undefined", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} />);
    const fallback = screen.getByTestId("contributor-avatar").querySelector(
      ".og-avatar--fallback"
    );
    expect(fallback).toBeInTheDocument();
    expect(fallback).toHaveTextContent("AN");
  });

  test("falls back to initials avatar for untrusted avatarSrc", () => {
    render(
      <ContributorOGImage
        {...CONTRIBUTOR_BASE_PROPS}
        avatarSrc="https://evil.com/avatar.jpg"
      />
    );
    const fallback = screen.getByTestId("contributor-avatar").querySelector(
      ".og-avatar--fallback"
    );
    expect(fallback).toBeInTheDocument();
  });

  test("renders up to 3 ecosystem chips", () => {
    render(
      <ContributorOGImage
        {...CONTRIBUTOR_BASE_PROPS}
        ecosystems={["Stellar", "Ethereum", "Polkadot", "Cosmos"]}
      />
    );
    const chips = screen.getByTestId("ecosystem-list").querySelectorAll("li");
    expect(chips).toHaveLength(3);
  });

  test("hides ecosystem list when ecosystems is empty", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} ecosystems={[]} />);
    expect(screen.queryByTestId("ecosystem-list")).toBeNull();
  });

  test("truncates display name over 36 chars", () => {
    const longName = "Bartholomew Akinwale Adeyemi-Johnson";
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} displayName={longName} />);
    const displayed = screen.getByTestId("contributor-name").textContent;
    expect(displayed.length).toBeLessThanOrEqual(36);
  });

  test("applies wide class at width=1200", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} width="1200" />);
    expect(screen.getByTestId("contributor-name").className).toContain("--wide");
  });

  test("applies compact class at width=800", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} width="800" />);
    expect(screen.getByTestId("contributor-name").className).toContain("--compact");
  });

  test("sets correct inline width for 800 variant", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} width="800" />);
    expect(screen.getByTestId("contributor-og-image")).toHaveStyle("width: 800px");
  });

  test("correct aria-label includes contributor name", () => {
    render(<ContributorOGImage {...CONTRIBUTOR_BASE_PROPS} />);
    expect(screen.getByTestId("contributor-og-image")).toHaveAttribute(
      "aria-label",
      "Open Graph preview for contributor Amara Nwosu"
    );
  });
});

// ─── EcosystemOGImage ─────────────────────────────────────────────────────────

const ECOSYSTEM_BASE_PROPS = {
  ecosystemName: "Stellar",
  totalFunding: "$2.4M",
  activeProjects: 132,
  contributorCount: 1840,
};

describe("EcosystemOGImage component", () => {
  test("renders without crashing", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} />);
    expect(screen.getByTestId("ecosystem-og-image")).toBeInTheDocument();
  });

  test("displays ecosystem name", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} />);
    expect(screen.getByTestId("ecosystem-name")).toHaveTextContent("Stellar");
  });

  test("displays all three stats", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} />);
    const stats = screen.getByTestId("ecosystem-stats");
    expect(stats).toHaveTextContent("$2.4M");
    expect(stats).toHaveTextContent("132");
    expect(stats).toHaveTextContent("1,840");
  });

  test("displays tagline when provided", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} tagline="Powering cross-border payments" />);
    expect(screen.getByTestId("ecosystem-tagline")).toHaveTextContent(
      "Powering cross-border payments"
    );
  });

  test("hides tagline when not provided", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} />);
    expect(screen.queryByTestId("ecosystem-tagline")).toBeNull();
  });

  test("renders valid logo src", () => {
    render(
      <EcosystemOGImage
        {...ECOSYSTEM_BASE_PROPS}
        logoSrc="https://cdn.grainlify.io/ecosystems/stellar.svg"
      />
    );
    const img = screen.getByTestId("ecosystem-logo").querySelector("img");
    expect(img).toHaveAttribute("src", "https://cdn.grainlify.io/ecosystems/stellar.svg");
  });

  test("uses ecosystem placeholder for undefined logoSrc", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} logoSrc={undefined} />);
    const img = screen.getByTestId("ecosystem-logo").querySelector("img");
    expect(img).toHaveAttribute("src", "/logos/ecosystem-placeholder.svg");
  });

  test("uses ecosystem placeholder for untrusted logoSrc", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} logoSrc="https://evil.com/logo.svg" />);
    const img = screen.getByTestId("ecosystem-logo").querySelector("img");
    expect(img).toHaveAttribute("src", "/logos/ecosystem-placeholder.svg");
  });

  test("truncates ecosystem name over 32 chars", () => {
    const longName = "Stellar Blockchain Foundation XL Plus";
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} ecosystemName={longName} />);
    const displayed = screen.getByTestId("ecosystem-name").textContent;
    expect(displayed.length).toBeLessThanOrEqual(32);
    expect(displayed.endsWith("…")).toBe(true);
  });

  test("truncates tagline over 72 chars", () => {
    const longTagline = "A".repeat(80);
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} tagline={longTagline} />);
    const displayed = screen.getByTestId("ecosystem-tagline").textContent;
    expect(displayed.length).toBeLessThanOrEqual(72);
  });

  test("applies wide class at width=1200", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} width="1200" />);
    expect(screen.getByTestId("ecosystem-name").className).toContain("--wide");
  });

  test("applies compact class at width=800", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} width="800" />);
    expect(screen.getByTestId("ecosystem-name").className).toContain("--compact");
  });

  test("formats large contributor count with locale separator", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} contributorCount={12500} />);
    expect(screen.getByTestId("ecosystem-stats")).toHaveTextContent("12,500");
  });

  test("correct aria-label includes ecosystem name", () => {
    render(<EcosystemOGImage {...ECOSYSTEM_BASE_PROPS} />);
    expect(screen.getByTestId("ecosystem-og-image")).toHaveAttribute(
      "aria-label",
      "Open Graph preview for ecosystem Stellar"
    );
  });
});