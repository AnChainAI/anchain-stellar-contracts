#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol, Vec,
};

mod nft_contract {
    soroban_sdk::contractimport!(file = "nft/nft_soroban.wasm");
}

const SELLEVENT: Symbol = symbol_short!("SELLEVENT");
const AUCEVENT: Symbol = symbol_short!("AUCEVENT");
const BIDEVENT: Symbol = symbol_short!("BIDEVENT");
const DLEVENT: Symbol = symbol_short!("DLEVENT");

#[derive(Clone)]
#[contracttype]
pub struct AuctionEvent {
    token_id: u128,
    owner: Address,
    start_price: i128,
    expiration_date: u64,
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
pub struct BidEvent {
    token_id: u128,
    user: Address,
    bid_price: i128,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    NFTAddress,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct AuctionNFT {
    token_id: u128,
    owner: Address,
    start_price: i128,
    expiration_date: u64,
    bidders: Vec<Bidder>,
    highest_bidder: HighestBidder,
}

#[derive(Clone, Debug)]
#[contracttype]
struct Bidder {
    user: Address,
    price: i128,
}

#[derive(Clone, Debug)]
#[contracttype]
struct HighestBidder {
    user: Address,
    price: i128,
}

#[contract]
pub struct NFTAuctionStorefront;

#[contractimpl]
impl NFTAuctionStorefront {
    pub fn initialize(env: Env, nft_contract_address: Address, admin: Address) {
        if Self::has_administrator(env.clone()) {
            panic!("already initialized")
        }

        env.storage()
            .instance()
            .set(&DataKey::NFTAddress, &nft_contract_address);
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn auction_nft(env: Env, from: Address, token_id: u128, price: i128, expiration_date: u64) {
        from.require_auth();

        let nft_client = Self::get_nft_client(env.clone());

        if nft_client.has_nft_owner(&from.clone(), &token_id) {
            panic!("Invalid Sender")
        } else if from == env.current_contract_address() {
            panic!("Sender can not be contract address")
        } else if token_id == 0 {
            panic!("Token ID can not be zero")
        }

        let auction_nft = Self::get_auctioned_nft(env.clone(), token_id);

        if auction_nft.owner == from {
            panic!("NFT Auctioned Already")
        }

        let auction_event = AuctionEvent {
            token_id,
            owner: from.clone(),
            start_price: price,
            expiration_date,
        };
        let auction_listing = AuctionNFT {
            token_id,
            owner: from,
            start_price: price,
            expiration_date,
            bidders: Vec::new(&env),
            highest_bidder: HighestBidder {
                user: env.current_contract_address(),
                price: 0,
            },
        };

        env.storage().instance().set(&token_id, &auction_listing); // store auction listing at token_id

        env.events()
            .publish((AUCEVENT, symbol_short!("auctioned")), auction_event);
    }

    pub fn get_auctioned_nft(env: Env, token_id: u128) -> AuctionNFT {
        let auction_nft: AuctionNFT = env.storage().instance().get(&token_id).unwrap_or(AuctionNFT {
            token_id: 0,
            owner: env.current_contract_address(),
            start_price: 0,
            expiration_date: 0,
            bidders: Vec::new(&env),
            highest_bidder: HighestBidder {
                user: env.current_contract_address(),
                price: 0,
            },
        });

        return auction_nft;
    }

    pub fn bid_nft(env: Env, user: Address, token_id: u128, bid_price: i128, xlm_address: Address) {
        user.require_auth();

        if user == env.current_contract_address() {
            panic!("Sender can not be contract address")
        } else if token_id == 0 {
            panic!("Token ID can not be zero")
        }

        let mut auction_nft = Self::get_auctioned_nft(env.clone(), token_id);

        if auction_nft.owner == user {
            panic!("Owner NFT can not be bidder")
        } else if auction_nft.token_id == 0 {
            panic!("NFT not auctioned yet")
        }

        if env.ledger().timestamp() > auction_nft.expiration_date {
            panic!("The auction has expired");
        }

        let previous_bid = auction_nft.highest_bidder.clone();

        if bid_price <= previous_bid.price {
            panic!("bid price must be greater than highest bid")
        }

        auction_nft.bidders.push_front(Bidder {
            user: user.clone(),
            price: bid_price,
        });

        auction_nft.highest_bidder = HighestBidder {
            user: user.clone(),
            price: bid_price,
        };

        env.storage().instance().set(&token_id, &auction_nft);

        // Refund the previous highest bidder
        if previous_bid.user != env.current_contract_address() {
            let client = token::Client::new(&env.clone(), &xlm_address);
            client.transfer(&env.current_contract_address(), &previous_bid.user, &previous_bid.price);
        }

        // Transfer XLM to contract address
        let client = token::Client::new(&env.clone(), &xlm_address);
        client.transfer(&user, &env.current_contract_address(), &bid_price);


        let bid_event = BidEvent {
            token_id,
            user,
            bid_price,
        };
        env.events().publish((BIDEVENT, symbol_short!("bid")), bid_event)
    }

    pub fn sell_auctioned_nft(env: Env, owner: Address, token_id: u128, xlm_address: Address) {
        let nft_client = Self::get_nft_client(env.clone());

        if nft_client.has_nft_owner(&owner.clone(), &token_id) {
            panic!("Invalid Sender")
        } else if owner == env.current_contract_address() {
            panic!("Sender can not be contract address")
        } else if token_id == 0 {
            panic!("Token ID can not be zero")
        }

        let auction_nft = Self::get_auctioned_nft(env.clone(), token_id);

        if auction_nft.token_id == 0 {
            panic!("NFT not auctioned yet")
        }

        if env.ledger().timestamp() < auction_nft.expiration_date {
            panic!("Auction has not expired yet")
        }

        env.storage().instance().remove(&token_id);

        let highest_bid = auction_nft.highest_bidder.clone();

        let client = token::Client::new(&env.clone(), &xlm_address);
        client.transfer(&env.current_contract_address(), &owner, &highest_bid.price);

        nft_client.transfer_from(&owner, &highest_bid.user, &token_id);

        let sell_event = SellEvent {
            token_id,
            buyer: highest_bid.user,
            price: highest_bid.price,
        };

        env.events().publish((SELLEVENT, symbol_short!("sell")), sell_event)
    }

    pub fn delist_auctioned_nft(env: Env, from: Address, token_id: u128, xlm_address: Address) {
        from.require_auth();

        let admin = Self::read_administrator(env.clone());
        let auctioned_nft = Self::get_auctioned_nft(env.clone(), token_id);

        if auctioned_nft.token_id == 0 {
            panic!("NFT not auctioned");
        }

        if from != auctioned_nft.owner && from != admin {
            panic!("Only the owner or admin can delist the auctioned NFT");
        }

        env.storage().instance().remove(&token_id);

        let highest_bidder = auctioned_nft.highest_bidder.clone();

        if highest_bidder.user != auctioned_nft.owner && highest_bidder.price != 0 {
            // Refund the previous bidder
            if highest_bidder.user != env.current_contract_address() {
                let client = token::Client::new(&env.clone(), &xlm_address);
                client.transfer(&env.current_contract_address(), &highest_bidder.user, &highest_bidder.price);
            }
        }

        let delist_event = DelistEvent {
            token_id,
            owner: from,
        };

        env.events().publish((DLEVENT, symbol_short!("delisted")), delist_event)
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
