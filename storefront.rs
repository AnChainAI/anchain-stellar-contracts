#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol
};

mod nft_contract {
    soroban_sdk::contractimport!(file = "nft/nft_soroban.wasm");
}

const LISTEVENT: Symbol = symbol_short!("LISTEVENT");
const SELLEVENT: Symbol = symbol_short!("SELLEVENT");
const DLEVENT: Symbol = symbol_short!("DLEVENT");

#[derive(Clone)]
#[contracttype]
pub struct ListEvent {
    token_id: u128,
    owner: Address,
    price: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct SellEvent {
    token_id: u128,
    buyer: Address,
    price: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct DelistEvent {
    token_id: u128,
    owner: Address,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    NFTAddress,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct NFTListing {
    token_id: u128,
    owner: Address,
    price: i128,
}

#[contract]
pub struct NFTStoreFront;

#[contractimpl]
impl NFTStoreFront {
    fn initialize(env: Env, nft_contract_address: Address, admin: Address) {
        if Self::has_administrator(env.clone()) {
            panic!("Contract already initialized")
        }

        env.storage()
            .instance()
            .set(&DataKey::NFTAddress, &nft_contract_address);
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn list_nft(env: Env, from: Address, token_id: u128, price: i128) {
        from.require_auth();

        let nft_client = Self::get_nft_client(env.clone());

        if nft_client.has_nft_owner(&from.clone(), &token_id) {
            panic!("Invalid Sender")
        } else if from == env.current_contract_address() {
            panic!("Sender can not be contract address")
        } else if token_id == 0 {
            panic!("Token ID can not be zero")
        }

        let list_nft = Self::get_listed_nft(env.clone(), token_id);

        if list_nft.owner == from {
            panic!("NFT Listed Already")
        }

        let list_event = ListEvent {
            token_id,
            owner: from.clone(),
            price,
        };
        let listing = NFTListing {
            token_id,
            owner: from,
            price,
        };

        env.storage().instance().set(&token_id, &listing); // store list nft at token_id

        env.events().publish((LISTEVENT, symbol_short!("listed")), list_event);
    }

    pub fn delist_nft(env: Env, from: Address, token_id: u128) {
        from.require_auth();

        let admin = Self::read_administrator(env.clone());
        let listed_nft = Self::get_listed_nft(env.clone(), token_id);

        if listed_nft.token_id == 0 {
            panic!("NFT not listed");
        }

        if from != listed_nft.owner && from != admin {
            panic!("Only the owner or admin can delist the NFT");
        }

        env.storage().instance().remove(&token_id);

        let delist_event = DelistEvent {
            token_id,
            owner: from,
        };

        env.events().publish((DLEVENT, symbol_short!("delisted")), delist_event)
    }

    fn get_listed_nft(env: Env, token_id: u128) -> NFTListing {
        let listed_nft: NFTListing = env.storage().instance().get(&token_id).unwrap_or(NFTListing {
            token_id: 0,
            owner: env.current_contract_address(),
            price: 0
        });

        return listed_nft;
    }

    pub fn purchase_listed_nft(env: Env, owner: Address, buyer: Address, token_id: u128, xlm_address: Address) {
        buyer.require_auth();
        let nft_client = Self::get_nft_client(env.clone());

        if nft_client.has_nft_owner(&owner.clone(), &token_id) {
            panic!("Invalid Sender")
        } else if owner == env.current_contract_address() {
            panic!("Sender can not be contract address")
        } else if token_id == 0 {
            panic!("Token ID can not be zero")
        }

        let mut listed_nft = Self::get_listed_nft(env.clone(), token_id);

        if listed_nft.token_id == 0 {
            panic!("NFT not listed yet")
        }

        let client = token::Client::new(&env.clone(), &xlm_address);
        client.transfer(&buyer, &owner, &listed_nft.price);

        nft_client.transfer_from(&owner, &buyer, &token_id);

        env.storage().instance().remove(&token_id);

        let sell_event = SellEvent {
            token_id,
            buyer: buyer.clone(),
            price: listed_nft.price,
        };

        env.events().publish((SELLEVENT, symbol_short!("sold")), sell_event)
    }

    fn read_administrator(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    fn has_administrator(env: Env) -> bool {
        let key = DataKey::Admin;
        env.storage().instance().has(&key)
    }

    fn get_nft_client(env: Env) -> nft_contract::Client<'static> {
        let contract = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::NFTAddress)
            .expect("none");

        let client = nft_contract::Client::new(&env, &contract);

        return client;
    }
}

#[cfg(test)]
mod test;

mod testutils;
