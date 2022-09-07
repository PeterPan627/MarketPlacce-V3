use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,from_binary,
    StdResult, Uint128,CosmosMsg,WasmMsg,Decimal,BankMsg,Storage, Timestamp
};

use cw2::set_contract_version;
use cw20::{ Cw20ExecuteMsg,Cw20ReceiveMsg};
use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg};

use crate::error::{ContractError};
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg,SellNft, BuyNft};
use crate::query::query_bids;
use crate::state::{
    State,CONFIG,Asset,UserInfo, MEMBERS,SaleInfo, COLLECTIONINFO, CollectionInfo, TOKENADDRESS,  TvlInfo, COINDENOM, SaleType
};
use crate::state::{
    Ask,asks,AskKey,ask_key,Order,Bid, bids, BidKey, bid_key, sale_history, sale_history_key, tvl_key, tvl, collection_bid_key, collection_bids, CollectionBid
};
use crate::package::{QueryOfferingsResult};


const CONTRACT_NAME: &str = "Hope_Market_Place";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_QUERY_LIMIT: u32 = 30;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        owner:msg.owner,
        bid_limit: 10,
        admin: msg.admin
    };
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(msg) =>execute_receive_nft(deps,env,info,msg),
        ExecuteMsg::UpdateAskPrice { 
            nft_address, 
            token_id ,
            list_price,
            token_address } => execute_update_ask_price(
            deps,
            env,
            info, 
            nft_address, 
            token_id,
            list_price,
            token_address),
        ExecuteMsg::WithdrawNft {
            nft_address,
            token_id 
        } => execute_withdraw(
            deps,
            env,
            info,
            nft_address,
            token_id),
        ExecuteMsg::Receive(msg) =>execute_receive(
            deps,
            env,
            info,
            msg),
        ExecuteMsg::SetBidCoin { 
            nft_address, 
            expire, 
            sale_type, 
            token_id, 
            list_price 
        } => execute_bid_with_coin(
            deps, 
            env, 
            info, 
            nft_address, 
            token_id, 
            expire, 
            sale_type, 
            list_price),
        ExecuteMsg::RemoveBid { 
            nft_address, 
            token_id 
        } => execute_remove_bid(
            deps,
            env,
            info,
            nft_address,
            token_id
        ),
        ExecuteMsg::AcceptBid { 
            nft_address, 
            token_id, 
            bidder 
        } => execute_accept_bid(
            deps,
            env,
            info,
            nft_address,
            token_id,
            bidder
        ),
        ExecuteMsg::RemoveCollectionBid {
             nft_address
        } => execute_remove_collection_bid(
            deps,
            env,
            info,
            nft_address
        ),
        ExecuteMsg::AcceptCollectionBid { 
            nft_address,
            token_id, 
            bidder 
        } => execute_accept_collection_bid(
            deps,
            env,
            info,
            nft_address,
            token_id,
            bidder
        ),
        ExecuteMsg::AddTokenAddress {
             symbol, 
             address 
        }  => execute_token_address(
            deps,
            env,
            info,
            symbol,
            address),
        ExecuteMsg::AddCoin {
             symbol
        } => execute_add_coin(
            deps,
            env,
            info,
            symbol
        ),
        ExecuteMsg::ChangeOwner {
             address 
        } =>execute_change_owner(
            deps,
            env,
            info,
            address),
        ExecuteMsg::ChangeAdmin { 
          address 
        } => execute_change_admin(
          deps,
          env,
          info,
          address
        ),
        ExecuteMsg::AddCollection { 
            royalty_portion, 
            members,
            nft_address ,
        } =>execute_add_collection(
            deps,
            env,
            info,
            royalty_portion,
            members,
            nft_address),
        ExecuteMsg::UpdateCollection { 
            royalty_portion, 
            members,
            nft_address 
        } =>execute_update_collection(deps,env,info,royalty_portion,members,nft_address),
        ExecuteMsg::FixNft{address,token_id} =>execute_fix_nft(deps,env,info,address,token_id),
        ExecuteMsg::SetOfferings { address, offering }=>execute_set_offerings(deps,env,info,address,offering),
        ExecuteMsg::SetTvl { address, tvl } =>execute_set_tvl(deps,env,info,address,tvl),
        ExecuteMsg::Migrate {
             address, 
             dest, 
             token_id }=>execute_migrate(deps,env,info,address,dest,token_id),
        ExecuteMsg::SetSaleHistory { address, history }=>execute_history(deps,env,info,address,history),
        ExecuteMsg::SetBidLimit { bid_limit } => execute_bid_limit(deps,env,info,bid_limit),
        ExecuteMsg::Withdraw { token_amount, coin_amount ,token_address, coin_denom } => execute_withdraw_coin(deps,env,info,token_amount,coin_amount,token_address,coin_denom)
 }
}


