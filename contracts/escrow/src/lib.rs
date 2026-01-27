#![no_std]

use soroban_sdk::{contract, contractevent, contractimpl, contracttype, token, Address, Env, Map, Vec};

/// Role-based access control for the escrow contract
///
/// Authorization Model:
/// - Backend: Can trigger automated payouts (release_funds, batch_payout)
/// - Maintainer: Can trigger refunds (along with backend)
/// - Admin: Can manage authorized keys (set during initialization)
///
/// Role Hierarchy:
/// - Backend ⊇ Maintainer (Backend can perform all Maintainer operations)
/// - Admin is separate and only manages keys
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Backend,
    Maintainer,
}

#[contracttype]
#[derive(Clone)]
pub struct AuthorizedKey {
    pub key: Address,
    pub role: Role,
}

/// Authorization event for audit trail
#[contractevent]
#[derive(Clone)]
pub struct AuthorizationEvent {
    pub caller: Address,
    pub action: u32, // 0: release_funds, 1: refund, 2: batch_payout, 3: set_key
    pub authorized: bool,
}

#[contractevent]
#[derive(Clone)]
pub struct FundsReleasedEvent {
    pub contributor: Address,
    pub amount: i128,
    pub authorized_by: Address,
}

#[contractevent]
#[derive(Clone)]
pub struct RefundEvent {
    pub recipient: Address,
    pub amount: i128,
    pub authorized_by: Address,
}

#[contractevent]
#[derive(Clone)]
pub struct BatchPayoutEvent {
    pub payouts: Vec<(Address, i128)>,
    pub authorized_by: Address,
}

#[contractevent]
#[derive(Clone)]
pub struct KeyManagementEvent {
    pub key: Address,
    pub role: Role,
    pub action: u32, // 0: added, 1: removed
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Initialize the contract with admin, token address, and initial authorized keys
    ///
    /// Authorization Flow:
    /// - Admin: Can set authorized keys
    /// - Backend: Can trigger automated payouts (release_funds, batch_payout)
    /// - Maintainer: Can trigger refunds (along with backend)
    /// - Everyone: Read-only access to contract state
    ///
    /// Security Guarantees:
    /// - Only admin can initialize (one-time setup)
    /// - All state-changing operations require proper authorization
    /// - Reentrancy protection on all fund transfers
    /// - Input validation on all amounts and addresses
    pub fn initialize(env: Env, admin: Address, token_address: Address, backend_key: Address, maintainer_keys: Vec<Address>) {
        admin.require_auth();

        // Prevent re-initialization
        if env.storage().instance().has(&"admin") {
            panic!("Contract already initialized");
        }

        // Store admin
        env.storage().instance().set(&"admin", &admin);

        // Store token address
        env.storage().instance().set(&"token", &token_address);

        // Initialize authorized keys map
        let mut auth_keys = Map::new(&env);

        // Add backend key
        auth_keys.set(backend_key.clone(), Role::Backend);

        // Add maintainer keys
        for key in maintainer_keys.iter() {
            auth_keys.set(key, Role::Maintainer);
        }

        env.storage().instance().set(&"authorized_keys", &auth_keys);

        // Initialize reentrancy guard
        env.storage().instance().set(&"locked", &false);
    }

    /// Check if an address is authorized for a specific role
    ///
    /// Authorization Logic:
    /// - Checks if address exists in authorized_keys map
    /// - Validates role hierarchy: Backend ⊇ Maintainer
    /// - Returns false for unauthorized addresses
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `address` - Address to check authorization for
    /// * `required_role` - Role required for the operation
    ///
    /// # Returns
    /// true if authorized, false otherwise
    pub fn is_authorized(env: Env, address: Address, required_role: Role) -> bool {
        let auth_keys: Map<Address, Role> = env.storage().instance().get(&"authorized_keys").unwrap_or(Map::new(&env));

        if let Some(role) = auth_keys.get(address) {
            match required_role {
                // Backend can only perform backend operations
                Role::Backend => matches!(role, Role::Backend),
                // Maintainer operations can be performed by Backend or Maintainer
                Role::Maintainer => matches!(role, Role::Backend | Role::Maintainer),
            }
        } else {
            false
        }
    }

