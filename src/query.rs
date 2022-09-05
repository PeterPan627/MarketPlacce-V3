use crate::msg::{
    AskCountResponse,  AskResponse, AsksResponse,  BidResponse, BidsResponse,CollectionOffset, QueryMsg, CollectionOffsetBid, SaleHistoryOffset, SaleHistroyResponse, TvlResponse, TvlIndividualResponse, CollectionBidOffset, CollectionBidResponse, CollectionBidsResponse, SaleHistoryOffsetByUser
};
use crate::state::{
    ask_key, asks, bid_key, bids,  BidKey, State, CONFIG, CollectionInfo, COLLECTIONINFO, MEMBERS, UserInfo, sale_history_key, sale_history, tvl,collection_bid_key,collection_bids
};
use cosmwasm_std::{entry_point, to_binary, Addr, Binary, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::{Bound, PrefixBound};

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;


#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;

    match msg {
        QueryMsg::GetStateInfo {} => to_binary(&query_state_info(deps)?),
        QueryMsg::GetMembers {
          address
        } => to_binary(&query_get_members(deps,address)?),
        QueryMsg::GetCollectionInfo {
           address 
          } =>to_binary(&query_collection_info(deps,address)?),
        QueryMsg::Ask {
            collection,
            token_id,
        } => to_binary(&query_ask(deps, collection, token_id)?),
        QueryMsg::Asks {
            collection,
            start_after,
            limit,
        } => to_binary(&query_asks(
            deps,
            collection,
            start_after,
            limit,
        )?),
        QueryMsg::ReverseAsks {
            collection,
            start_before,
            limit,
        } => to_binary(&reverse_query_asks(
            deps,
            collection,
            start_before,
            limit,
        )?),
        QueryMsg::AsksBySeller {
            seller,
            start_after,
            limit,
        } => to_binary(&query_asks_by_seller(
            deps,
            seller,
            start_after,
            limit,
        )?),
        QueryMsg::AskCount { collection } => {
            to_binary(&query_ask_count(deps, collection)?)
        },
        QueryMsg::Bid {
            collection,
            token_id,
            bidder,
        } => to_binary(&query_bid(
            deps,
            collection,
            token_id,
            bidder,
        )?),
        QueryMsg::Bids {
            collection,
            token_id,
            start_after,
            limit,
        } => to_binary(&query_bids(
            deps,
            collection,
            token_id,
            start_after,
            limit,
        )?),
        QueryMsg::BidsByBidder {
            bidder,
            start_after,
            limit,
        } => to_binary(&query_bids_by_bidder(
            deps,
            bidder,
            start_after,
            limit,
        )?),
        QueryMsg::BidsByBidderSortedByExpiration {
            bidder,
            start_after,
            limit,
        } => to_binary(&query_bids_by_bidder_sorted_by_expiry(
            deps,
            bidder,
            start_after,
            limit,
        )?),
        QueryMsg::BidsBySeller {
           seller,
           start_after,
           limit 
         }  => to_binary(&query_bids_by_seller(
            deps,
            seller,
            start_after,
            limit,
        )?),
        QueryMsg::CollectionBid { collection, bidder } => to_binary(&query_collection_bid(
            deps,
            collection,
            bidder,
        )?),
          QueryMsg::CollectionBidsByBidder {
            bidder,
            start_after,
            limit,
        } => to_binary(&query_collection_bids_by_bidder(
            deps,
            bidder,
            start_after,
            limit,
        )?),
        QueryMsg::CollectionBidsByBidderSortedByExpiration {
            bidder,
            start_after,
            limit,
        } => to_binary(&query_collection_bids_by_bidder_sorted_by_expiry(
            deps,
            bidder,
            start_after,
            limit,
        )?),
        QueryMsg::SaleHistoryByCollection {
           collection,
           start_after,
           limit
         } => to_binary(&query_sale_history(
             deps,
             collection,
             start_after,
             limit
           )?),
        QueryMsg::SaleHistoryByTokenId { 
          collection,
          token_id,
          start_after, 
          limit 
        } => to_binary(&query_sale_history_by_token_id(
          deps,
          collection,
          token_id,
          start_after,
          limit
        )?),
        QueryMsg::GetTvlbyCollection {
          collection,
          start_after ,
          limit
        } => to_binary(&query_tvl_by_collection(
          deps,
          collection, 
          start_after, 
          limit)?),
        QueryMsg::GetTvlByDenom { 
          denom, 
          start_after, 
          limit 
        } => to_binary(&query_tvl_by_denom(
          deps,
          denom,
          start_after,
          limit
        )?),
        QueryMsg::GetTvlIndividaul { 
          collection, 
          denom 
        } => to_binary(&query_tvl_by_individual(
          deps,
          collection,
          denom
        )?),
        QueryMsg::GetSaleHistoryByBuyer { 
          buyer, 
          start_after, 
          limit 
        } => to_binary(&query_sale_history_by_buyer(
          deps,
          buyer,
          start_after,
          limit
        )?),
        QueryMsg::GetSaleHistoryBySeller { 
          seller, 
          start_after, 
          limit 
        } => to_binary(&query_sale_history_by_seller(
          deps,
          seller,
          start_after,
          limit
        )?),
        QueryMsg::CollectionBidByCollection {collection, start_after, limit } => to_binary(&query_collection_bid_by_collection(
          deps,
          collection,
          start_after,
          limit
        )?)  
    }
}