fn execute_receive_nft(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    rcv_msg: Cw721ReceiveMsg,
)-> Result<Response, ContractError> {

    let collection_info = COLLECTIONINFO.may_load(deps.storage, &info.sender.to_string())?;

    //Collection Validation Check
    if collection_info.is_none() {
        return Err(ContractError::WrongNFTContractError { });
    }

    let msg:SellNft = from_binary(&rcv_msg.msg)?;
    let nft_address = info.sender.to_string();
    let token_address = msg.token_address;

    //Coin and Token validation
    if token_address.is_none(){
        match COINDENOM.may_load(deps.storage, &msg.list_price.denom)?{
            Some(denom) =>{
               if !denom{
                return Err(ContractError::WrongCoinDenom {  })
               }
            },
            None =>{
                return Err(ContractError::WrongCoinDenom {  })
            }
        }
    } else{
        //Validate Configuration, token_address can not be existed if the list price is set as coin
        let token_address = token_address.unwrap();
        let token_denom = TOKENADDRESS.may_load(deps.storage, &token_address)?;
        match  token_denom{
          Some(denom) =>{
            if denom != msg.list_price.denom{
                return Err(ContractError::WrongTokenContractError {  })
            }
          }
          None =>{
                return Err(ContractError::WrongTokenContractError {  })
           }
        }   
    }

    //Save ask
    let ask = Ask {
        token_id: rcv_msg.token_id.clone(),
        seller: deps.api.addr_validate(&rcv_msg.sender)?.to_string(),
        list_price: msg.list_price.clone(),
        expires_at: msg.expire,
        collection: nft_address,
    };

    if ask.is_expired(&env.block){
        return Err(ContractError::AskExpired {  })
    }

    store_ask(deps.storage, &ask)?;

    Ok(Response::new()
        .add_attribute("action", "Put NFT on Sale")
        .add_attribute("token_id", rcv_msg.token_id)
        .add_attribute("seller", rcv_msg.sender))
}

fn execute_receive(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    rcv_msg: Cw20ReceiveMsg,
)-> Result<Response, ContractError> {

    let state = CONFIG.load(deps.storage)?;
    let bid_limit = state.bid_limit;

    let token_symbol = TOKENADDRESS.may_load(deps.storage, &info.sender.to_string())?;

    //token symbol validation
    if token_symbol == None{
        return Err(ContractError::WrongTokenContractError {  })
    }
    let token_symbol = token_symbol.unwrap();

    let msg:BuyNft = from_binary(&rcv_msg.msg)?;
    let nft_address = msg.nft_address;
    let token_id = msg.token_id;
    let token_address = info.sender.to_string();

    //Collection Validation
    deps.api.addr_validate(&nft_address)?;
    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }

    let collection_info = collection_info.unwrap();

  
    //Bid or Buy with fixed_price
    match msg.sale_type{
        SaleType::Auction => {
            if token_id.is_none() {
                return Err(ContractError::WrongConfig {  })
            }
            let token_id = token_id.unwrap();
            //Get ask and bid keys
            let bidder = rcv_msg.sender;
            let bid_key = bid_key(&nft_address, &token_id, &bidder);
            let ask_key = ask_key(&nft_address, &token_id);

            //Ask validation
            let existing_ask = asks().may_load(deps.storage, ask_key.clone())?;
            match existing_ask.clone(){
                Some(ask) =>{
                    if ask.is_expired(&env.block) {
                        return Err(ContractError::AskExpired {  })
                    }
                },
                None =>{
                    return Err(ContractError::NoSuchAsk {  })
                }
            }
            
            let existing_bids_token = query_bids(deps.as_ref(), nft_address.clone(), token_id.clone(), None, Some(MAX_QUERY_LIMIT))?;
            

            let mut messages: Vec<CosmosMsg> = Vec::new();
            
            //bid count check
            if existing_bids_token.bids.len() >= bid_limit as usize{
                return Err(ContractError::BidCountExpired {  })
            }
            
            //Refund money if this bidder bided for this token in the past
            let existing_bid = bids().may_load(deps.storage, bid_key.clone())?;
            if !existing_bid.is_none(){
                let bid = existing_bid.unwrap();
                bids().remove(deps.storage, bid_key)?;
                //refund money of previous bid to the bidder
                if bid.token_address.is_none(){
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                             to_address: bidder.clone(),
                             amount: vec![Coin{ denom:bid.list_price.denom, amount:bid.list_price.amount}] 
                        }))
                }
                else{
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                             contract_addr: bid.token_address.unwrap(),
                             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                 recipient: bidder.clone(),
                                 amount: bid.list_price.amount })?,
                             funds: vec![] }))
                }
            }   

            //Save the bid
            let bid = Bid{
                collection: nft_address,
                token_id: token_id.clone(),
                bidder: bidder.clone(),
                token_address: Some(token_address),
                list_price: Asset { denom: token_symbol, amount: rcv_msg.amount },
                expires_at: msg.expire,
                seller: existing_ask.unwrap().seller
            };
            if bid.is_expired(&env.block){
                return Err(ContractError::BidExpired {  })
            }
            store_bid(deps.storage, &bid)?;

            if messages.len() >0 {
                Ok(Response::new()
                    .add_attribute("action", "Bid for the auction")
                    .add_attribute("bidder", bidder)
                    .add_attribute("token_id", token_id)
                    .add_messages(messages)
                )
            }
            else{
                 Ok(Response::new()
                    .add_attribute("action", "Bid for the auction")
                    .add_attribute("bidder", bidder)
                    .add_attribute("token_id", token_id)
                )
            }
        }

        SaleType::CollectionBid => {
           if !token_id.is_none(){
            return Err(ContractError::WrongConfig {  });
           }

            let mut messages:Vec<CosmosMsg> = Vec::new();
            let bidder = rcv_msg.sender;
            let key = collection_bid_key(&nft_address, &bidder);
            // println!("{:?}",key);

            let existing_bid = collection_bids().may_load(deps.storage, key.clone())?;
            if let Some(bid) = existing_bid{
                collection_bids().remove(deps.storage, key.clone())?;
             
                if bid.token_address.is_none(){
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                             to_address: bidder.clone(),
                             amount: vec![Coin{ denom:bid.list_price.denom.clone(), amount:bid.list_price.amount}] 
                        }))
                }
                else{
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                             contract_addr: bid.token_address.unwrap(),
                             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                 recipient: bidder.clone(),
                                 amount: bid.list_price.amount })?,
                             funds: vec![] }))
                }
            }

            let collection_bid = CollectionBid{
                collection: nft_address,
                bidder: bidder.clone(),
                token_address: Some(token_address),
                list_price: Asset { 
                    denom: token_symbol,
                    amount: rcv_msg.amount 
                },
                expires_at: msg.expire
            };
            
            if collection_bid.is_expired(&env.block){
                return Err(ContractError::BidExpired {  })
            }

            collection_bids().save(deps.storage, key, &collection_bid)?;

             if messages.len() >0 {
                Ok(Response::new()
                    .add_attribute("action", "Collection Bid for the auction")
                    .add_attribute("bidder", bidder)
                    .add_messages(messages)
                )
            }
            else{
                 Ok(Response::new()
                    .add_attribute("action", "Collection Bid for the auction")
                    .add_attribute("bidder", bidder)
                )
            }
        }

        SaleType::FixedPrice =>{
            if token_id.is_none() {
                return Err(ContractError::WrongConfig {  })
            }
            let token_id = token_id.unwrap();
            //Get ask and bid keys
            let bidder = rcv_msg.sender;
            let ask_key = ask_key(&nft_address, &token_id);

            //Ask validation
            let existing_ask = asks().may_load(deps.storage, ask_key.clone())?;
            match existing_ask.clone(){
                Some(ask) =>{
                    if ask.is_expired(&env.block) {
                        return Err(ContractError::AskExpired {  })
                    }
                },
                None =>{
                    return Err(ContractError::NoSuchAsk {  })
                }
            }
            
            let existing_bids_token = query_bids(deps.as_ref(), nft_address.clone(), token_id.clone(), None, Some(MAX_QUERY_LIMIT))?;
            

            let mut messages: Vec<CosmosMsg> = Vec::new();
            let existing_ask = existing_ask.unwrap();
            asks().remove(deps.storage, ask_key)?;

            //token amount validation for the fixed price sale
            if token_symbol != existing_ask.list_price.denom{
                return Err(ContractError::NotEnoughFunds {  })
            }
            if rcv_msg.amount != existing_ask.list_price.amount{
                return Err(ContractError::NotEnoughFunds {  })
            }

            //bid information for this token_id;
            for bid in existing_bids_token.bids{
                match bid.token_address{
                    Some(token_address) =>{
                        messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
                            contract_addr: token_address.clone(),
                            msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                                recipient: bid.bidder.clone(),
                                amount: bid.list_price.amount })?,
                            funds: vec![] }));
                    }   
                    None =>{
                        messages.push(CosmosMsg::Bank(BankMsg::Send {
                            to_address: bid.bidder.clone(),
                            amount: vec![Coin{denom: bid.list_price.denom, amount: bid.list_price.amount}] }));               
                    }

                }
                bids().remove(deps.storage, (nft_address.clone(), token_id.clone(), bid.bidder))?;                
            }

            update_sale_history_tvl(
                deps.storage,
                env,
                info,
                existing_ask.seller.clone(),
                bidder.clone(),
                nft_address.clone(),
                token_id.clone(),
                existing_ask.list_price.clone()
            )?;

            distribute_money(
                deps.as_ref(),
                nft_address, 
                collection_info.royalty_portion, 
                existing_ask.seller, 
                bidder.clone(), 
                existing_ask.list_price, 
                Some(token_address), 
                token_id, 
                &mut messages
            )?;
            
            Ok(Response::new()
                .add_attribute("action", "buy Nft as fixed price with token")
                .add_attribute("bidder", bidder)
                .add_messages(messages))
        }
    }

}