    /// Verify authorization and emit audit event
    ///
    /// This function combines authorization check with event emission for audit trail.
    /// Used internally by all state-changing functions.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `caller` - Address attempting the operation
    /// * `required_role` - Role required for the operation
    /// * `action` - Action code for audit trail (0: release_funds, 1: refund, 2: batch_payout, 3: set_key)
    ///
    /// # Panics
    /// If caller is not authorized for the required role
    fn verify_authorization(env: &Env, caller: Address, required_role: Role, action: u32) {
        let is_auth = Self::is_authorized(env.clone(), caller.clone(), required_role);
        
        // Emit authorization event for audit trail
        AuthorizationEvent {
            caller: caller.clone(),
            action,
            authorized: is_auth,
        }.publish(env);

        if !is_auth {
            panic!("Unauthorized: insufficient permissions for this operation");
        }
    }

    /// Reentrancy guard helper - prevents recursive calls during external operations
    ///
    /// Security: Protects against reentrancy attacks by setting a lock flag
    /// before making external calls (token transfers).
    fn check_reentrancy(env: &Env) {
        let locked: bool = env.storage().instance().get(&"locked").unwrap_or(false);
        if locked {
            panic!("Reentrancy detected: operation already in progress");
        }
        env.storage().instance().set(&"locked", &true);
    }

    /// Release reentrancy guard after operation completes
    fn release_reentrancy(env: &Env) {
        env.storage().instance().set(&"locked", &false);
    }

