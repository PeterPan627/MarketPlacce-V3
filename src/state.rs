use cosmwasm_std::{Uint128, Decimal, Timestamp, BlockInfo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw_storage_plus::{Item,Map,MultiIndex,IndexList,Index,IndexedMap};

pub const CONFIG: Item<State> = Item::new("config_state");
pub const MEMBERS: Map<&str,Vec<UserInfo>> = Map::new("config_members");
pub const COLLECTIONINFO: Map<&str, CollectionInfo> = Map::new("collection_info");
pub const TOKENADDRESS: Map<&str, String> = Map::new("token_address");
pub const COINDENOM: Map<&str, bool> = Map::new("coin_denom");
pub const TVL:Map<(&str,&str),Uint128> = Map::new("tvl_config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: String,
    pub bid_limit: u32,
    pub admin: String
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
     // Cannot include `Timestamp` in index, converted `Timestamp` to `seconds` and stored as `u64`
    pub bidder_expires_at: MultiIndex<'a, (String, u64), Bid, BidKey>,
}

impl<'a> IndexList<Bid> for BidIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Bid>> + '_> {
        let v: Vec<&dyn Index<Bid>> = vec![
            &self.collection,
            &self.collection_token_id,
            &self.bidder,
            &self.seller,
            &self.bidder_expires_at,
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
        bidder_expires_at: MultiIndex::new(
            |d: &Bid| (d.bidder.clone(), d.expires_at.seconds()),
            "bids",
            "bids__bidder_expires_at",
        ),   
    };
    IndexedMap::new("bids", indexes)
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleInfo {
    pub from :String,
    pub to: String,
    pub denom:String,
    pub amount:Uint128,
    pub time : u64,
    pub collection:String,
    pub token_id:String
}

/// Primary key for bids: (collection, token_id, bidder)
pub type SaleHistoryKey = (String, String, u64);
/// Convenience bid key constructor
pub fn sale_history_key(collection: &String, token_id: &String, time: u64) -> SaleHistoryKey {
    (collection.clone(), token_id.clone(), time)
}


/// Defines incides for accessing bids
pub struct SaleHistoryIndicies<'a> {
    pub collection: MultiIndex<'a, String, SaleInfo, SaleHistoryKey>,
    pub collection_token_id: MultiIndex<'a, (String, String), SaleInfo, SaleHistoryKey>,
    pub buyer: MultiIndex<'a, String, SaleInfo, SaleHistoryKey>,
    pub seller: MultiIndex<'a, String, SaleInfo, SaleHistoryKey>
}

impl<'a> IndexList<SaleInfo> for SaleHistoryIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<SaleInfo>> + '_> {
        let v: Vec<&dyn Index<SaleInfo>> = vec![
            &self.collection,
            &self.collection_token_id,
            &self.buyer,
            &self.seller
        ];
        Box::new(v.into_iter())
    }
}

pub fn sale_history<'a>() -> IndexedMap<'a, SaleHistoryKey, SaleInfo, SaleHistoryIndicies<'a>> {
    let indexes = SaleHistoryIndicies {
        collection: MultiIndex::new(|d: &SaleInfo| d.collection.clone(), "sale_history", "sale_history_collection"),
        collection_token_id: MultiIndex::new(
            |d: &SaleInfo| (d.collection.clone(), d.token_id.clone()),
            "sale_history",
            "sale_history__collection_token_id",
        ),
        buyer: MultiIndex::new(
            |d: &SaleInfo| d.to.clone(),
            "sale_history",
            "sale_history__buyer",
        ),
        seller: MultiIndex::new(
            |d: &SaleInfo| d.from.clone(),
            "sale_history",
            "sale_history__seller",
        )
    };
    IndexedMap::new("sale_history", indexes)
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TvlInfo {
   pub denom : String,
   pub amount: Uint128,
   pub collection: String 
}


/// Primary key for bids: (collection, denom)
pub type TvlKey = (String, String);
/// Convenience bid key constructor
pub fn tvl_key(collection: &String, denom: &String) -> TvlKey {
    (collection.clone(), denom.clone())
}


/// Defines incides for accessing bids
pub struct TvlIndicies<'a> {
    pub collection: MultiIndex<'a, String, TvlInfo, TvlKey>,
    pub denom: MultiIndex<'a, String, TvlInfo, TvlKey>,
}

impl<'a> IndexList<TvlInfo> for TvlIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TvlInfo>> + '_> {
        let v: Vec<&dyn Index<TvlInfo>> = vec![
            &self.collection,
            &self.denom,
        ];
        Box::new(v.into_iter())
    }
}

pub fn tvl<'a>() -> IndexedMap<'a, TvlKey, TvlInfo, TvlIndicies<'a>> {
    let indexes = TvlIndicies {
        collection: MultiIndex::new(|d: &TvlInfo| d.collection.clone(), "tvl", "tvl_collection"),
        denom: MultiIndex::new(
            |d: &TvlInfo| d.denom.clone(),
            "tvl",
            "tvl_denom",
        ),
    };
    IndexedMap::new("tvl", indexes)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBid {
    pub collection: String,
    pub bidder: String,
    pub list_price: Asset,
    pub expires_at: Timestamp,
    pub token_address: Option<String>
}

impl Order for CollectionBid {
    fn expires_at(&self) -> Timestamp {
        self.expires_at
    }
}

/// Primary key for bids: (collection, bidder)
pub type CollectionBidKey = (String, String);
/// Convenience collection bid key constructor
pub fn collection_bid_key(collection: &String, bidder: &String) -> CollectionBidKey {
    (collection.clone(), bidder.clone())
}

/// Defines incides for accessing collection bids
pub struct CollectionBidIndicies<'a> {
    pub collection: MultiIndex<'a, String, CollectionBid, CollectionBidKey>,
    pub bidder: MultiIndex<'a, String, CollectionBid, CollectionBidKey>,
    // Cannot include `Timestamp` in index, converted `Timestamp` to `seconds` and stored as `u64`
    pub bidder_expires_at: MultiIndex<'a, (String, u64), CollectionBid, CollectionBidKey>,
}

impl<'a> IndexList<CollectionBid> for CollectionBidIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CollectionBid>> + '_> {
        let v: Vec<&dyn Index<CollectionBid>> = vec![
            &self.collection,
            &self.bidder,
            &self.bidder_expires_at,
        ];
        Box::new(v.into_iter())
    }
}

pub fn collection_bids<'a>(
) -> IndexedMap<'a, CollectionBidKey, CollectionBid, CollectionBidIndicies<'a>> {
    let indexes = CollectionBidIndicies {
        collection: MultiIndex::new(
            |d: &CollectionBid| d.collection.clone(),
            "col_bids",
            "col_bids__collection",
        ),
        bidder: MultiIndex::new(
            |d: &CollectionBid| d.bidder.clone(),
            "col_bids",
            "col_bids__bidder",
        ),
        bidder_expires_at: MultiIndex::new(
            |d: &CollectionBid| (d.bidder.clone(), d.expires_at.seconds()),
            "col_bids",
            "col_bids__bidder_expires_at",
        ),
    };
    IndexedMap::new("col_bids", indexes)
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

pub struct CollectionInfo{
    pub nft_address :String,
    pub royalty_portion:Decimal
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SaleType {
    FixedPrice,
    Auction,
    CollectionBid
}