fn execute_bid_with_coin(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    nft_address : String,
    token_id: Option<String>,
    expire: Timestamp,
    sale_type: SaleType,   
    list_price: Asset
) -> Result<Response, ContractError> {

    let state = CONFIG.load(deps.storage)?;
    let bid_limit = state.bid_limit;
  
    //Collection Validation
    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongNFTContractError {  })
    }

    let collection_info = collection_info.unwrap();

    let is_registered_coin = COINDENOM.may_load(deps.storage, &list_price.denom)?;
    if is_registered_coin.is_none(){
      return Err(ContractError::WrongCoinDenom {  })
    }

    //Coin Validation to check if the sent amount is the same as the list price
    let amount = info  
        .funds
        .iter()
        .find(|c| c.denom == list_price.denom.clone())
        .map(|c| Uint128::from(c.amount))
        .unwrap_or_else(Uint128::zero);
    
    if list_price.amount != amount{
        return Err(ContractError::NotEnoughFunds {  })
    }

     //Bid or Buy with fixed_price
    match sale_type{
        SaleType::Auction => {

            if token_id.is_none(){
                return Err(ContractError::WrongConfig {  });
            }
            let token_id = token_id.unwrap();

            let bidder = info.sender.to_string();
            let bid_key = bid_key(&nft_address, &token_id, &bidder);
            let ask_key = ask_key(&nft_address, &token_id);

            //Ask validation
            let existing_ask = asks().may_load(deps.storage, ask_key.clone())?;
            match existing_ask.clone() {
                Some(ask) =>{
                    if ask.is_expired(&env.block) {
                        return Err(ContractError::AskExpired {  })
                    }
                },
                None =>{
                    return Err(ContractError::NoSuchAsk {  })
                }
            }

            let mut messages: Vec<CosmosMsg> = Vec::new();

            //bid count check
            let existing_bids_token = query_bids(deps.as_ref(), nft_address.clone(), token_id.clone(), None, Some(MAX_QUERY_LIMIT))?;
            if existing_bids_token.bids.len() >= bid_limit as usize{
                return Err(ContractError::BidCountExpired {  })
            }
            
            //Refund money if this bidder bided for this token in the past
            let existing_bid = bids().may_load(deps.storage, bid_key.clone())?;
            if !existing_bid.is_none(){
                let bid = existing_bid.unwrap();
                bids().remove(deps.storage, bid_key)?;
                //refund money of previous bid to the bidder
                if bid.token_address.is_none(){
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                             to_address: bidder.clone(),
                             amount: vec![Coin{ denom:bid.list_price.denom.clone(), amount:bid.list_price.amount}] 
                        }))
                }
                else{
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                             contract_addr: bid.token_address.unwrap(),
                             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                 recipient: bidder.clone(),
                                 amount: bid.list_price.amount })?,
                             funds: vec![] }))
                }
            }   

            //Save the bid
            let bid = Bid{
                collection: nft_address,
                token_id: token_id.clone(),
                bidder: bidder.clone(),
                token_address: None,
                list_price: list_price.clone(),
                expires_at: expire,
                seller: existing_ask.unwrap().seller
            };
            if bid.is_expired(&env.block){
                return Err(ContractError::BidExpired {  })
            }
            store_bid(deps.storage, &bid)?;

           if messages.len() >0 {
                Ok(Response::new()
                    .add_attribute("action", "Bid for the auction")
                    .add_attribute("bidder", bidder)
                    .add_attribute("token_id", token_id)
                    .add_messages(messages)
                )
            }
            else{
                 Ok(Response::new()
                    .add_attribute("action", "Bid for the auction")
                    .add_attribute("bidder", bidder)
                    .add_attribute("token_id", token_id)
                )
            }
    
        }
        
        SaleType::CollectionBid =>{
            let mut messages:Vec<CosmosMsg> = Vec::new();
            let bidder = info.sender.to_string();
            let key = collection_bid_key(&nft_address, &bidder);

            if !token_id.is_none(){
              return Err(ContractError::WrongConfig {  });
           }

            let existing_bid = collection_bids().may_load(deps.storage, key.clone())?;
            if let Some(bid) = existing_bid{
                collection_bids().remove(deps.storage, key.clone())?;
               
                if bid.token_address.is_none(){
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                             to_address: bidder.clone(),
                             amount: vec![Coin{ denom:bid.list_price.denom.clone(), amount:bid.list_price.amount}] 
                        }))
                }
                else{
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                             contract_addr: bid.token_address.unwrap(),
                             msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                 recipient: bidder.clone(),
                                 amount: bid.list_price.amount })?,
                             funds: vec![] }))
                }
            }

            let collection_bid = CollectionBid{
                collection: nft_address,
                bidder: bidder.clone(),
                token_address: None,
                list_price: list_price.clone(),
                expires_at: expire
            };
            
            if collection_bid.is_expired(&env.block){
                return Err(ContractError::BidExpired {  })
            }

            collection_bids().save(deps.storage, key, &collection_bid)?;

             if messages.len() >0 {
                Ok(Response::new()
                    .add_attribute("action", "Collection Bid for the auction")
                    .add_attribute("bidder", bidder)
                    .add_messages(messages)
                )
            }
            else{
                 Ok(Response::new()
                    .add_attribute("action", "Collection Bid for the auction")
                    .add_attribute("bidder", bidder)
                )
            }
        }
        SaleType::FixedPrice =>{
            let mut messages: Vec<CosmosMsg> = Vec::new();

            if token_id.is_none(){
                return Err(ContractError::WrongConfig {  });
            }
            let token_id = token_id.unwrap();

            let bidder = info.sender.to_string();
            let ask_key = ask_key(&nft_address, &token_id);

            //Ask validation
            let existing_ask = asks().may_load(deps.storage, ask_key.clone())?;
            match existing_ask.clone() {
                Some(ask) =>{
                    if ask.is_expired(&env.block) {
                        return Err(ContractError::AskExpired {  })
                    }
                },
                None =>{
                    return Err(ContractError::NoSuchAsk {  })
                }
            }

            let existing_bids_token = query_bids(deps.as_ref(), nft_address.clone(), token_id.clone(), None, Some(MAX_QUERY_LIMIT))?;
  
            let existing_ask = existing_ask.unwrap();

            asks().remove(deps.storage, ask_key.clone())?;
            
            //token amount validation for the fixed price sale
            if list_price.denom.clone() != existing_ask.list_price.denom.clone(){
                return Err(ContractError::NotEnoughFunds {  })
            }
            if list_price.amount != existing_ask.list_price.amount{
                return Err(ContractError::NotEnoughFunds {  })
            }

            //bid information for this token_id;
            for bid in existing_bids_token.bids{
                match bid.token_address{
                    Some(token_address) =>{
                        messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
                            contract_addr: token_address,
                            msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                                recipient: bid.bidder.clone(),
                                amount: bid.list_price.amount })?,
                            funds: vec![] }));
                    }   
                    None =>{
                        messages.push(CosmosMsg::Bank(BankMsg::Send {
                            to_address: bid.bidder.clone(),
                            amount: vec![Coin{denom: bid.list_price.denom.clone(), amount: bid.list_price.amount}] }));               
                    }

                }
                bids().remove(deps.storage, (nft_address.clone(), token_id.clone(), bid.bidder))?;                
            }

            update_sale_history_tvl(
                deps.storage,
                 env, 
                 info, 
                 existing_ask.seller.clone(), 
                 bidder.clone(), 
                 nft_address.clone(), 
                 token_id.clone(), 
                 existing_ask.list_price.clone()
            )?;
            
            distribute_money(
                deps.as_ref(),
                nft_address.clone(),
                collection_info.royalty_portion,
                existing_ask.seller.clone(),
                bidder.clone(),
                 existing_ask.list_price,
                None,
                token_id.clone(),
                & mut messages
            )?;           
            
            Ok(Response::new()
                .add_attribute("action", "buy Nft as fixed price with coin")
                .add_attribute("bidder", bidder)
                .add_messages(messages))
        }
    }

}

