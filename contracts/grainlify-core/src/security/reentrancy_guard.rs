use soroban_sdk::{contracttype, Env, Symbol, symbol_short};

const REENTRANCY_KEY: Symbol = symbol_short!("RE_GUARD");

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuardState {
    Unlocked = 0,
    Locked = 1,
}

impl ReentrancyGuard {
    /// Enter the guarded section
    /// Returns error if already locked (reentrancy detected)
    pub fn enter(env: &Env) -> Result<(), ReentrancyError> {
        // FIXED: Using instance storage to persist state across external calls
        let current_state = env
            .storage()
            .instance()
            .get(&REENTRANCY_KEY)
            .unwrap_or(GuardState::Unlocked);

        if current_state == GuardState::Locked {
            return Err(ReentrancyError::ReentrantCall);
        }

        env.storage()
            .instance()
            .set(&REENTRANCY_KEY, &GuardState::Locked);

        Ok(())
    }

    /// Exit the guarded section
    pub fn exit(env: &Env) {
        env.storage()
            .instance()
            .set(&REENTRANCY_KEY, &GuardState::Unlocked);
    }

    /// Check if currently locked
    pub fn is_locked(env: &Env) -> bool {
        env.storage()
            .instance()
            .get(&REENTRANCY_KEY)
            .unwrap_or(GuardState::Unlocked)
            == GuardState::Locked
    }
}

/// Macro for automatic guard management
#[macro_export]
macro_rules! guarded {
    ($env:expr, $body:expr) => {{
        ReentrancyGuard::enter(&$env)?;
        let result = $body;
        ReentrancyGuard::exit(&$env);
        result
    }};
}

/// Guard that automatically exits on drop (RAII pattern)
/// Note: Soroban doesn't support defer blocks yet, so RAII is a good alternative.
pub struct ReentrancyGuardRAII<'a> {
    env: &'a Env,
}

impl<'a> ReentrancyGuardRAII<'a> {
    pub fn new(env: &'a Env) -> Result<Self, ReentrancyError> {
        ReentrancyGuard::enter(env)?;
        Ok(Self { env })
    }
}

impl<'a> Drop for ReentrancyGuardRAII<'a> {
    fn drop(&mut self) {
        // TODO: Future automated reentrancy detection
        ReentrancyGuard::exit(self.env);
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReentrancyError {
    ReentrantCall = 1,
}
