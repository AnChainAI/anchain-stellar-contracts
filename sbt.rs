use soroban_sdk::{
  contract, contractimpl, contracttype, symbol_short, Symbol, Address, Env, String
};

const METADATA_KEY: Symbol = symbol_short!("METADATA");
const MINT_EVENT: Symbol = symbol_short!("MINT");
const COUNTER: Symbol = symbol_short!("COUNTER");

#[derive(Clone)]
#[contracttype]
pub struct SBTMetadata {
  pub name: String,
  pub symbol: String,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
  Admin,
}

#[derive(Clone)]
#[contracttype]
pub struct SBTDetail {
  pub owner: Address,
  pub uri: String,
}

#[derive(Clone)]
#[contracttype]
pub struct MintEvent {
  pub address: Address,
  pub token_id: u128
}

pub trait SBTTrait {
  fn initialize(env: Env, admin: Address, name: String, symbol: String);

  fn mint_sbt(env: Env, to: Address, token_uri: String) -> u128;

  fn get_sbt_detail(env: Env, token_id: u128) -> SBTDetail;

  fn read_administrator(env: Env) -> Address;

  fn has_administrator(env: Env) -> bool;

  fn has_sbt_owner(env: Env, account: Address, token_id: u128) -> bool;

  fn name(env: Env) -> String;

  fn symbol(env: Env) -> String;
}

#[contract]
pub struct SBTContract;

#[contractimpl]
impl SBTTrait for SBTContract {
  fn initialize(env: Env, admin: Address, name: String, symbol: String) {
      if Self::has_administrator(env.clone()) {
          panic!("Contract already initialized")
      }

      let metadata = SBTMetadata { name, symbol };

      env.storage().instance().set(&DataKey::Admin, &admin);
      env.storage().persistent().set(&METADATA_KEY, &metadata);
  }

  fn mint_sbt(env: Env, to: Address, token_uri: String) -> u128 {
      to.require_auth();

      if to == env.current_contract_address() {
          panic!("Sender can not be contract address")
      } else if token_uri == String::from_slice(&env, "") {
          panic!("SBT URI can not be empty")
      }

      let mut token_id: u128 = env.storage().instance().get(&COUNTER).unwrap_or(0);

      token_id += 1;

      let mint_event: MintEvent = MintEvent { address: to.clone(), token_id };
      let sbt_detail: SBTDetail = SBTDetail {
          owner: to,
          uri: token_uri,
      };

      env.storage().instance().set(&token_id, &sbt_detail);
      env.storage().instance().set(&COUNTER, &token_id);
      env.events().publish((MINT_EVENT, symbol_short!("mint")), mint_event);
      
      token_id
  }

  fn get_sbt_detail(env: Env, token_id: u128) -> SBTDetail {
      let detail: SBTDetail = env
          .storage()
          .instance()
          .get(&token_id)
          .unwrap_or(SBTDetail {
              owner: env.current_contract_address(),
              uri: String::from_slice(&env, ""),
          });

      return detail;
  }

  fn read_administrator(env: Env) -> Address {
      env.storage().instance().get(&DataKey::Admin).unwrap()
  }

  fn has_administrator(env: Env) -> bool {
      let key = DataKey::Admin;
      env.storage().instance().has(&key)
  }

  fn has_sbt_owner(env: Env, account: Address, token_id: u128) -> bool {
      let sbt_detail = Self::get_sbt_detail(env.clone(), token_id.clone());

      if sbt_detail.owner != account {
          return true;
      } else {
          return false;
      }
  }

  fn name(env: Env) -> String {
      let metadata: SBTMetadata = env.storage().persistent().get(&METADATA_KEY).unwrap();

      metadata.name
  }

  fn symbol(env: Env) -> String {
      let metadata: SBTMetadata = env.storage().persistent().get(&METADATA_KEY).unwrap();

      metadata.symbol
  }
}

#[cfg(test)]
mod test;

mod testutils;