fn execute_withdraw(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    nft_address: String,
    token_id:String
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let sender = info.sender.to_string();
    let mut messages :Vec<CosmosMsg> = Vec::new();

    let ask_key = ask_key(&nft_address, &token_id);
    let ask = asks().load(deps.storage, ask_key.clone())?;
   
    if ask.seller != sender{
        return Err(ContractError::Unauthorized {  })
    }

    asks().remove(deps.storage, ask_key)?;

    
    //bid information for this token_id;
    let existing_bids_token = query_bids(deps.as_ref(), nft_address.clone(), token_id.clone(), None, Some(MAX_QUERY_LIMIT))?;
    //remove bids for this token_id
    for bid in existing_bids_token.bids{
        match bid.token_address{
            Some(token_address) =>{
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
                    contract_addr: token_address.clone(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                        recipient: bid.bidder.clone(),
                        amount: bid.list_price.amount })?,
                    funds: vec![] }));
            }   
            None =>{
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: bid.bidder.clone(),
                    amount: vec![Coin{denom: bid.list_price.denom, amount: bid.list_price.amount}] }));               
            }
        }
        bids().remove(deps.storage, (nft_address.clone(), token_id.clone(), bid.bidder))?;                
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
         contract_addr: nft_address.clone(), 
         msg: to_binary(&Cw721ExecuteMsg::TransferNft {
             recipient: ask.seller, 
             token_id: token_id.clone() })?, 
         funds: vec![] }));

    Ok(Response::new()
        .add_attribute("action", "cancel the ask")
        .add_attribute("contract_address", nft_address)
        .add_attribute("token_id", token_id)
        .add_messages(messages))
}


