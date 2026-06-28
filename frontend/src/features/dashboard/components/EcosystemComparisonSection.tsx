/**
 * EcosystemComparisonSection
 *
 * NatSpec-style design contract for the EcosystemsPage comparison UI.
 * This component is a contract placeholder and documents the expected
 * ecosystem selection, side-by-side metrics table, and accessible delta indicators.
 *
 * Design Spec: design/specs/ecosystem-comparison-view.md
 */
import type { FC } from 'react';

/**
 * The base metrics required for side-by-side ecosystem comparison.
 */
export interface EcosystemComparisonItem {
  /** Unique ecosystem identifier. */
  id: string;
  /** Ecosystem display name. */
  name: string;
  /** Optional ecosystem logo URL. */
  logo_url?: string | null;
  /** Ecosystem status label. */
  status: 'active' | 'inactive' | 'archived';
  /** Currently active bounties. */
  active_bounties: number;
  /** Total reward amount. */
  total_rewards: number;
  /** Contributor count. */
  contributor_count: number;
  /** Year-over-year growth percentage. */
  yoy_growth_percent: number;
}

export interface EcosystemComparisonSectionProps {
  /** The ecosystems available for comparison. */
  ecosystems: EcosystemComparisonItem[];
  /** The ids of currently selected ecosystems. */
  selectedEcosystemIds: string[];
  /** Toggle selection of an ecosystem. */
  onToggleSelection: (ecosystemId: string) => void;
  /** Trigger open comparison view action. */
  onOpenComparison: () => void;
  /** Maximum number of selectable ecosystems. */
  maxSelection?: number;
}

/**
 * Placeholder render contract for the ecosystem comparison UI.
 * Implementation should support selecting up to three ecosystems and rendering
 * a side-by-side comparison table with delta indicators and mobile horizontal scrolling.
 */
export const EcosystemComparisonSection: FC<EcosystemComparisonSectionProps> = () => {
  return null;
};
