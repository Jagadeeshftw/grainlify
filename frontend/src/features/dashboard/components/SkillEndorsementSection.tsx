/**
 * SkillEndorsementSection
 *
 * NatSpec-style design contract for the ProfilePage skill endorsement UI.
 * This component is a contract placeholder and documents the expected
 * data shape, interaction states, and accessibility requirements.
 *
 * Design Spec: design/specs/skill-endorsement-ui.md
 */
import type { FC } from 'react';

/**
 * A summary of a contributor skill item.
 */
export interface ProfileSkill {
  /** Unique skill identifier used for API operations. */
  id: string;
  /** Skill display label, e.g. "Rust", "Soroban", "React". */
  name: string;
  /** Total number of endorsements for this skill. */
  endorsement_count: number;
  /** True when this skill is one of the member's top skills. */
  is_top_skill: boolean;
  /** True when the currently viewing user has already endorsed it. */
  is_endorsed_by_viewer: boolean;
  /** True when the profile owner explicitly added this skill. */
  is_owner_skill: boolean;
}

/**
 * A skill endorser entry shown in the full skills modal.
 */
export interface SkillEndorser {
  id: string;
  login: string;
  avatar_url?: string;
  endorsement_date: string; // ISO 8601
}

export interface SkillEndorsementSectionProps {
  /** Profile owner's skills and endorsement metadata. */
  skills: ProfileSkill[];
  /** Viewer identity used to compute endorsement availability. */
  viewerId?: string | null;
  /** Triggered when the viewer endorses a skill. */
  onEndorse: (skillId: string) => Promise<void>;
  /** Triggered when the owner adds a new skill. */
  onAddSkill?: (skillName: string) => Promise<void>;
  /** Opens the full skills modal. */
  onOpenSkillsModal: () => void;
}

/**
 * Placeholder render contract for ProfilePage skill endorsement UI.
 * Implementation should be fully keyboard-accessible, allow endorse state toggle,
 * show top 5 skills with counts, and include an add skill affordance for owners.
 */
export const SkillEndorsementSection: FC<SkillEndorsementSectionProps> = () => {
  return null;
};