fn execute_update_ask_price(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    nft_address: String,
    token_id:String,
    list_price: Asset,
    token_address: Option<String>
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let sender = info.sender.to_string();
    
    let ask_key = ask_key(&nft_address, &token_id);
    let mut ask = asks().load(deps.storage, ask_key.clone())?;
   
    if ask.seller != sender{
        return Err(ContractError::Unauthorized {  })
    }

    let denom = list_price.denom.clone();
    match token_address{
      Some(token_address) => {
        match TOKENADDRESS.may_load(deps.storage, &token_address)? {
            Some(token_denom) =>{
                if token_denom != denom {
                    return Err(ContractError::WrongTokenContractError {  })
                }
                else{
                    ask.list_price = list_price.clone();
                    asks().save(deps.storage, ask_key, &ask)?;
                }
            }
            None =>{
                return Err(ContractError::WrongTokenContractError {  })
            }
        }
      }
      None => {
         match COINDENOM.may_load(deps.storage, &denom)? {
            Some(_result) => {
                ask.list_price = list_price.clone();
                asks().save(deps.storage, ask_key, &ask)?;
            },
            None => {
                return Err(ContractError::WrongCoinDenom {  })
            }
        }
        }
    }
    
    Ok(Response::new()
        .add_attribute("action", "Update Price")
        .add_attribute("contract_address", nft_address)
        .add_attribute("token_id", token_id)
        .add_attribute("denom", denom)
        .add_attribute("amount", list_price.amount.to_string())
      )
}


/// Removes a bid made by the bidder. Bidders can only remove their own bids
pub fn execute_remove_bid(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    nft_address: String,
    token_id: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let bidder = info.sender.to_string();
    let mut messages : Vec<CosmosMsg> = Vec::new();

    let key = bid_key(&nft_address, &token_id, &bidder);
    let bid = bids().load(deps.storage, key.clone())?;
    bids().remove(deps.storage, key)?;
    
    if bid.token_address.is_none(){
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: bidder.clone(),
            amount: vec![Coin{ denom:bid.list_price.denom, amount:bid.list_price.amount}] 
        }))
    }
    else{
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: bid.token_address.unwrap(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: bidder.clone(),
                amount: bid.list_price.amount })?,
            funds: vec![] }))
    }

    Ok(Response::new()
        .add_attribute("action", "remove bid")
        .add_attribute("nft_address", nft_address)
        .add_attribute("token_id", token_id)
        .add_attribute("bidder", bidder)
        .add_messages(messages))
}