pub fn query_state_info(deps:Deps) -> StdResult<State>{
    let state =  CONFIG.load(deps.storage)?;
    Ok(state)
}

pub fn query_collection_info(deps:Deps,address:String) -> StdResult<CollectionInfo>{
    let collection_info =  COLLECTIONINFO.load(deps.storage,&address)?;
    Ok(collection_info)
}


pub fn query_get_members(deps:Deps,address:String) -> StdResult<Vec<UserInfo>>{
    let members = MEMBERS.load(deps.storage,&address)?;
    Ok(members)
}




pub fn query_ask(deps: Deps, collection: String, token_id: String) -> StdResult<AskResponse> {
    let ask = asks().may_load(deps.storage, ask_key(&collection, &token_id))?;

    Ok(AskResponse { ask })
}


pub fn query_asks(
    deps: Deps,
    collection: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let asks = asks()
        .idx
        .collection
        .prefix(collection.clone())
        .range(
            deps.storage,
            Some(Bound::exclusive((
                collection,
                start_after.unwrap_or_default(),
            ))),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AsksResponse { asks })
}

pub fn reverse_query_asks(
    deps: Deps,
    collection: String,
    start_before: Option<String>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let asks = asks()
        .idx
        .collection
        .prefix(collection.clone())
        .range(
            deps.storage,
            None,
            Some(Bound::exclusive((
                collection,
                start_before.unwrap_or_default(),
            ))),
            Order::Descending,
        )
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AsksResponse { asks })
}

pub fn query_ask_count(deps: Deps, collection: String) -> StdResult<AskCountResponse> {
    let count = asks()
        .idx
        .collection
        .prefix(collection)
        .keys_raw(deps.storage, None, None, Order::Ascending)
        .count() as u32;

    Ok(AskCountResponse { count })
}

pub fn query_asks_by_seller(
    deps: Deps,
    seller: String,
    start_after: Option<CollectionOffset>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        deps.api.addr_validate(&start.collection)?;
        let collection = start.collection;
        Some(Bound::exclusive(ask_key(&collection, &start.token_id)))
    } else {
        None
    };

    let asks = asks()
        .idx
        .seller
        .prefix(seller)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AsksResponse { asks })
}


pub fn query_bid(
    deps: Deps,
    collection: String,
    token_id: String,
    bidder: String,
) -> StdResult<BidResponse> {
    let bid = bids().may_load(deps.storage, (collection, token_id, bidder))?;

    Ok(BidResponse { bid })
}

pub fn query_bids_by_bidder(
    deps: Deps,
    bidder: String,
    start_after: Option<CollectionOffset>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        deps.api.addr_validate(&start.collection)?;
        let collection = start.collection;
        Some(Bound::exclusive(bid_key(
            &collection,
            &start.token_id,
            &bidder,
        )))
    } else {
        None
    };

    let bids = bids()
        .idx
        .bidder
        .prefix(bidder)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}

pub fn query_bids(
    deps: Deps,
    collection: String,
    token_id: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let bids = bids()
        .idx
        .collection_token_id
        .prefix((collection, token_id))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}

pub fn query_bids_by_bidder_sorted_by_expiry(
    deps: Deps,
    bidder: String,
    start_after: Option<CollectionOffset>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = match start_after {
        Some(offset) => {
            deps.api.addr_validate(&offset.collection)?;
            let collection = offset.collection;
            let bid = query_bid(deps, collection.clone(), offset.token_id.clone(), bidder.clone())?;
            match bid.bid {
                Some(bid) => Some(Bound::exclusive((
                    bid.expires_at.seconds(),
                    bid_key(&collection, &offset.token_id, &bidder),
                ))),
                None => None,
            }
        }
        None => None,
    };

    let bids = bids()
        .idx
        .bidder_expires_at
        .sub_prefix(bidder)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}


pub fn query_bids_by_seller(
    deps: Deps,
    seller: String,
    start_after: Option<CollectionOffsetBid>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        deps.api.addr_validate(&start.collection)?;
        let collection = start.collection;
        Some(Bound::exclusive(bid_key(&collection, &start.token_id, &start.bidder)))
    } else {
        None
    };

    let bids = bids()
        .idx
        .seller
        .prefix(seller)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}


pub fn query_sale_history(
    deps: Deps,
    collection: String,
    start_after: Option<SaleHistoryOffset>,
    limit: Option<u32>,
) -> StdResult<SaleHistroyResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        Some(Bound::exclusive(sale_history_key(
            &collection,
            &start.token_id,
            start.time,
        )))
    } else {
        None
    };

    let sale_history = sale_history()
        .idx
        .collection
        .prefix(collection)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(SaleHistroyResponse {  sale_history })
}

