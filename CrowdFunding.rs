#![no_std]
use soroban_sdk::{contractimpl, contracttype, Address, BytesN, Env, IntoVal, RawVal};

mod token {
    soroban_sdk::contractimport!(file = "./soroban_token_spec.wasm");
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Deadline,
    Recipient,
    Started,
    Target,
    Token,
    User(Address),
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum State {
    Running = 0,
    Success = 1,
    Expired = 2,
}

impl IntoVal<Env, RawVal> for State {
    fn into_val(&self, env: &Env) -> RawVal {
        (*self as u32).into_val(env)
    }
}

// Retrieves the current timestamp from the blockchain ledger.
fn get_ledger_timestamp(e: &Env) -> u64 {
    e.ledger().timestamp()
}

// Retrieves the address of the recipient
fn get_recipient(e: &Env) -> Address {
    e.storage()
        .get(&DataKey::Recipient)
        .expect("not initialized")
        .unwrap()
}

// Retrieves the timestamp of the deadline
fn get_deadline(e: &Env) -> u64 {
    e.storage()
        .get(&DataKey::Deadline)
        .expect("not initialized")
        .unwrap()
}

// Retrieves the timestamp of when this fund started
fn get_started(e: &Env) -> u64 {
    e.storage()
        .get(&DataKey::Started)
        .expect("not initialized")
        .unwrap()
}

// Retrieves the total amount to complete this fund
fn get_target_amount(e: &Env) -> i128 {
    e.storage()
        .get(&DataKey::Target)
        .expect("not initialized")
        .unwrap()
}

// Retrieves the token that is being used for this fund
fn get_token(e: &Env) -> BytesN<32> {
    e.storage()
        .get(&DataKey::Token)
        .expect("not initialized")
        .unwrap()
}

// Retrieves the amount of tokens deposited by a specific user
fn get_user_deposited(e: &Env, user: &Address) -> i128 {
    e.storage()
        .get(&DataKey::User(user.clone()))
        .unwrap_or(Ok(0))
        .unwrap()
}

// Retrieves the current balance by contract id
fn get_balance(e: &Env, contract_id: &BytesN<32>) -> i128 {
    let client = token::Client::new(e, contract_id);
    client.balance(&e.current_contract_address())
}

// Retrieves the current state of the crowd funding contract based on its stored values
fn get_state(e: &Env) -> State {
    let deadline = get_deadline(e);
    let target_amount = get_target_amount(e);
    let token_id = get_token(e);
    let token_balance = get_balance(e, &token_id);
    let current_timestamp = get_ledger_timestamp(e);

    if current_timestamp < deadline {
        return State::Running;
    };
    if token_balance >= target_amount {
        return State::Success;
    };
    State::Expired
}

// Updates the amount of tokens depositied by a specific user
fn set_user_deposited(e: &Env, user: &Address, amount: &i128) {
    e.storage().set(&DataKey::User(user.clone()), amount);
}

// Transfer tokens from the contract to the recipient
fn transfer(e: &Env, to: &Address, amount: &i128) {
    let token_contract_id = &get_token(e);
    let client = token::Client::new(e, token_contract_id);
    client.xfer(&e.current_contract_address(), to, amount);
}

struct Crowdfund;

#[contractimpl]
#[allow(clippy::needless_pass_by_value)]
impl Crowdfund {
    pub fn initialize(
        e: Env,
        recipient: Address,
        deadline: u64,
        target_amount: i128,
        token: BytesN<32>,
    ) {
        assert!(!e.storage().has(&DataKey::Recipient), "already initialized");

        e.storage().set(&DataKey::Recipient, &recipient);
        e.storage()
            .set(&DataKey::Started, &get_ledger_timestamp(&e));
        e.storage().set(&DataKey::Deadline, &deadline);
        e.storage().set(&DataKey::Target, &target_amount);
        e.storage().set(&DataKey::Token, &token);
    }

    pub fn recipient(e: Env) -> Address {
        get_recipient(&e)
    }

    pub fn deadline(e: Env) -> u64 {
        get_deadline(&e)
    }

    pub fn started(e: Env) -> u64 {
        get_started(&e)
    }

    pub fn state(e: Env) -> u32 {
        get_state(&e) as u32
    }

    pub fn target(e: Env) -> i128 {
        get_target_amount(&e)
    }

    pub fn token(e: Env) -> BytesN<32> {
        get_token(&e)
    }

    pub fn balance(e: Env, user: Address) -> i128 {
        let recipient = get_recipient(&e);
        if get_state(&e) == State::Success {
            if user != recipient {
                return 0;
            };
            return get_balance(&e, &get_token(&e));
        };

        get_user_deposited(&e, &user)
    }

    // Deposit tokens into the contract and record the sender
    pub fn deposit(e: Env, user: Address, amount: i128) {
        user.require_auth();
        assert!(amount > 0, "amount must be positive");
        assert!(get_state(&e) == State::Running, "sale is not running");

        let recipient = get_recipient(&e);
        assert!(user != recipient, "recipient may not deposit");

        let balance = get_user_deposited(&e, &user);
        set_user_deposited(&e, &user, &(balance + amount));

        let client = token::Client::new(&e, &get_token(&e));
        client.xfer(&user, &e.current_contract_address(), &amount);
    }

    // Withdraw funded tokens depending on current state
    // If funding was met within the alloted timeline, the receiver is allowed to withdraw
    // If funding was not met wthin the alloted timeline, the sender can withdraw their fundings
    pub fn withdraw(e: Env, to: Address) {
        let state = get_state(&e);
        let recipient = get_recipient(&e);

        match state {
            State::Running => {
                panic!("funding is still running")
            }
            State::Success => {
                assert!(
                    to == recipient,
                    "fund was successful, only the recipient may withdraw"
                );
                let token = get_token(&e);
                transfer(&e, &recipient, &get_balance(&e, &token));
            }
            State::Expired => {
                assert!(
                    to != recipient,
                    "fund has expired, the recipient may not withdraw"
                );
                // Withdraw full amount
                let balance = get_user_deposited(&e, &to);
                set_user_deposited(&e, &to, &0);
                transfer(&e, &to, &balance);
            }
        };
    }
}