pub fn execute_accept_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    nft_address: String,
    token_id: String,
    bidder: String
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut messages : Vec<CosmosMsg> = Vec::new();

    //collection validation check
    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info.is_none(){
        return Err(ContractError::WrongNFTContractError {  })
    }
    let collection_info = collection_info.unwrap();


    let sender = info.sender.to_string();
    let bid_key = bid_key(&nft_address, &token_id, &bidder);
    let ask_key = ask_key(&nft_address, &token_id);

    //Bid validation and ask auth check
    let crr_bid = bids().load(deps.storage, bid_key.clone())?;
    if crr_bid.is_expired(&env.block) {
        return Err(ContractError::BidExpired {});
    }

    let existing_ask = asks().load(deps.storage, ask_key.clone())?;
    asks().remove(deps.storage, ask_key.clone())?;

    if existing_ask.seller != sender{
        return Err(ContractError::Unauthorized {  })
    }

    let existing_bids_token = query_bids(deps.as_ref(), nft_address.clone(), token_id.clone(), None, Some(MAX_QUERY_LIMIT))?;
    
      //remove bids for this token_id
    for bid in existing_bids_token.bids{
       if bid.bidder != crr_bid.bidder{
            match bid.token_address{
                Some(token_address) =>{
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
                        contract_addr: token_address.clone(),
                        msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                            recipient: bid.bidder.clone(),
                            amount: bid.list_price.amount })?,
                        funds: vec![] }));
                }   
                None =>{
                    messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: bid.bidder.clone(),
                        amount: vec![Coin{denom: bid.list_price.denom, amount: bid.list_price.amount}] }));               
                }
                }
            bids().remove(deps.storage, (nft_address.clone(), token_id.clone(), bid.bidder))?;
        }
        else{
            bids().remove(deps.storage, (nft_address.clone(), token_id.clone(), bid.bidder))?;
        }                
    }

    update_sale_history_tvl(
        deps.storage, 
        env, 
        info, 
        existing_ask.seller.clone(), 
        bidder.clone(), 
        nft_address.clone(), 
        token_id.clone(), 
        crr_bid.list_price.clone()
    )?;

    match crr_bid.token_address {
        Some(token_address) =>{
            distribute_money(
                deps.as_ref(), 
                nft_address,
                collection_info.royalty_portion, 
                existing_ask.seller.clone(), 
                bidder.clone(), 
                crr_bid.list_price.clone(), 
                Some(token_address), 
                token_id, 
                &mut messages
            )?;        
        },
        None =>{
             distribute_money(
                deps.as_ref(), 
                nft_address,
                collection_info.royalty_portion, 
                existing_ask.seller.clone(), 
                bidder.clone(), 
                crr_bid.list_price.clone(), 
                None, 
                token_id, 
                &mut messages
            )?;   
        }
    }


    Ok(Response::new()
        .add_attribute("action", "accept bid")
        .add_attribute("seller", existing_ask.seller)
        .add_attribute("bidder", bidder)
        .add_attribute("denom", crr_bid.list_price.denom)
        .add_attribute("amount", crr_bid.list_price.amount.to_string())
        .add_messages(messages))
}

/// Remove an existing collection bid (limit order)
pub fn execute_remove_collection_bid(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    nft_address: String,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let mut messages : Vec<CosmosMsg> = Vec::new();
    let bidder = info.sender.to_string();

    let key = collection_bid_key(&nft_address, &bidder);

    let collection_bid = collection_bids().load(deps.storage, key.clone())?;
    collection_bids().remove(deps.storage, key)?;
    
    if collection_bid.token_address.is_none(){
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: bidder.clone(),
            amount: vec![Coin{ denom:collection_bid.list_price.denom, amount:collection_bid.list_price.amount}] 
        }))
    }
    else{
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: collection_bid.token_address.unwrap(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: bidder.clone(),
                amount: collection_bid.list_price.amount })?,
            funds: vec![] }))
    }
   
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "remove collection bidder")
        .add_attribute("collection", nft_address)
        .add_attribute("bidder", bidder))
}
/// Owner/seller of an item in a collection can accept a collection bid which transfers funds as well as a token
pub fn execute_accept_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    nft_address: String,
    token_id: String,
    bidder: String,
) -> Result<Response, ContractError> {

    let mut messages : Vec<CosmosMsg> = Vec::new();

    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info.is_none(){
        return Err(ContractError::WrongNFTContractError {  })
    }
    let collection_info = collection_info.unwrap();

    nonpayable(&info)?;
    let sender = info.sender.to_string();

    let bid_key = collection_bid_key(&nft_address, &bidder);
    let ask_key = ask_key(&nft_address, &token_id);

    let bid = collection_bids().load(deps.storage, bid_key.clone())?;
    if bid.is_expired(&env.block) {
        return Err(ContractError::BidExpired {});
    }
    collection_bids().remove(deps.storage, bid_key)?;

    let existing_ask = asks().may_load(deps.storage, ask_key.clone())?;
    
    match existing_ask.clone(){
        Some(existing_ask) =>{
            asks().remove(deps.storage, ask_key)?;
            if existing_ask.seller != sender{
                return Err(ContractError::Unauthorized {  });
            }
            if existing_ask.is_expired(&env.block){
              return Err(ContractError::AskExpired {  })
            }
             //bid information for this token_id;
            let existing_bids_token = query_bids(deps.as_ref(), nft_address.clone(), token_id.clone(), None, Some(MAX_QUERY_LIMIT))?;
            //remove bids for this token_id
            for each_bid in existing_bids_token.bids{
                match each_bid.token_address{
                    Some(token_address) =>{
                        messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
                            contract_addr: token_address.clone(),
                            msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                                recipient: each_bid.bidder.clone(),
                                amount: each_bid.list_price.amount })?,
                            funds: vec![] }));
                    }   
                    None =>{
                        messages.push(CosmosMsg::Bank(BankMsg::Send {
                            to_address: each_bid.bidder.clone(),
                            amount: vec![Coin{denom: each_bid.list_price.denom, amount: each_bid.list_price.amount}] }));               
                    }
                }
                bids().remove(deps.storage, (nft_address.clone(), token_id.clone(), each_bid.bidder))?;                
            }
            
            update_sale_history_tvl(
                deps.storage, 
                env, 
                info, 
                existing_ask.seller.clone(), 
                bidder.clone(), 
                nft_address.clone(), 
                token_id.clone(), 
                bid.list_price.clone()
            )?;

            match bid.token_address {
                Some(token_address) =>{
                    distribute_money(
                        deps.as_ref(), 
                        nft_address,
                        collection_info.royalty_portion, 
                        existing_ask.seller.clone(), 
                        bidder.clone(), 
                        bid.list_price.clone(), 
                        Some(token_address), 
                        token_id, 
                        &mut messages
                    )?;        
                },
                None =>{
                    distribute_money(
                        deps.as_ref(), 
                        nft_address,
                        collection_info.royalty_portion, 
                        existing_ask.seller.clone(), 
                        bidder.clone(), 
                        bid.list_price.clone(), 
                        None, 
                        token_id, 
                        &mut messages
                    )?;   
                }
            }


        },
        None =>{
            update_sale_history_tvl(
                deps.storage, 
                env, 
                info, 
                sender.clone(), 
                bidder.clone(), 
                nft_address.clone(), 
                token_id.clone(), 
                bid.list_price.clone()
            )?;

            match bid.token_address {
                Some(token_address) =>{
                    distribute_money(
                        deps.as_ref(), 
                        nft_address,
                        collection_info.royalty_portion, 
                        sender.clone(), 
                        bidder.clone(), 
                        bid.list_price.clone(), 
                        Some(token_address), 
                        token_id, 
                        &mut messages
                    )?;        
                },
                None =>{
                    distribute_money(
                        deps.as_ref(), 
                        nft_address,
                        collection_info.royalty_portion, 
                        sender.clone(), 
                        bidder.clone(), 
                        bid.list_price.clone(), 
                        None, 
                        token_id, 
                        &mut messages
                    )?;   
                }
            }
        }
    }

    Ok(Response::new()
        .add_attribute("action", "accept collection bid")
        .add_attribute("bidder", bidder)
        .add_attribute("denom", bid.list_price.denom)
        .add_attribute("amount", bid.list_price.amount.to_string())
        .add_messages(messages))
}



