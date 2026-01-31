//! # Test Utilities Library
//!
//! Comprehensive testing utilities for Soroban smart contract development.
//! This library provides reusable helpers, factories, and assertion utilities
//! to simplify test development and reduce boilerplate code.
//!
//! ## Modules
//!
//! - [`factories`] - Contract factory functions for creating test contracts
//! - [`setup`] - Test setup helpers and TestSetup struct
//! - [`assertions`] - Assertion utilities for common test scenarios
//! - [`generators`] - Test data generators
//! - [`time`] - Time manipulation helpers
//! - [`balances`] - Balance verification helpers

pub mod factories;
pub mod setup;
pub mod assertions;
pub mod generators;
pub mod time;
pub mod balances;

// Re-export commonly used items (only in test context)
#[cfg(test)]
pub use factories::*;
#[cfg(test)]
pub use setup::*;
#[cfg(test)]
pub use assertions::*;
pub use generators::*;
pub use time::*;
pub use balances::*;