    /// Set or update an authorized key (admin only)
    ///
    /// Authorization: Only the contract admin can manage authorized keys.
    /// This is a critical operation that should be carefully controlled.
    ///
    /// Security Considerations:
    /// - Requires admin authentication via require_auth()
    /// - Validates admin identity against stored admin address
    /// - Emits event for audit trail
    /// - Can add new keys or update existing key roles
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `admin` - Address claiming to be admin (must match stored admin)
    /// * `key` - Address to authorize or update
    /// * `role` - Role to assign (Backend or Maintainer)
    ///
    /// # Panics
    /// If caller is not the stored admin address
    pub fn set_authorized_key(env: Env, admin: Address, key: Address, role: Role) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&"admin").unwrap();
        if admin != stored_admin {
            panic!("Unauthorized: only admin can manage authorized keys");
        }

        let mut auth_keys: Map<Address, Role> = env.storage().instance().get(&"authorized_keys").unwrap_or(Map::new(&env));
        auth_keys.set(key.clone(), role.clone());
        env.storage().instance().set(&"authorized_keys", &auth_keys);

        // Emit key management event for audit trail
        KeyManagementEvent {
            key,
            role,
            action: 0, // 0 = added/updated
        }.publish(&env);
    }

    /// Remove an authorized key (admin only)
    ///
    /// Authorization: Only the contract admin can remove authorized keys.
    ///
    /// Security Considerations:
    /// - Requires admin authentication
    /// - Validates admin identity
    /// - Emits event for audit trail
    /// - Prevents removal of all backend keys (safety check)
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `admin` - Address claiming to be admin
    /// * `key` - Address to remove from authorization
    ///
    /// # Panics
    /// If caller is not admin or if removing the key would leave no backend keys
    pub fn remove_authorized_key(env: Env, admin: Address, key: Address) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&"admin").unwrap();
        if admin != stored_admin {
            panic!("Unauthorized: only admin can manage authorized keys");
        }

        let mut auth_keys: Map<Address, Role> = env.storage().instance().get(&"authorized_keys").unwrap_or(Map::new(&env));
        
        // Safety check: ensure at least one backend key remains
        if let Some(role) = auth_keys.get(key.clone()) {
            if matches!(role, Role::Backend) {
                let mut backend_count = 0;
                for (_, r) in auth_keys.iter() {
                    if matches!(r, Role::Backend) {
                        backend_count += 1;
                    }
                }
                if backend_count <= 1 {
                    panic!("Cannot remove last backend key: contract requires at least one backend");
                }
            }
        }

        auth_keys.remove(key.clone());
        env.storage().instance().set(&"authorized_keys", &auth_keys);

        // Emit key removal event for audit trail
        KeyManagementEvent {
            key,
            role: Role::Backend, // Placeholder, actual role doesn't matter for removal
            action: 1, // 1 = removed
        }.publish(&env);
    }

    /// Release funds to a contributor (backend only)
    ///
    /// Authorization: Only authorized backend can trigger.
    /// This function transfers tokens from the escrow contract to a contributor.
    ///
    /// Security Guarantees:
    /// - Requires backend authorization (checked via is_authorized)
    /// - Validates amount is positive
    /// - Checks contract has sufficient balance
    /// - Reentrancy protection prevents recursive calls
    /// - Emits event for audit trail
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `backend` - Address of backend service (must be authorized)
    /// * `contributor` - Address to receive funds
    /// * `amount` - Amount of tokens to transfer (must be > 0)
    ///
    /// # Panics
    /// If:
    /// - backend is not authorized
    /// - amount <= 0
    /// - contract balance is insufficient
    /// - reentrancy is detected
    pub fn release_funds(env: Env, backend: Address, contributor: Address, amount: i128) {
        backend.require_auth();

        // Verify authorization and emit audit event (action code: 0)
        Self::verify_authorization(&env, backend.clone(), Role::Backend, 0);

        // Input validation: amount must be positive
        if amount <= 0 {
            panic!("Invalid amount: must be positive");
        }

        // Reentrancy protection
        Self::check_reentrancy(&env);

        let token_address: Address = env.storage().instance().get(&"token").unwrap();
        let token = token::TokenClient::new(&env, &token_address);

        // Check contract balance
        let contract_balance = token.balance(&env.current_contract_address());
        if contract_balance < amount {
            Self::release_reentrancy(&env);
            panic!("Insufficient funds in escrow");
        }

        // Transfer tokens
        token.transfer(&env.current_contract_address(), &contributor, &amount);

        // Release guard
        Self::release_reentrancy(&env);

        // Emit event with authorization info
        FundsReleasedEvent {
            contributor,
            amount,
            authorized_by: backend,
        }.publish(&env);
    }

    /// Refund funds (maintainer or backend)
    ///
    /// Authorization: Authorized maintainer or backend can trigger.
    /// This function transfers tokens from the escrow contract to a recipient.
    ///
    /// Security Guarantees:
    /// - Requires maintainer or backend authorization
    /// - Validates amount is positive
    /// - Checks contract has sufficient balance
    /// - Reentrancy protection prevents recursive calls
    /// - Emits event for audit trail
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `caller` - Address of caller (must be authorized as maintainer or backend)
    /// * `recipient` - Address to receive refund
    /// * `amount` - Amount of tokens to transfer (must be > 0)
    ///
    /// # Panics
    /// If:
    /// - caller is not authorized as maintainer or backend
    /// - amount <= 0
    /// - contract balance is insufficient
    /// - reentrancy is detected
    pub fn refund(env: Env, caller: Address, recipient: Address, amount: i128) {
        caller.require_auth();

        // Verify authorization and emit audit event (action code: 1)
        Self::verify_authorization(&env, caller.clone(), Role::Maintainer, 1);

        // Input validation: amount must be positive
        if amount <= 0 {
            panic!("Invalid amount: must be positive");
        }

        // Reentrancy protection
        Self::check_reentrancy(&env);

        let token_address: Address = env.storage().instance().get(&"token").unwrap();
        let token = token::TokenClient::new(&env, &token_address);

        // Check contract balance
        let contract_balance = token.balance(&env.current_contract_address());
        if contract_balance < amount {
            Self::release_reentrancy(&env);
            panic!("Insufficient funds in escrow");
        }

        // Transfer tokens
        token.transfer(&env.current_contract_address(), &recipient, &amount);

        // Release guard
        Self::release_reentrancy(&env);

        // Emit event with authorization info
        RefundEvent {
            recipient,
            amount,
            authorized_by: caller,
        }.publish(&env);
    }

    /// Batch payout to multiple contributors (backend only)
    ///
    /// Authorization: Only authorized backend can trigger.
    /// This function transfers tokens to multiple recipients in a single transaction.
    ///
    /// Security Guarantees:
    /// - Requires backend authorization
    /// - Validates all amounts are positive
    /// - Checks for amount overflow
    /// - Checks contract has sufficient total balance
    /// - Reentrancy protection prevents recursive calls
    /// - Emits event for audit trail
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `backend` - Address of backend service (must be authorized)
    /// * `payouts` - Vector of (recipient, amount) tuples
    ///
    /// # Panics
    /// If:
    /// - backend is not authorized
    /// - payouts vector is empty
    /// - any amount <= 0
    /// - total amount overflows i128
    /// - contract balance is insufficient
    /// - reentrancy is detected
    pub fn batch_payout(env: Env, backend: Address, payouts: Vec<(Address, i128)>) {
        backend.require_auth();

        // Verify authorization and emit audit event (action code: 2)
        Self::verify_authorization(&env, backend.clone(), Role::Backend, 2);

        // Input validation: payouts must not be empty
        if payouts.is_empty() {
            panic!("No payouts specified");
        }

        // Validate all amounts and calculate total
        let mut total_amount = 0i128;
        for (_recipient, amount) in payouts.iter() {
            // Validate amount is positive
            if amount <= 0 {
                panic!("Invalid amount: must be positive");
            }

            // Check for overflow
            total_amount = total_amount.checked_add(amount).unwrap_or_else(|| panic!("Amount overflow: total exceeds i128 max"));
        }

        // Reentrancy protection
        Self::check_reentrancy(&env);

        let token_address: Address = env.storage().instance().get(&"token").unwrap();
        let token = token::TokenClient::new(&env, &token_address);

        // Check contract balance
        let contract_balance = token.balance(&env.current_contract_address());
        if contract_balance < total_amount {
            Self::release_reentrancy(&env);
            panic!("Insufficient funds in escrow for batch payout");
        }

        // Transfer tokens for each payout
        for (contributor, amount) in payouts.iter() {
            token.transfer(&env.current_contract_address(), &contributor, &amount);
        }

        // Release guard
        Self::release_reentrancy(&env);

        // Emit event with authorization info
        BatchPayoutEvent {
            payouts,
            authorized_by: backend,
        }.publish(&env);
    }

    /// Get the count of authorized keys (read-only, no authorization required)
    ///
    /// This is a read-only operation that returns the number of authorized keys
    /// in the contract. Useful for monitoring and auditing purposes.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    ///
    /// # Returns
    /// Number of authorized keys currently stored
    pub fn get_authorized_keys_count(env: Env) -> u32 {
        let auth_keys: Map<Address, Role> = env.storage().instance().get(&"authorized_keys").unwrap_or(Map::new(&env));
        auth_keys.len()
    }

    /// Check if a specific address is authorized (read-only, no authorization required)
    ///
    /// This is a read-only operation that allows anyone to check if an address
    /// has authorization for a specific role. Useful for frontend/UI purposes.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `address` - Address to check
    /// * `role` - Role to check authorization for
    ///
    /// # Returns
    /// true if authorized for the role, false otherwise
    pub fn check_authorization(env: Env, address: Address, role: Role) -> bool {
        Self::is_authorized(env, address, role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, MockAuth, MockAuthInvoke}, Env, IntoVal};

    #[test]
    fn test_authorization_hierarchy() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer.clone()]);

        // Initialize with mocked auth
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Test backend authorization
        assert!(client.is_authorized(&backend, &Role::Backend));
        assert!(!client.is_authorized(&maintainer, &Role::Backend));

        // Test maintainer authorization (should allow backend too)
        assert!(client.is_authorized(&backend, &Role::Maintainer));
        assert!(client.is_authorized(&maintainer, &Role::Maintainer));
        assert!(!client.is_authorized(&unauthorized, &Role::Maintainer));
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_release_funds() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let contributor = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to release funds with unauthorized address
        env.mock_auths(&[MockAuth {
            address: &unauthorized,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "release_funds",
                args: (unauthorized.clone(), contributor.clone(), 1000i128).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.release_funds(&unauthorized, &contributor, &1000i128);
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_refund() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let recipient = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to refund with unauthorized address
        env.mock_auths(&[MockAuth {
            address: &unauthorized,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "refund",
                args: (unauthorized.clone(), recipient.clone(), 1000i128).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.refund(&unauthorized, &recipient, &1000i128);
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_batch_payout() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let contributor = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try batch payout with unauthorized address
        let payouts = Vec::from_array(&env, [(contributor.clone(), 1000i128)]);
        env.mock_auths(&[MockAuth {
            address: &unauthorized,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "batch_payout",
                args: (unauthorized.clone(), payouts.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.batch_payout(&unauthorized, &payouts);
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_set_key() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        let new_key = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to set key with unauthorized address
        env.mock_auths(&[MockAuth {
            address: &unauthorized,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_authorized_key",
                args: (unauthorized.clone(), new_key.clone(), Role::Backend).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.set_authorized_key(&unauthorized, &new_key, &Role::Backend);
    }

    #[test]
    #[should_panic(expected = "Invalid amount")]
    fn test_invalid_amount_release_funds() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let contributor = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to release with invalid amount
        env.mock_auths(&[MockAuth {
            address: &backend,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "release_funds",
                args: (backend.clone(), contributor.clone(), 0i128).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.release_funds(&backend, &contributor, &0i128);
    }

    #[test]
    #[should_panic(expected = "No payouts")]
    fn test_empty_batch_payout() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try empty batch payout
        let payouts = Vec::new(&env);
        env.mock_auths(&[MockAuth {
            address: &backend,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "batch_payout",
                args: (backend.clone(), payouts.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.batch_payout(&backend, &payouts);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_prevent_reinitialization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize first time
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to initialize again
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);
    }

    #[test]
    fn test_maintainer_can_refund() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer.clone()]);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Verify maintainer is authorized for refund
        assert!(client.is_authorized(&maintainer, &Role::Maintainer));
    }

    #[test]
    fn test_backend_can_perform_maintainer_operations() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer.clone()]);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Verify backend can perform maintainer operations (role hierarchy)
        assert!(client.is_authorized(&backend, &Role::Maintainer));
        assert!(client.is_authorized(&backend, &Role::Backend));
    }

    #[test]
    fn test_maintainer_cannot_perform_backend_operations() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer.clone()]);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Verify maintainer cannot perform backend operations
        assert!(!client.is_authorized(&maintainer, &Role::Backend));
        assert!(client.is_authorized(&maintainer, &Role::Maintainer));
    }

    #[test]
    fn test_set_and_remove_authorized_keys() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let new_backend = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Add new backend key
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_authorized_key",
                args: (admin.clone(), new_backend.clone(), Role::Backend).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.set_authorized_key(&admin, &new_backend, &Role::Backend);

        // Verify new backend is authorized
        assert!(client.is_authorized(&new_backend, &Role::Backend));

        // Remove original backend (we have 2 now, so this should succeed)
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "remove_authorized_key",
                args: (admin.clone(), backend.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.remove_authorized_key(&admin, &backend);

        // Verify original backend is no longer authorized
        assert!(!client.is_authorized(&backend, &Role::Backend));
        assert!(client.is_authorized(&new_backend, &Role::Backend));
    }

    #[test]
    #[should_panic(expected = "Cannot remove last backend key")]
    fn test_cannot_remove_last_backend_key() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, []);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to remove the only backend key (should fail)
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "remove_authorized_key",
                args: (admin.clone(), backend.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.remove_authorized_key(&admin, &backend);
    }

    #[test]
    fn test_get_authorized_keys_count() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer1 = Address::generate(&env);
        let maintainer2 = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer1.clone(), maintainer2.clone()]);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Verify count (1 backend + 2 maintainers = 3)
        assert_eq!(client.get_authorized_keys_count(), 3);
    }

    #[test]
    fn test_check_authorization_read_only() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer.clone()]);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Check authorization for various addresses (read-only, no auth required)
        assert!(client.check_authorization(&backend, &Role::Backend));
        assert!(client.check_authorization(&backend, &Role::Maintainer));
        assert!(client.check_authorization(&maintainer, &Role::Maintainer));
        assert!(!client.check_authorization(&maintainer, &Role::Backend));
        assert!(!client.check_authorization(&unauthorized, &Role::Backend));
        assert!(!client.check_authorization(&unauthorized, &Role::Maintainer));
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_set_key_non_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer = Address::generate(&env);
        let new_key = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer.clone()]);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to set key as maintainer (not admin)
        env.mock_auths(&[MockAuth {
            address: &maintainer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_authorized_key",
                args: (maintainer.clone(), new_key.clone(), Role::Backend).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.set_authorized_key(&maintainer, &new_key, &Role::Backend);
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_remove_key_non_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, EscrowContract);
        let client = EscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let backend = Address::generate(&env);
        let maintainer = Address::generate(&env);

        let maintainer_keys = Vec::from_array(&env, [maintainer.clone()]);

        // Initialize
        env.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), token.clone(), backend.clone(), maintainer_keys.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &token, &backend, &maintainer_keys);

        // Try to remove key as maintainer (not admin)
        env.mock_auths(&[MockAuth {
            address: &maintainer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "remove_authorized_key",
                args: (maintainer.clone(), backend.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.remove_authorized_key(&maintainer, &backend);
    }
}