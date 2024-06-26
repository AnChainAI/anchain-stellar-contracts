use soroban_sdk::{
  contract, contractimpl, contracttype, symbol_short, Symbol, Address, Env, String
};

const TRANSFER_EVENT: Symbol = symbol_short!("TRANSFER");
const METADATA_KEY: Symbol = symbol_short!("METADATA");
const MINT_EVENT: Symbol = symbol_short!("MINT");
const BURN_EVENT: Symbol = symbol_short!("BURN");
const COUNTER: Symbol = symbol_short!("COUNTER");

#[derive(Clone)]
#[contracttype]
pub struct NFTMetadata {
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
pub struct NFTDetail {
  pub owner: Address,
  pub uri: String,
}

#[derive(Clone)]
#[contracttype]
pub struct MintEvent {
  pub address: Address,
  pub token_id: u128
}

#[derive(Clone)]
#[contracttype]
pub struct BurnEvent {
  pub address: Address,
  pub token_id: u128
}

#[derive(Clone)]
#[contracttype]
pub struct TransferEvent {
  pub from: Address,
  pub to: Address,
  pub token_id: u128
}

pub trait NFTTrait {
  fn initialize(env: Env, admin: Address, name: String, symbol: String);

  fn mint_nft(env: Env, to: Address, token_uri: String) -> u128;

  fn burn_nft(env: Env, to: Address, token_id: u128);

  fn transfer_nft(env: Env, from: Address, to: Address, token_id: u128);

  fn get_nft_detail(env: Env, token_id: u128) -> NFTDetail;

  fn read_administrator(env: Env) -> Address;

  fn has_administrator(env: Env) -> bool;

  fn has_nft_owner(env: Env, account: Address, token_id: u128) -> bool;

  fn name(env: Env) -> String;

  fn symbol(env: Env) -> String;
}

#[contract]
pub struct NFTContract;

#[contractimpl]
impl NFTTrait for NFTContract {
  fn initialize(env: Env, admin: Address, name: String, symbol: String) {
      if Self::has_administrator(env.clone()) {
          panic!("Contract already initialized")
      }

      let metadata = NFTMetadata { name, symbol };

      env.storage().instance().set(&DataKey::Admin, &admin);
      env.storage().persistent().set(&METADATA_KEY, &metadata);
  }

  fn mint_nft(env: Env, to: Address, token_uri: String) -> u128 {
      to.require_auth();

      if to == env.current_contract_address() {
          panic!("Sender can not be contract address")
      } else if token_uri == String::from_slice(&env, "") {
          panic!("NFT URI can not be empty")
      }

      let mut token_id: u128 = env.storage().instance().get(&COUNTER).unwrap_or(0);

      token_id += 1;

      let mint_event: MintEvent = MintEvent { address: to.clone(), token_id };
      let nft_detail: NFTDetail = NFTDetail {
          owner: to,
          uri: token_uri,
      };

      env.storage().instance().set(&token_id, &nft_detail);
      env.storage().instance().set(&COUNTER, &token_id);
      env.events().publish((MINT_EVENT, symbol_short!("mint")), mint_event);
      
      token_id
  }

  fn burn_nft(env: Env, owner: Address, token_id: u128) {
      owner.require_auth();

      if Self::has_nft_owner(env.clone(), owner.clone(), token_id) {
          panic!("Invalid Sender")
      } else if owner == env.current_contract_address() {
          panic!("Sender can not be contract address")
      }

      let mut nft_detail = Self::get_nft_detail(env.clone(), token_id);

      if nft_detail.owner != owner || nft_detail.owner == env.current_contract_address() {
          panic!("NFT not exist")
      }

      nft_detail.owner = env.current_contract_address();
      nft_detail.uri = String::from_slice(&env, "");

      let burn_event: BurnEvent = BurnEvent { address: owner.clone(), token_id };

      env.storage().instance().set(&token_id, &nft_detail);
      env.events().publish((BURN_EVENT, symbol_short!("burn")), burn_event);
  }

  fn transfer_nft(env: Env, from: Address, to: Address, token_id: u128) {
      from.require_auth();

      if Self::has_nft_owner(env.clone(), from.clone(), token_id) {
          panic!("Invalid Sender")
      } else if from == env.current_contract_address() {
          panic!("Sender can not be contract address")
      }

      let mut nft_detail = Self::get_nft_detail(env.clone(), token_id);

      if nft_detail.owner != from || nft_detail.owner == env.current_contract_address() {
          panic!("NFT not exist")
      }

      let transfer_event: TransferEvent = TransferEvent { from: from.clone(), to: to.clone(), token_id };
      nft_detail.owner = to;

      env.storage().instance().set(&token_id, &nft_detail);
      env.events().publish((TRANSFER_EVENT, symbol_short!("transfer")), transfer_event);
  }

  fn get_nft_detail(env: Env, token_id: u128) -> NFTDetail {
      let detail: NFTDetail = env
          .storage()
          .instance()
          .get(&token_id)
          .unwrap_or(NFTDetail {
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

  fn has_nft_owner(env: Env, account: Address, token_id: u128) -> bool {
      let nft_detail = Self::get_nft_detail(env.clone(), token_id.clone());

      if nft_detail.owner != account {
          return true;
      } else {
          return false;
      }
  }

  fn name(env: Env) -> String {
      let metadata: NFTMetadata = env.storage().persistent().get(&METADATA_KEY).unwrap();

      metadata.name
  }

  fn symbol(env: Env) -> String {
      let metadata: NFTMetadata = env.storage().persistent().get(&METADATA_KEY).unwrap();

      metadata.symbol
  }
}

#[cfg(test)]
mod test;

mod testutils;