use cosmwasm_std::{Uint128, Decimal, Timestamp, BlockInfo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::{Item,Map,MultiIndex,IndexList,Index,IndexedMap};

pub const CONFIG: Item<State> = Item::new("config_state");
pub const MEMBERS: Map<&str,Vec<UserInfo>> = Map::new("config_members");
pub const SALEHISTORY: Map<(&str,&str), SaleInfo> = Map::new("sale");
pub const COLLECTIONINFO: Map<&str, CollectionInfo> = Map::new("collection_info");
pub const TOKENADDRESS: Map<&str, String> = Map::new("token_address");
pub const COINDENOM: Map<&str, bool> = Map::new("coin_denom");
pub const TVL:Map<(&str,&str),Uint128> = Map::new("tvl_config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner:String,
}


pub trait Order {
    fn expires_at(&self) -> Timestamp;

    fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires_at() <= block.time
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Ask {
    pub token_id: String,
    pub seller: String,
    pub list_price: Asset,
    pub expires_at: Timestamp,
    pub collection: String,
}


impl Order for Ask {
    fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
}

/// Primary key for asks: (collection, token_id)
pub type AskKey<'a> = (String, String);
/// Convenience ask key constructor
pub fn ask_key<'a>(collection: &'a String, token_id: &'a String) -> AskKey<'a> {
    (collection.clone(), token_id.clone())
}

/// Defines indices for accessing Asks
pub struct AskIndicies<'a> {
    pub collection: MultiIndex<'a, String, Ask, AskKey<'a>>,
    pub seller: MultiIndex<'a, String, Ask, AskKey<'a>>,
}

impl<'a> IndexList<Ask> for AskIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ask>> + '_> {
        let v: Vec<&dyn Index<Ask>> = vec![&self.collection, &self.seller];
        Box::new(v.into_iter())
    }
}

pub fn asks<'a>() -> IndexedMap<'a, AskKey<'a>, Ask, AskIndicies<'a>> {
    let indexes = AskIndicies {
        collection: MultiIndex::new(|d: &Ask| d.collection.clone(), "asks", "asks__collection"),
        seller: MultiIndex::new(|d: &Ask| d.seller.clone(), "asks", "asks__seller"),
    };
    IndexedMap::new("asks", indexes)
}


/// Represents a bid (offer) on the marketplace
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bid {
    pub collection: String,
    pub token_id: String,
    pub bidder: String,
    pub list_price: Asset,
    pub expires_at: Timestamp,
    pub token_address: Option<String>,
    pub seller: String
}



impl Order for Bid {
    fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
}

/// Primary key for bids: (collection, token_id, bidder)
pub type BidKey = (String, String, String);
/// Convenience bid key constructor
pub fn bid_key(collection: &String, token_id: &String, bidder: &String) -> BidKey {
    (collection.clone(), token_id.clone(), bidder.clone())
}

/// Defines incides for accessing bids
pub struct BidIndicies<'a> {
    pub collection: MultiIndex<'a, String, Bid, BidKey>,
    pub collection_token_id: MultiIndex<'a, (String, String), Bid, BidKey>,
    pub bidder: MultiIndex<'a, String, Bid, BidKey>,
    pub seller: MultiIndex<'a, String, Bid, BidKey>,
}

impl<'a> IndexList<Bid> for BidIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Bid>> + '_> {
        let v: Vec<&dyn Index<Bid>> = vec![
            &self.collection,
            &self.collection_token_id,
            &self.bidder,
            &self.seller,
        ];
        Box::new(v.into_iter())
    }
}

pub fn bids<'a>() -> IndexedMap<'a, BidKey, Bid, BidIndicies<'a>> {
    let indexes = BidIndicies {
        collection: MultiIndex::new(|d: &Bid| d.collection.clone(), "bids", "bids__collection"),
        collection_token_id: MultiIndex::new(
            |d: &Bid| (d.collection.clone(), d.token_id.clone()),
            "bids",
            "bids__collection_token_id",
        ),
        bidder: MultiIndex::new(|d: &Bid| d.bidder.clone(), "bids", "bids__bidder"),
        seller: MultiIndex::new(|d: &Bid| d.seller.clone(), "bids", "bids__seller"),
    };
    IndexedMap::new("bids", indexes)
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    pub denom:String,
    pub amount:Uint128
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UserInfo {
    pub address: String,
    pub portion:Decimal
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SaleInfo {
    pub from :String,
    pub to: String,
    pub denom:String,
    pub amount:Uint128,
    pub time : u64,
    pub nft_address:String,
    pub token_id:String
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]

pub struct CollectionInfo{
    pub nft_address :String,
    pub royalty_portion:Decimal,
    pub sale_id : u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TvlInfo {
   pub denom : String,
   pub amount: Uint128
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SaleType {
    FixedPrice,
    Auction,
}
