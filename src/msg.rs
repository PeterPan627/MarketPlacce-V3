
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::{Asset,UserInfo, TvlInfo, SaleInfo, SaleType, Ask, Bid, CollectionBid};
use crate::package::QueryOfferingsResult;
use cosmwasm_std::{Decimal, Timestamp, Uint128};
use cw721::Cw721ReceiveMsg;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
  pub owner: String,
  pub admin: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
    WithdrawNft{
        nft_address: String,
        token_id: String
    },
    UpdateAskPrice{
        nft_address: String,
        token_id: String,
        list_price: Asset,
        token_address: Option<String>
    },
    Receive(Cw20ReceiveMsg), 
    SetBidCoin{
        nft_address:String, 
        expire: Timestamp, 
        sale_type: SaleType, 
        token_id: Option<String>, 
        list_price:Asset
    },
    RemoveBid{
        nft_address: String,
        token_id: String
    },
    AcceptBid{
        nft_address: String,
        token_id: String,
        bidder: String,
    },
    RemoveCollectionBid { 
        nft_address: String 
    },
    AcceptCollectionBid {
        nft_address: String,
        token_id: String,
        bidder: String,
    },
    /// Priviledged operation to change the active state of an ask when an NFT is transferred
    ChangeOwner{
        address: String
    },
    AddTokenAddress{
        symbol: String,address: String
    },
    AddCoin{
        symbol: String
    },
    AddCollection{
        royalty_portion:Decimal,
        members:Vec<UserInfo>,
        nft_address:String
    },
    UpdateCollection{
        royalty_portion:Decimal,
        members:Vec<UserInfo>,
        nft_address:String
    },
    FixNft{
        address:String,
        token_id:String
    },
    SetOfferings{
        address:String,
        offering:Vec<QueryOfferingsResult>
    },
    SetTvl{
        address:String,
        tvl:Vec<TvlInfo>
    },
    Migrate{
        address:String,
        dest:String,
        token_id : Vec<String>
    },
    SetSaleHistory{
        address:String,
        history:Vec<SaleInfo>
    },
    SetBidLimit{bid_limit:u32},
    Withdraw{
        token_amount: Uint128,
        coin_amount: Uint128,
        token_address: String,
        coin_denom: String
    },
    ChangeAdmin{
        address:String
    },
    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns a human-readable representation of the arbiter.
    GetStateInfo {},
    GetMembers{address:String},
    GetCollectionInfo{address:String},
    /// Get the current ask for specific NFT
    /// Return type: `CurrentAskResponse`
    Ask{collection:String, token_id:String},
    /// Get all asks for a collection
    /// Return type: `AsksResponse`
    //start_after is  token_id
    Asks {
        collection: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get all asks for a collection in reverse
    /// Return type: `AsksResponse`
    ReverseAsks {
        collection: String,
        start_before: Option<String>,
        limit: Option<u32>,
    },
      /// Count of all asks
    /// Return type: `AskCountResponse`
    AskCount { collection: String },
    /// Get all asks by seller
    /// Return type: `AsksResponse`
    AsksBySeller {
        seller: String,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
       /// Get data for a specific bid
    /// Return type: `BidResponse`
    Bid {
        collection: String,
        token_id: String,
        bidder: String,
    },
    /// Get all bids by a bidder
    /// Return type: `BidsResponse`
    BidsByBidder {
        bidder: String,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
      /// Get all bids for a specific NFT
    /// Return type: `BidsResponse`
    /// start after is bidder
    Bids {
        collection: String,
        token_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    BidsByBidderSortedByExpiration {
        bidder: String,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
     /// Get all bids by a bidder
    /// Return type: `BidsResponse`
    BidsBySeller {
        seller: String,
        start_after: Option<CollectionOffsetBid>,
        limit: Option<u32>,
    },
      /// Get data for a specific collection bid
    /// Return type: `CollectionBidResponse`
    CollectionBid {
        collection: String,
        bidder: String,
    },

    CollectionBidByCollection{
        collection:String,
        start_after: Option<String>,
        limit: Option<u32>
    },
    /// Get all collection bids by a bidder
    /// Return type: `CollectionBidsResponse`
    CollectionBidsByBidder {
        bidder: String,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
    /// Get all collection bids by a bidder, sorted by expiration
    /// Return type: `CollectionBidsResponse`
    CollectionBidsByBidderSortedByExpiration {
        bidder: String,
        start_after: Option<CollectionBidOffset>,
        limit: Option<u32>,
    },
    SaleHistoryByCollection{
        collection: String,
        start_after: Option<SaleHistoryOffset>,
        limit: Option<u32>
    },
    //start after is time
    SaleHistoryByTokenId{
        collection: String,
        token_id: String,
        start_after: Option<u64>,
        limit: Option<u32>
    },
    //start after is denom
    GetTvlbyCollection{
        collection: String,
        start_after: Option<String>,
        limit: Option<u32>
    },
    //start after is collection
    GetTvlByDenom{
        denom: String,
        start_after: Option<String>,
        limit: Option<u32>
    },
    GetTvlIndividaul{
        collection:String,
        denom:String
    },
    GetSaleHistoryBySeller{
        seller: String,
        start_after: Option<SaleHistoryOffsetByUser>,
        limit: Option<u32>
    },
    GetSaleHistoryByBuyer{
        buyer: String,
        start_after: Option<SaleHistoryOffsetByUser>,
        limit: Option<u32>
    }
}

/// Offset for collection pagination
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionOffset {
    pub collection: String,
    pub token_id: String,
}


/// Offset for collection pagination bid by seller
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionOffsetBid {
    pub collection: String,
    pub token_id: String,
    pub bidder: String
}

/// Offset for collection bid pagination
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBidOffset {
    pub price: Asset,
    pub collection: String,
    pub bidder: String,
}

impl CollectionBidOffset {
    pub fn new(price: Asset, collection: String, bidder: String) -> Self {
        CollectionBidOffset {
            price,
            collection,
            bidder,
        }
    }
}



/// Salehistory offset for the pagination sale histroy by collection
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleHistoryOffset {
    pub token_id: String,
    pub time: u64
}

/// Salehistory offset for the pagination sale histroy by collection
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleHistoryOffsetByUser {
    pub collection:String,
    pub token_id: String,
    pub time: u64
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SellNft {
    pub list_price: Asset,
    pub expire: Timestamp,
    pub token_address: Option<String>
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BuyNft {
    pub nft_address : String,
    pub expire: Timestamp,
    pub sale_type: SaleType,
    pub token_id: Option<String>
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskResponse {
    pub ask: Option<Ask>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AsksResponse {
    pub asks: Vec<Ask>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskCountResponse {
    pub count: u32,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidResponse {
    pub bid: Option<Bid>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidsResponse {
    pub bids: Vec<Bid>,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SaleHistroyResponse {
    pub sale_history: Vec<SaleInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TvlResponse {
    pub tvl: Vec<TvlInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TvlIndividualResponse {
    pub tvl: Option<TvlInfo>,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBidResponse {
    pub bid: Option<CollectionBid>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBidsResponse {
    pub bids: Vec<CollectionBid>,
}