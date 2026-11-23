export { ProgramEscrowClient, fetchAllPages } from "./program-escrow-client";
export { BountyEscrowClient } from "./bounty-escrow-client";
export type {
  BountyEscrowConfig,
  Escrow,
  EscrowStatus,
  LockFundsItem,
  PauseFlags,
  RefundApproval,
  RefundEligibility,
  RefundMode,
  RefundRecord,
  SetPausedOptions,
} from "./bounty-escrow-client";
export type {
  ProgramEscrowConfig,
  ProgramData,
  PayoutRecord,
  ProgramReleaseSchedule,
  PayoutQueryFilter,
  ScheduleQueryFilter,
} from "./program-escrow-client";

export {
  SDKError,
  ContractError,
  NetworkError,
  ValidationError,
  ContractErrorCode,
  createContractError,
  parseContractError,
  parseContractErrorByCode,
  getContractErrorMessage,
  BOUNTY_ESCROW_ERROR_MAP,
  GOVERNANCE_ERROR_MAP,
  CIRCUIT_BREAKER_ERROR_MAP,
} from './errors';