fn execute_add_collection(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    royalty_potion: Decimal,
    members: Vec<UserInfo>,
    nft_address:String,
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    //auth validate
    deps.api.addr_validate(&nft_address)?;
    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    
    let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
    if collection_info != None{
        return Err(ContractError::WrongNFTContractError {  })
    } 
    
    let mut sum_portion = Decimal::zero();
    for item in members.clone() {
        sum_portion = sum_portion + item.portion;
        deps.api.addr_validate(&item.address)?;
    }

    if sum_portion != Decimal::one(){
        return Err(ContractError::WrongPortionError { })
    }

    MEMBERS.save(deps.storage,&nft_address, &members)?;
    COLLECTIONINFO.save(deps.storage,&nft_address,&CollectionInfo{
        nft_address:nft_address.clone(),
        royalty_portion:royalty_potion
    })?;
    Ok(Response::default())
}


fn execute_update_collection(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    royalty_potion: Decimal,
    members: Vec<UserInfo>,
    nft_address:String
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    deps.api.addr_validate(&nft_address)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }

    let collection_info = COLLECTIONINFO.may_load(deps.storage,&nft_address)?;
    if collection_info == None{
        return Err(ContractError::WrongCollection {  })
    }
    
    let mut sum_portion = Decimal::zero();

    for item in members.clone() {
        sum_portion = sum_portion + item.portion;
        deps.api.addr_validate(&item.address)?;
    }

    if sum_portion != Decimal::one(){
        return Err(ContractError::WrongPortionError { })
    }

    MEMBERS.save(deps.storage,&nft_address, &members)?;
    COLLECTIONINFO.save(deps.storage,&nft_address,&CollectionInfo{
        nft_address:nft_address.clone(),
        royalty_portion:royalty_potion,
    })?;
    Ok(Response::default())
}


fn execute_token_address(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    symbol:String,
    address: String,
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    
    TOKENADDRESS.save(deps.storage,&address,&symbol)?;

    Ok(Response::default())
}


fn execute_add_coin(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    symbol:String,
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
    }
    
    COINDENOM.save(deps.storage, &symbol, &true)?;
    
    Ok(Response::default())
}

fn execute_fix_nft(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
    token_id:String
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: address,
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: info.sender.to_string(),
                    token_id: token_id.clone(),
            })?,
        })))
}


fn execute_migrate(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
    dest:String,
    token_ids:Vec<String>
) -> Result<Response, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    deps.api.addr_validate(&address)?;
     deps.api.addr_validate(&dest)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    let mut messages:Vec<CosmosMsg> = vec![];

    for token_id in token_ids{
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: address.clone(),
                funds: vec![],
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: dest.clone(),
                    token_id: token_id.clone(),
            })?,
        }))
    }

    Ok(Response::new()
        .add_messages(messages))
}


fn execute_change_owner(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;

    if state.admin != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }
    deps.api.addr_validate(&address)?;
    state.owner = address;
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}


fn execute_change_admin(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let mut state = CONFIG.load(deps.storage)?;

    if state.admin != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }
    deps.api.addr_validate(&address)?;
    state.admin = address;
    CONFIG.save(deps.storage,&state)?;
    Ok(Response::default())
}

fn execute_set_tvl(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    address: String,
    tvls: Vec<TvlInfo>,
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

   
    for collection_tvl in tvls{
         let tvl_key = tvl_key(&address, &collection_tvl.denom);
         tvl().save(deps.storage,tvl_key.clone(), &collection_tvl)?;
    }

    Ok(Response::default())
}

fn execute_set_offerings(
    deps: DepsMut,
    env:Env,
    info:MessageInfo,
    nft_address: String,
    offerings:Vec<QueryOfferingsResult>
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }
    for offering in offerings{
    
        let new_ask = Ask{
            token_id: offering.token_id.clone(),
            seller: offering.seller,
            list_price: offering.list_price,
            expires_at: Timestamp::from_seconds(env.block.time.seconds() + 259200),
            collection: nft_address.clone()
        };

        let ask_key = ask_key(&nft_address, &offering.token_id);
        asks().save(deps.storage, ask_key, &new_ask)?;
    }
    
    Ok(Response::default())
}


