use soroban_sdk::{contracttype, Env, Symbol, symbol_short};

const REENTRANCY_KEY: Symbol = symbol_short!("RE_GUARD");

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuardState {
    Unlocked = 0,
    Locked = 1,
}

#[derive(Clone, Copy)]
pub struct ReentrancyGuard;

impl ReentrancyGuard {
    /// Enter the guarded section
    /// Returns error if already locked (reentrancy detected)
    pub fn enter(env: &Env) -> Result<(), ReentrancyError> {
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

/// Guard that automatically exits on drop (RAII pattern)
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
        ReentrancyGuard::exit(self.env);
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReentrancyError {
    ReentrantCall = 1,
}
