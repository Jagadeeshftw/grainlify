//! Shared test harness for the governance integration tests.
//!
//! The governance entrypoints in `grainlify_core::governance::GovernanceContract`
//! are plain associated functions (the type is not a `#[contractimpl]` contract),
//! so they read and write instance storage. Soroban's host only permits storage
//! access from inside a contract frame, and each `require_auth` may only be
//! recorded once per frame.
//!
//! These wrappers open a fresh contract frame around every call, using a single
//! lazily-registered contract address per test `Env` so all calls share the same
//! storage. Test bodies can then be written exactly as before (`env.clone()`,
//! `&env`, ...), which keeps the suite readable.

#![allow(dead_code)]

use std::cell::RefCell;

use grainlify_core::governance::{
    self, Error, GovernanceConfig, GovernanceContract, PendingAdminRotation, ProposalStatus, Role,
    VoteType,
};
use grainlify_core::GrainlifyContract;
use soroban_sdk::{Address, BytesN, Env, Symbol};

thread_local! {
    /// Contract address backing the current test's governance storage.
    ///
    /// Reset at the start of every test via [`reset`], so tests running on the
    /// same harness thread never share storage.
    static CONTEXT: RefCell<Option<Address>> = const { RefCell::new(None) };
}

/// Clears the cached contract address. Call once at the start of every test.
pub fn reset() {
    CONTEXT.with(|cell| *cell.borrow_mut() = None);
}

/// Returns the contract address for the current test, registering one on first use.
fn ctx(env: &Env) -> Address {
    CONTEXT.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(address) = slot.as_ref() {
            return address.clone();
        }
        let address = env.register_contract(None, GrainlifyContract);
        *slot = Some(address.clone());
        address
    })
}

/// The contract address backing the current test's governance storage.
pub fn contract_address(env: &Env) -> Address {
    ctx(env)
}

pub fn current_contract_address(env: &Env) -> Address {
    let id = ctx(env);
    env.as_contract(&id, || env.current_contract_address())
}

pub fn init_governance_state(
    env: Env,
    admin: Address,
    config: GovernanceConfig,
) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::init_governance_state(env.clone(), admin, config)
    })
}

pub fn rotate_admin(env: Env, current_admin: Address, new_admin: Address) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::rotate_admin(env.clone(), current_admin, new_admin)
    })
}

pub fn confirm_admin_rotation(env: Env, new_admin: Address) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::confirm_admin_rotation(env.clone(), new_admin)
    })
}

pub fn set_emergency_role(env: Env, admin: Address, new_holder: Address) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::set_emergency_role(env.clone(), admin, new_holder)
    })
}

pub fn set_upgrade_role(env: Env, admin: Address, new_holder: Address) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::set_upgrade_role(env.clone(), admin, new_holder)
    })
}

pub fn set_config_role(env: Env, admin: Address, new_holder: Address) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::set_config_role(env.clone(), admin, new_holder)
    })
}

pub fn set_security_council(
    env: Env,
    admin: Address,
    security_council: Address,
) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::set_security_council(env.clone(), admin, security_council)
    })
}

pub fn emergency_pause(env: Env, caller: Address) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::emergency_pause(env.clone(), caller)
    })
}

pub fn emergency_unpause(env: Env, caller: Address) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::emergency_unpause(env.clone(), caller)
    })
}

pub fn update_governance_config(
    env: Env,
    caller: Address,
    new_voting_period: u64,
    new_execution_delay: u64,
    new_quorum_percentage: u32,
    new_approval_threshold: u32,
    new_min_proposal_stake: i128,
) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::update_governance_config(
            env.clone(),
            caller,
            new_voting_period,
            new_execution_delay,
            new_quorum_percentage,
            new_approval_threshold,
            new_min_proposal_stake,
        )
    })
}

pub fn create_proposal(
    env: Env,
    proposer: Address,
    new_wasm_hash: BytesN<32>,
    description: Symbol,
) -> Result<u32, Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::create_proposal(env.clone(), proposer, new_wasm_hash, description)
    })
}

pub fn cast_vote(
    env: Env,
    voter: Address,
    proposal_id: u32,
    vote_type: VoteType,
) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::cast_vote(env.clone(), voter, proposal_id, vote_type)
    })
}

pub fn finalize_proposal(env: Env, proposal_id: u32) -> Result<ProposalStatus, Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::finalize_proposal(env.clone(), proposal_id)
    })
}

pub fn execute_proposal(env: Env, executor: Address, proposal_id: u32) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::execute_proposal(env.clone(), executor, proposal_id)
    })
}

pub fn veto_proposal(env: Env, security_council: Address, proposal_id: u32) -> Result<(), Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::veto_proposal(env.clone(), security_council, proposal_id)
    })
}

pub fn get_config(env: Env) -> Result<GovernanceConfig, Error> {
    let id = ctx(&env);
    env.as_contract(&id, || GovernanceContract::get_config(env.clone()))
}

pub fn get_security_council(env: Env) -> Result<Address, Error> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::get_security_council(env.clone())
    })
}

pub fn get_pending_admin_rotation(env: Env) -> Option<PendingAdminRotation> {
    let id = ctx(&env);
    env.as_contract(&id, || {
        GovernanceContract::get_pending_admin_rotation(env.clone())
    })
}

pub fn get_role_holder(env: &Env, role: Role) -> Option<Address> {
    let id = ctx(env);
    env.as_contract(&id, || governance::get_role_holder(env, role))
}

pub fn is_emergency_paused(env: &Env) -> bool {
    let id = ctx(env);
    env.as_contract(&id, || governance::is_emergency_paused(env))
}