fn execute_history(
    deps: DepsMut,
    _env:Env,
    info:MessageInfo,
    address: String,
    histories:Vec<SaleInfo>
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    // let mut count = 0;
    
    for history in histories{
        let sale_history_key = sale_history_key(&address, &history.token_id, history.time);
        sale_history().save(deps.storage, sale_history_key, &history)?;
    }
   
    Ok(Response::default())
}


fn execute_bid_limit(
    deps: DepsMut,
    _env:Env,
    info:MessageInfo,
    bid_limit: u32
) -> Result<Response, ContractError> {

    //Validation Check
    let  state = CONFIG.load(deps.storage)?;
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    CONFIG.update(deps.storage, |mut state| -> StdResult<_>{
        state.bid_limit = bid_limit;
        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("action", "set bid limit")
        .add_attribute("bid_limit", bid_limit.to_string()))
}


fn execute_withdraw_coin(
    deps: DepsMut,
    _env:Env,
    info:MessageInfo,
    token_amount: Uint128,
    coin_amount: Uint128,
    token_address: String,
    coin_denom: String
) -> Result<Response, ContractError> {

    //Validation Check
    let  state = CONFIG.load(deps.storage)?;
    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }

    let mut messages: Vec<CosmosMsg> = Vec::new();
    if token_amount > Uint128::new(0){
      messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
        contract_addr: token_address, 
        msg: to_binary(&Cw20ExecuteMsg::Transfer { 
          recipient: state.owner.clone(), 
          amount: token_amount 
        })?, 
        funds: vec![] 
      }))
    }

    if coin_amount > Uint128::zero(){
      messages.push(CosmosMsg::Bank(BankMsg::Send { 
        to_address: state.owner, 
        amount: vec![Coin{
          denom: coin_denom,
          amount: coin_amount
        }] 
      }))
    }

    Ok(Response::new()
        .add_messages(messages)
      )
}


fn store_ask(store: &mut dyn Storage, ask: &Ask) -> StdResult<()> {
    asks().save(store, ask_key(&ask.collection, &ask.token_id), ask)
}


fn store_bid(store: &mut dyn Storage, bid: &Bid) -> StdResult<()> {
    bids().save(
        store,
        bid_key(&bid.collection, &bid.token_id, &bid.bidder),
        bid,
    )
}

fn update_sale_history_tvl(
     store:& mut dyn Storage,
     env:Env,
     _info: MessageInfo,
     seller: String,
     bidder:String,
     nft_address: String,
     token_id: String,
     list_price: Asset
) -> StdResult<()> {
     //Add sale info created by fixed price sale
        let new_sale_info = SaleInfo { 
            from: seller,
            to: bidder.clone(), 
            denom: list_price.denom.clone(),
            amount: list_price.amount, 
            time: env.block.time.seconds(),
            collection:nft_address.clone(),
            token_id:token_id.clone()
        };
        let crr_time = env.block.time.seconds();
        let sale_history_key = sale_history_key(&nft_address, &token_id, crr_time);
        sale_history().save(store, sale_history_key, &new_sale_info)?;

        //update the TVL
        let denom = list_price.denom;
        let tvl_key = tvl_key(&nft_address.clone(), &denom);
        let  crr_tvl = tvl().may_load(store, tvl_key.clone())?;
        match crr_tvl{
            Some(mut crr_tvl) => {
                crr_tvl.amount = crr_tvl.amount + list_price.amount;
                tvl().save(store, tvl_key, &crr_tvl)?; 
            }
            None =>{
                tvl().save(store, tvl_key, &TvlInfo { 
                    denom: denom,
                    amount: list_price.amount,
                    collection: nft_address.clone() })?; 
            }
        }

    Ok(())
}

fn distribute_money(
    deps: Deps,
    nft_address: String,
    royalty_portion: Decimal,
    seller: String,
    bidder: String,
    list_price: Asset,
    token_address: Option<String>,
    token_id: String,
    messages:& mut Vec<CosmosMsg>
) -> StdResult<()>{
        let members = MEMBERS.load(deps.storage,&nft_address)?;
        let amount = list_price.amount;
        //Distribute money to the admins
        if token_address.is_none(){
            for user in members{
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: user.address,
                        amount:vec![Coin{
                            denom:list_price.denom.clone(),
                            amount:amount*royalty_portion*user.portion
                        }]
                }))
            }
            
                //Send money to asker
            messages.push(
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: seller,
                    amount:vec![Coin{
                        denom:list_price.denom,
                        amount:amount*(Decimal::one()-royalty_portion)
                    }]
                })
            );
       }else{
          let token_address = token_address.unwrap();
           for user in members{
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
                  contract_addr: token_address.clone(), 
                  msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: user.address, 
                    amount: amount*royalty_portion*user.portion 
                  })?, 
                  funds: vec![] }))
            }
            
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute { 
                  contract_addr: token_address, 
                  msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: seller, 
                    amount: amount*(Decimal::one()-royalty_portion)
                  })?, 
                  funds: vec![] }));
                //Send money to asker
       }
            
        //Transfer NFT to bidder
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: nft_address,
                msg: to_binary(&Cw721ExecuteMsg::TransferNft { 
                recipient: bidder.clone(),
                token_id: token_id })?,
                funds: vec![] }));

        Ok(())
}


/// returns an error if any coins were sent
pub fn nonpayable(info: &MessageInfo) -> Result<(), ContractError> {
    if info.funds.is_empty() {
        Ok(())
    } else {
        Err(ContractError::TooMuchFunds {  })
    }
}