pub fn query_sale_history_by_token_id(
    deps: Deps,
    collection: String,
    token_id: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<SaleHistroyResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = if let Some(start) = start_after {
        Some(Bound::exclusive(sale_history_key(
            &collection,
            &token_id,
            start,
        )))
    } else {
        None
    };

    let sale_history = sale_history()
        .idx
        .collection_token_id
        .prefix((collection, token_id))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(SaleHistroyResponse { sale_history })
}


pub fn query_sale_history_by_buyer(
    deps: Deps,
    buyer: String,
    start_after: Option<SaleHistoryOffsetByUser>,
    limit: Option<u32>,
) -> StdResult<SaleHistroyResponse> {
   let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        Some(Bound::exclusive(sale_history_key(
            &start.collection,
            &start.token_id,
            start.time,
        )))
    } else {
        None
    };

    let sale_history = sale_history()
        .idx
        .buyer
        .prefix(buyer)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(SaleHistroyResponse {  sale_history })
}


pub fn query_sale_history_by_seller(
    deps: Deps,
    seller: String,
    start_after: Option<SaleHistoryOffsetByUser>,
    limit: Option<u32>,
) -> StdResult<SaleHistroyResponse> {
   let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        Some(Bound::exclusive(sale_history_key(
            &start.collection,
            &start.token_id,
            start.time,
        )))
    } else {
        None
    };

    let sale_history = sale_history()
        .idx
        .seller
        .prefix(seller)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(SaleHistroyResponse {  sale_history })
}


pub fn query_tvl_by_collection(
    deps: Deps,
    collection: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TvlResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let tvl = tvl()
        .idx
        .collection
        .prefix(collection.clone())
        .range(
            deps.storage,
            Some(Bound::exclusive((
                collection,
                start_after.unwrap_or_default(),
            ))),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(TvlResponse { tvl })
}



pub fn query_tvl_by_denom(
    deps: Deps,
    denom: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TvlResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let tvl = tvl()
        .idx
        .denom
        .prefix(denom.clone())
        .range(
            deps.storage,
            Some(Bound::exclusive((
              start_after.unwrap_or_default(),  
              denom
            ))),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(TvlResponse { tvl })
}

pub fn query_tvl_by_individual(
  deps: Deps,
  collection: String,
  denom: String
) -> StdResult<TvlIndividualResponse>{
  let tvl = tvl().may_load(deps.storage, (collection,denom))?;
  
  Ok(TvlIndividualResponse{ tvl })
}


pub fn query_collection_bid(
    deps: Deps,
    collection: String,
    bidder: String,
) -> StdResult<CollectionBidResponse> {
    let bid = collection_bids().may_load(deps.storage, collection_bid_key(&collection, &bidder))?;

    Ok(CollectionBidResponse { bid })
}


pub fn query_collection_bids_by_bidder(
    deps: Deps,
    bidder: String,
    start_after: Option<CollectionOffset>,
    limit: Option<u32>,
) -> StdResult<CollectionBidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start: Option<Bound<(String, String)>> = match start_after {
        Some(offset) => {
             deps.api.addr_validate(&offset.collection)?;
             let collection = offset.collection;
             Some(Bound::exclusive((collection, bidder.clone())))
        }
        None => None,
    };
    let bids = collection_bids()
        .idx
        .bidder
        .prefix(bidder)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionBidsResponse { bids })
}

pub fn query_collection_bids_by_bidder_sorted_by_expiry(
    deps: Deps,
    bidder: String,
    start_after: Option<CollectionBidOffset>,
    limit: Option<u32>,
) -> StdResult<CollectionBidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = match start_after {
        Some(offset) => {
            deps.api.addr_validate(&offset.bidder)?;
            let bidder = offset.bidder;
            deps.api.addr_validate(&offset.collection)?;
            let collection = offset.collection;
            let collection_bid =
                query_collection_bid(deps, collection.clone(), bidder.clone())?.bid;
            let bound = match collection_bid {
                Some(collection_bid) => Some(Bound::exclusive((
                    collection_bid.expires_at.seconds(),
                    (collection, bidder),
                ))),
                None => None,
            };
            bound
        }
        None => None,
    };

    let bids = collection_bids()
        .idx
        .bidder_expires_at
        .sub_prefix(bidder)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionBidsResponse { bids })
}



pub fn query_collection_bid_by_collection(
    deps: Deps,
    collection: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<CollectionBidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start: Option<Bound<(String, String)>> = match start_after {
        Some(start) => {
             deps.api.addr_validate(&collection)?;
             let collection = collection.clone();
             Some(Bound::exclusive((collection, start.clone())))
        }
        None => None,
    };
    let bids = collection_bids()
        .idx
        .collection
        .prefix(collection)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionBidsResponse { bids })
}
