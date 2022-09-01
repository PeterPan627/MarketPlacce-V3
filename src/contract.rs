use std::rc;

use cosmwasm_std::{
    entry_point, to_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,from_binary,Binary,
    StdResult, Uint128,CosmosMsg,WasmMsg,Decimal,BankMsg,Storage, Timestamp
};

use cw2::set_contract_version;
use cw20::{ Cw20ExecuteMsg,Cw20ReceiveMsg};
use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg};

use crate::error::{ContractError};
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg,SellNft, BuyNft};
use crate::query::query_bids;
use crate::state::{State,CONFIG,Asset,UserInfo, MEMBERS,SALEHISTORY,SaleInfo, COLLECTIONINFO, CollectionInfo, TOKENADDRESS, TVL, TvlInfo, COINDENOM, SaleType, };
use crate::state::{Ask,asks,AskKey,ask_key,Order,Bid, bids, BidKey, bid_key};
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
        bid_limit: 10
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
    ExecuteMsg::Receive(msg) =>execute_receive(deps,env,info,msg),
    ExecuteMsg::SetBidCoin { nft_address, expire, sale_type, token_id, list_price } => execute_bid_with_coin(deps, env, info, nft_address, token_id, expire, sale_type, list_price),
    ExecuteMsg::WithdrawNft { offering_id,nft_address } => execute_withdraw(deps,env,info,offering_id,nft_address),
    ExecuteMsg::AddTokenAddress { symbol, address }  => execute_token_address(deps,env,info,symbol,address),
    ExecuteMsg::ChangeOwner { address } =>execute_change_owner(deps,env,info,address),
    ExecuteMsg::AddCollection { royalty_portion, members,nft_address ,offering_id,sale_id} =>execute_add_collection(deps,env,info,royalty_portion,members,nft_address,offering_id,sale_id),
    ExecuteMsg::UpdateCollection { royalty_portion, members,nft_address } =>execute_update_collection(deps,env,info,royalty_portion,members,nft_address),
    ExecuteMsg::FixNft{address,token_id} =>execute_fix_nft(deps,env,info,address,token_id),
    ExecuteMsg::SetOfferings { address, offering }=>execute_set_offerings(deps,env,info,address,offering),
    ExecuteMsg::SetTvl { address, tvl } =>execute_set_tvl(deps,env,info,address,tvl),
    ExecuteMsg::Migrate { address, dest, token_id }=>execute_migrate(deps,env,info,address,dest,token_id),
    ExecuteMsg::SetSaleHistory { address, history }=>execute_history(deps,env,info,address,history),
    ExecuteMsg::SetBidLimit { bid_limit } => execute_bid_limit(deps,env,info,bid_limit)
}
}


fn execute_receive_nft(
    deps: DepsMut,
    _env:Env,
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
    if msg.is_coin{
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
        //Validate Configuration, token_address can not be existed if the list price is set as coin
        if !token_address.is_none(){
            return Err(ContractError::WrongConfig {  })
        }
    } else{
        //Validate Configuration, token_address can not be existed if the list price is set as coin
        if token_address.clone().is_none(){
            return Err(ContractError::WrongConfig {  })
        }

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

    //Get ask and bid keys
    let bidder = rcv_msg.sender;
    let bid_key = bid_key(&nft_address, &token_id, &bidder);
    let ask_key = ask_key(&nft_address, &token_id);

    //Ask validation
    let existing_ask = asks().may_load(deps.storage, ask_key)?;
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
     
    //Bid or Buy with fixed_price
    match msg.sale_type{
        SaleType::Auction => {
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
        SaleType::FixedPrice =>{
            let mut messages: Vec<CosmosMsg> = Vec::new();
            let existing_ask = existing_ask.unwrap();
            
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
                            contract_addr: token_address,
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

            let members = MEMBERS.load(deps.storage,&nft_address)?;
            //Distribute money to the admins
            for user in members{
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: info.sender.to_string(),
                        funds: vec![],
                        msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                            recipient: user.address.clone(), 
                            amount: rcv_msg.amount*collection_info.royalty_portion*user.portion })?,
                }))
            }
            
            //Send money to asker
            messages.push(
                CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_address,
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                    recipient: existing_ask.seller, 
                    amount: rcv_msg.amount*(Decimal::one()-collection_info.royalty_portion) })?,
                })
            );
            
            //Transfer NFT to bidder
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                 contract_addr: nft_address,
                 msg: to_binary(&Cw721ExecuteMsg::TransferNft { 
                    recipient: bidder.clone(),
                    token_id: token_id })?,
                 funds: vec![] }));
            Ok(Response::new()
                .add_attribute("action", "buy Nft as fixed price with token")
                .add_attribute("bidder", bidder))
        }
    }

    
//     let off = OFFERINGS.load(deps.storage, (&msg.nft_address,&msg.offering_id))?;

    
//     if off.list_price.denom != token_symbol{
//         return Err(ContractError::NotEnoughFunds  { })
//     }

//     if off.list_price.amount != rcv_msg.amount{
//         return Err(ContractError::NotEnoughFunds  { })
//     }

//     let tvl = TVL.may_load(deps.storage, (&msg.nft_address,&off.list_price.denom))?;
//     let crr_tvl:Uint128;
//     if tvl == None {
//         crr_tvl = off.list_price.amount;
//     }
//     else {
//         crr_tvl = tvl.unwrap()+off.list_price.amount;
//     }

//     TVL.save(deps.storage,( &msg.nft_address,&off.list_price.denom), &crr_tvl)?;
  
//     let members = MEMBERS.load(deps.storage,&msg.nft_address)?;
//     let collection_info = COLLECTIONINFO.may_load(deps.storage, &msg.nft_address)?;

//     if collection_info == None{
//         return Err(ContractError::WrongNFTContractError {  })
//     }

//     let collection_info = collection_info.unwrap();

//     if collection_info.offering_id == 1{    
//           OFFERINGS.remove( deps.storage, (&msg.nft_address,&msg.offering_id));
//           COLLECTIONINFO.update(deps.storage, &msg.nft_address,
//              |collection_info|->StdResult<_>{
//                     let mut collection_info = collection_info.unwrap();
//                     collection_info.offering_id = 0;
//                     Ok(collection_info)
//              })?;
//     }

//     else{
//         let crr_offering_id = collection_info.offering_id;
//         let offering = OFFERINGS.may_load(deps.storage, (&msg.nft_address,&crr_offering_id.to_string()))?;
        
//         if offering!=None{
//             OFFERINGS.save(deps.storage, (&msg.nft_address,&msg.offering_id.to_string()), &offering.unwrap())?;
           
//             COLLECTIONINFO.update(deps.storage, &msg.nft_address,
//                 |collection_info|->StdResult<_>{
//                         let mut collection_info = collection_info.unwrap();
//                         collection_info.offering_id =collection_info.offering_id-1;
//                         Ok(collection_info)
//                 })?;
//           OFFERINGS.remove( deps.storage, (&msg.nft_address,&crr_offering_id.to_string()));
//          }

         
//     }

//     let mut messages:Vec<CosmosMsg> = vec![];
//     for user in members{
//         messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: info.sender.to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw20ExecuteMsg::Transfer { 
//                     recipient: user.address.clone(), 
//                     amount: rcv_msg.amount*collection_info.royalty_portion*user.portion })?,
//         }))
//     }

//     let sale_id = collection_info.sale_id+1;

//     SALEHISTORY.save(deps.storage, (&msg.nft_address,&sale_id.to_string()),&SaleInfo { 
//         from:off.seller.clone(),
//         to: rcv_msg.sender.to_string(), 
//         denom: off.list_price.denom,
//         amount: rcv_msg.amount, 
//         time: env.block.time.seconds(),
//         nft_address:msg.nft_address.clone(),
//         token_id:off.token_id.clone()
//     })?;

//     COLLECTIONINFO.update(deps.storage, &msg.nft_address, 
//         |collection_info|->StdResult<_>{
//             let mut collection_info = collection_info.unwrap();
//             collection_info.sale_id = sale_id;
//             Ok(collection_info)
//         }
//     )?;
    

//     Ok(Response::new()
//         .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: msg.nft_address.to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw721ExecuteMsg::TransferNft {
//                     recipient: deps.api.addr_validate(&rcv_msg.sender)?.to_string(),
//                     token_id: off.token_id.clone(),
//             })?,
//         }))
//         .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: info.sender.to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw20ExecuteMsg::Transfer { 
//                     recipient: off.seller, 
//                     amount: rcv_msg.amount*(Decimal::one()-collection_info.royalty_portion) })?,
//         }))
//         .add_messages(messages)
// )
}

fn execute_bid_with_coin(
    deps: DepsMut,
    env:Env,
    info: MessageInfo,
    nft_address : String,
    token_id: String,
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

    //Get ask and bid keys
    let bidder = info.sender.to_string();
    let bid_key = bid_key(&nft_address, &token_id, &bidder);
    let ask_key = ask_key(&nft_address, &token_id);

    //Ask validation
    let existing_ask = asks().may_load(deps.storage, ask_key)?;
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
    
     //Bid or Buy with fixed_price
    match sale_type{
        SaleType::Auction => {
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
        SaleType::FixedPrice =>{
            let mut messages: Vec<CosmosMsg> = Vec::new();
            let existing_ask = existing_ask.unwrap();
            
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

            let members = MEMBERS.load(deps.storage,&nft_address)?;
            //Distribute money to the admins
            for user in members{
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                        to_address: user.address,
                        amount:vec![Coin{
                            denom:list_price.denom.clone(),
                            amount:amount*collection_info.royalty_portion*user.portion
                        }]
                }))
            }
            
            //Send money to asker
            messages.push(
               CosmosMsg::Bank(BankMsg::Send {
                    to_address: existing_ask.seller,
                    amount:vec![Coin{
                        denom:list_price.denom,
                        amount:amount*(Decimal::one()-collection_info.royalty_portion)
                    }]
                })
            );
            
            //Transfer NFT to bidder
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                 contract_addr: nft_address,
                 msg: to_binary(&Cw721ExecuteMsg::TransferNft { 
                    recipient: bidder.clone(),
                    token_id: token_id })?,
                 funds: vec![] }));
            
            Ok(Response::new()
                .add_attribute("action", "buy Nft as fixed price with coin")
                .add_attribute("bidder", bidder))
        }
    }

//     let off = OFFERINGS.load(deps.storage, (&nft_address, &offering_id))?;

//     let amount= info
//         .funds
//         .iter()
//         .find(|c| c.denom == off.list_price.denom)
//         .map(|c| Uint128::from(c.amount))
//         .unwrap_or_else(Uint128::zero);

//     if off.list_price.amount!=amount{
//         return Err(ContractError::NotEnoughFunds {  })
//     }
//     if collection_info.offering_id == 1{    
//           OFFERINGS.remove( deps.storage, (&nft_address,&offering_id));
//           COLLECTIONINFO.update(deps.storage, &nft_address,
//              |collection_info|->StdResult<_>{
//                     let mut collection_info = collection_info.unwrap();
//                     collection_info.offering_id = 0;
//                     Ok(collection_info)
//              })?;
//     }

//     else{
//         let crr_offering_id = collection_info.offering_id;
//         let offering = OFFERINGS.may_load(deps.storage, (&nft_address,&crr_offering_id.to_string()))?;
//         if offering!=None{
//             OFFERINGS.save(deps.storage, (&nft_address,&offering_id.to_string()), &offering.unwrap())?;
           
//             COLLECTIONINFO.update(deps.storage, &nft_address,
//                 |collection_info|->StdResult<_>{
//                         let mut collection_info = collection_info.unwrap();
//                         collection_info.offering_id =collection_info.offering_id-1;
//                         Ok(collection_info)
//                 })?;
//         OFFERINGS.remove( deps.storage, (&nft_address,&crr_offering_id.to_string()));
  
//          }
//     }
    
//     let members = MEMBERS.load(deps.storage,&nft_address)?;
    
//     let mut messages:Vec<CosmosMsg> = vec![];
//     for user in members{
//         messages.push(CosmosMsg::Bank(BankMsg::Send {
//                 to_address: user.address,
//                 amount:vec![Coin{
//                     denom:off.clone().list_price.denom,
//                     amount:amount*collection_info.royalty_portion*user.portion
//                 }]
//         }))
//     }
    
//      let sale_id = collection_info.sale_id+1;

//     SALEHISTORY.save(deps.storage, (&nft_address,&sale_id.to_string()),&SaleInfo {
//          from:off.seller.clone(), 
//          to: info.sender.to_string(), 
//          denom: off.list_price.denom.clone(),
//          amount: amount, 
//          time: env.block.time.seconds(),
//          nft_address:nft_address.clone(),
//          token_id:off.token_id.clone()         
//         })?;

//     COLLECTIONINFO.update(deps.storage, &nft_address, 
//         |collection_info|->StdResult<_>{
//             let mut collection_info = collection_info.unwrap();
//             collection_info.sale_id = sale_id;
//             Ok(collection_info)
//         }
//     )?;

    
//     let tvl = TVL.may_load(deps.storage, (&nft_address,&off.list_price.denom))?;
//     let crr_tvl:Uint128;
//     if tvl == None {
//         crr_tvl = off.list_price.amount;
//     }
//     else {
//         crr_tvl = tvl.unwrap()+off.list_price.amount;
//     }

//     TVL.save(deps.storage,( &nft_address,&off.list_price.denom), &crr_tvl)?;

  
//     Ok(Response::new()
//         .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: nft_address.to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw721ExecuteMsg::TransferNft {
//                     recipient: info.sender.to_string(),
//                     token_id: off.token_id.clone(),
//             })?,
//         }))
//         .add_message(CosmosMsg::Bank(BankMsg::Send {
//                 to_address: off.seller,
//                 amount:vec![Coin{
//                     denom:off.list_price.denom,
//                     amount:amount*(Decimal::one()-collection_info.royalty_portion)
//                 }]
//         }))
//         .add_messages(messages)
// )
}

fn execute_withdraw(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    offering_id: String,
    nft_address:String
) -> Result<Response, ContractError> {
//     let off = OFFERINGS.load(deps.storage,(&nft_address,&offering_id))?;
//    // let state = CONFIG.load(deps.storage)?;

//     let collection_info = COLLECTIONINFO.may_load(deps.storage, &nft_address)?;
//     if collection_info == None{
//         return Err(ContractError::WrongNFTContractError {  })
//     }
//    let collection_info = collection_info.unwrap();

//    if collection_info.offering_id == 1{    
//           OFFERINGS.remove( deps.storage, (&nft_address,&offering_id));
//           COLLECTIONINFO.update(deps.storage, &nft_address,
//              |collection_info|->StdResult<_>{
//                     let mut collection_info = collection_info.unwrap();
//                     collection_info.offering_id = 0;
//                     Ok(collection_info)
//              })?;
//     }

//     else{
//         let crr_offering_id = collection_info.offering_id;
//         let offering = OFFERINGS.may_load(deps.storage, (&nft_address,&crr_offering_id.to_string()))?;
//         if offering!=None{
//             OFFERINGS.save(deps.storage, (&nft_address,&offering_id.to_string()), &offering.unwrap())?;
           
//             COLLECTIONINFO.update(deps.storage, &nft_address,
//                 |collection_info|->StdResult<_>{
//                         let mut collection_info = collection_info.unwrap();
//                         collection_info.offering_id =collection_info.offering_id-1;
//                         Ok(collection_info)
//                 })?;
//         OFFERINGS.remove( deps.storage, (&nft_address,&crr_offering_id.to_string()));
  
//          }
//     }

//     if off.seller == info.sender.to_string(){

//         Ok(Response::new()
//             .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: nft_address.to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw721ExecuteMsg::TransferNft {
//                     recipient: deps.api.addr_validate(&off.seller)?.to_string(),
//                     token_id: off.token_id.clone(),
//             })?,
//         }))
//     )
//     }
//     else {
//         return Err(ContractError::Unauthorized {});
//     }
    Ok(Response::default())
}


fn execute_add_collection(
    deps: DepsMut,
    _env:Env,
    info: MessageInfo,
    royalty_potion: Decimal,
    members: Vec<UserInfo>,
    nft_address:String,
    offering_id:u64,
    sale_id:u64
)->Result<Response,ContractError>{

    let state = CONFIG.load(deps.storage)?;

    deps.api.addr_validate(&nft_address)?;

    if info.sender.to_string() != state.owner{
        return Err(ContractError::Unauthorized {});
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
        sale_id:sale_id,
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
    let collection_info = collection_info.unwrap();

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
        sale_id:collection_info.sale_id
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

    CONFIG.save(deps.storage, &state)?;
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

    if state.owner != info.sender.to_string() {
        return Err(ContractError::Unauthorized {});
    }
    deps.api.addr_validate(&address)?;
    state.owner = address;
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
   
    for tvl in tvls{
        TVL.save(deps.storage, (&address,&tvl.denom), &tvl.amount)?;
    }

    Ok(Response::default())
}

fn execute_set_offerings(
    deps: DepsMut,
    _env:Env,
    info:MessageInfo,
    address: String,
    offerings:Vec<QueryOfferingsResult>
) -> Result<Response, ContractError> {
    let  state = CONFIG.load(deps.storage)?;

    // if state.owner != info.sender.to_string() {
    //     return Err(ContractError::Unauthorized {});
    // }
    
    // for offering in offerings{
    //     let crr_offering = Offering{
    //         token_id:offering.token_id,
    //         seller:offering.seller,
    //         list_price:offering.list_price
    //     };
    //     OFFERINGS.save(deps.storage, (&address,&offering.id), &crr_offering)?;
    // }
   
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

    let mut count = 0;
    
    for history in histories{
        count =  count+1;
        SALEHISTORY.save(deps.storage, (&address,&count.to_string()), &history)?;
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




// #[cfg(test)]
// mod tests {
  
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{ CosmosMsg, Coin};

//     #[test]
//     fn testing() {
//         //Instantiate
//         let mut deps = mock_dependencies();
//         let instantiate_msg = InstantiateMsg {
//            owner:"creator".to_string()
//         };
//         let info = mock_info("creator", &[]);
//         let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
//         assert_eq!(0, res.messages.len());
//         let state = query_state_info(deps.as_ref()).unwrap();
//         assert_eq!(state.owner,"creator".to_string());
       

//         //Change Owner

//         let info = mock_info("creator", &[]);
//         let msg = ExecuteMsg::ChangeOwner { address:"owner".to_string()};
//         execute(deps.as_mut(), mock_env(), info, msg).unwrap();
//         let state = query_state_info(deps.as_ref()).unwrap();
//         assert_eq!(state.owner,"owner".to_string());

//          //Change Token Contract Address

//         let info = mock_info("owner", &[]);
//         let msg = ExecuteMsg::AddTokenAddress  { address:"token_address".to_string(),symbol:"hope".to_string()};
//         execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        
//         let info = mock_info("owner", &[]);
//          let msg = ExecuteMsg::AddTokenAddress  { address:"raw_address".to_string(),symbol:"raw".to_string()};
//         execute(deps.as_mut(), mock_env(), info, msg).unwrap();
       
//         //Hope1 Collection Add
//        let info = mock_info("owner", &[]);
//        let msg = ExecuteMsg::AddCollection {
//             royalty_portion: Decimal::from_ratio(5 as u128, 100 as u128), 
//             members: vec![UserInfo{
//                 address:"admin1".to_string(),
//                 portion:Decimal::from_ratio(3 as u128, 10 as u128)
//                 },UserInfo{
//                 address:"admin2".to_string(),
//                 portion:Decimal::from_ratio(7 as u128, 10 as u128)
//                 }] ,
//             nft_address: "hope1_address".to_string() ,
//             offering_id:0,
//             sale_id:0
//         };
//         execute(deps.as_mut(), mock_env(), info, msg).unwrap();
       

//        // Sell nft
//         let cw721_msg = SellNft{
//             list_price:Asset{
//                 denom:"ujuno".to_string(),
//                 amount:Uint128::new(1000000)
//             }
//         };

//         let info = mock_info("hope1_address", &[]);
//         let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
//             sender:"owner1".to_string(),
//             token_id:"Hope.1".to_string(),
//             msg:to_binary(&cw721_msg).unwrap()
//         });
//         execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();


//         let collection_info = query_collection_info(deps.as_ref(),
//                 "hope1_address".to_string()).unwrap();
//         assert_eq!(collection_info,CollectionInfo{
//             nft_address:"hope1_address".to_string(),
//             offering_id:1,
//             royalty_portion:Decimal::from_ratio(5 as u128, 100 as u128),
//             sale_id:0
//             });

      
//         let offerings = query_get_offering(deps.as_ref(),vec!["1".to_string(),"2".to_string()],"hope1_address".to_string()).unwrap();
//         assert_eq!(offerings,vec![QueryOfferingsResult{
//             id:"1".to_string(),
//             token_id:"Hope.1".to_string(),
//             list_price:Asset { denom: "ujuno".to_string(), amount: Uint128::new(1000000) },
//             seller:"owner1".to_string()
//         }]);

//             //Buy nft

//       let info = mock_info("test_buyer1", &[Coin{
//         denom:"ujuno".to_string(),
//         amount:Uint128::new(1000000)
//       }]);
//       let msg = ExecuteMsg::BuyNft { offering_id: "1".to_string(), nft_address: "hope1_address".to_string() };
//       let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
//       assert_eq!(res.messages.len(),4);

//        let collection_info = query_collection_info(deps.as_ref(),"hope1_address".to_string()).unwrap();
//        assert_eq!(collection_info.offering_id,0); 

//       assert_eq!(res.messages[0].msg,CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "hope1_address".to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw721ExecuteMsg::TransferNft {
//                     recipient: "test_buyer1".to_string(),
//                     token_id:"Hope.1".to_string(),
//             }).unwrap(),
//         }));

//       assert_eq!(res.messages[1].msg,CosmosMsg::Bank(BankMsg::Send {
//                 to_address: "owner1".to_string(),
//                 amount:vec![Coin{
//                     denom:"ujuno".to_string(),
//                     amount:Uint128::new(950000)
//                 }]
//         }));

//         assert_eq!(res.messages[2].msg,CosmosMsg::Bank(BankMsg::Send {
//                 to_address: "admin1".to_string(),
//                 amount:vec![Coin{
//                     denom:"ujuno".to_string(),
//                     amount:Uint128::new(15000)
//                 }]
//         }));
//         assert_eq!(res.messages[3].msg,CosmosMsg::Bank(BankMsg::Send {
//                 to_address: "admin2".to_string(),
//                 amount:vec![Coin{
//                     denom:"ujuno".to_string(),
//                     amount:Uint128::new(35000)
//                 }]
//         }));
                                                                    
//         let ids =  query_get_ids(deps.as_ref(),"hope1_address".to_string()).unwrap();
//         let test_id:Vec<String> = vec![];
//         assert_eq!(ids,test_id);
        

//          // Rearragnge the offering

//          //sell
//         let cw721_msg = SellNft{
//             list_price:Asset{
//                 denom:"osmos".to_string(),
//                 amount:Uint128::new(2000000)
//             }
//         };

//         let info = mock_info("hope1_address", &[]);
//         let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
//             sender:"buyer1".to_string(),
//             token_id:"Hope.1".to_string(),
//             msg:to_binary(&cw721_msg).unwrap()
//         });
//          execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
     
//          //sell
//          let cw721_msg = SellNft{
//             list_price:Asset{
//                 denom:"ujuno".to_string(),
//                 amount:Uint128::new(2000000)
//             }
//         };

//           let info = mock_info("hope1_address", &[]);
//           let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
//             sender:"buyer2".to_string(),
//             token_id:"Hope.2".to_string(),
//             msg:to_binary(&cw721_msg).unwrap()
//             });
//          execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//          //buy

//         let info = mock_info("test_buyer2", &[Coin{
//             denom:"osmos".to_string(),
//             amount:Uint128::new(2000000)
//         }]);
//         let msg = ExecuteMsg::BuyNft { offering_id: "1".to_string(), nft_address: "hope1_address".to_string() };
//         execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let id = query_get_ids(deps.as_ref(), "hope1_address".to_string()).unwrap();
//         let collection_info = query_collection_info(deps.as_ref(),"hope1_address".to_string()).unwrap();
//         assert_eq!(collection_info.offering_id,1);  
//         assert_eq!(id,vec!["1"]);

//          let cw721_msg = SellNft{
//             list_price:Asset{
//                 denom:"hope".to_string(),
//                 amount:Uint128::new(2000000)
//             }
//         };

//         let info = mock_info("hope1_address", &[]);
//         let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
//             sender:"buyer3".to_string(),
//             token_id:"Hope.3".to_string(),
//             msg:to_binary(&cw721_msg).unwrap()
//         });

//         execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let ids = query_get_ids(deps.as_ref(),"hope1_address".to_string()).unwrap();
//         assert_eq!(ids,vec!["1".to_string(),"2".to_string()]);
//         let offerings = query_get_offering(deps.as_ref(),vec!["1".to_string(),"2".to_string()],"hope1_address".to_string()).unwrap();
//         assert_eq!(offerings,vec![QueryOfferingsResult{
//             id:"1".to_string(),
//             token_id:"Hope.2".to_string(),
//             list_price:Asset { denom: "ujuno".to_string(),  amount:Uint128::new(2000000) },
//             seller:"buyer2".to_string()
//         },QueryOfferingsResult{
//             id:"2".to_string(),
//             token_id:"Hope.3".to_string(),
//             list_price:Asset { denom: "hope".to_string(),  amount:Uint128::new(2000000) },
//             seller:"buyer3".to_string()
//         }]);

//         let cw20_msg= BuyNft{
//             offering_id:"2".to_string(),
//             nft_address:"hope1_address".to_string()
//         };

//         let info = mock_info("token_address", &[]);
//         let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
//             sender:"test_buyer3".to_string(),
//             amount:Uint128::new(2000000),
//             msg:to_binary(&cw20_msg).unwrap()
//         });
//         let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();    
//         assert_eq!(res.messages[1].msg,CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "token_address".to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw20ExecuteMsg::Transfer { 
//                     recipient: "buyer3".to_string(), 
//                     amount:Uint128::new(1900000),
//                  }).unwrap()
//          }));

//          assert_eq!(res.messages[2].msg,CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "token_address".to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw20ExecuteMsg::Transfer { 
//                     recipient: "admin1".to_string(), 
//                     amount:Uint128::new(30000),
//                  }).unwrap()
//          }));

//           assert_eq!(res.messages[3].msg,CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: "token_address".to_string(),
//                 funds: vec![],
//                 msg: to_binary(&Cw20ExecuteMsg::Transfer { 
//                     recipient: "admin2".to_string(), 
//                     amount:Uint128::new(70000),
//                  }).unwrap()
//          }));


//         let offerings = query_get_offering(deps.as_ref(),vec!["1".to_string(),"2".to_string()],"hope1_address".to_string()).unwrap();
//         assert_eq!(offerings,vec![QueryOfferingsResult{
//             id:"1".to_string(),
//             token_id:"Hope.2".to_string(),
//             list_price:Asset { denom: "ujuno".to_string(),  amount:Uint128::new(2000000) },
//             seller:"buyer2".to_string()
//         }]);

//         let cw721_msg = SellNft{
//             list_price:Asset{
//                 denom:"raw".to_string(),
//                 amount:Uint128::new(2000000)
//             }
//         };

//         let info = mock_info("hope1_address", &[]);
//         let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
//             sender:"owner1".to_string(),
//             token_id:"Hope.1".to_string(),
//             msg:to_binary(&cw721_msg).unwrap()
//         });
//         execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         let cw20_msg= BuyNft{
//             offering_id:"2".to_string(),
//             nft_address:"hope1_address".to_string()
//         };

//         let info = mock_info("raw_address", &[]);
//         let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
//             sender:"test_buyer4".to_string(),
//             amount:Uint128::new(2000000),
//             msg:to_binary(&cw20_msg).unwrap()
//         });
//         execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//         assert_eq!(offerings,vec![QueryOfferingsResult{
//             id:"1".to_string(),
//             token_id:"Hope.2".to_string(),
//             list_price:Asset { denom: "ujuno".to_string(),  amount:Uint128::new(2000000) },
//             seller:"buyer2".to_string()
//         }]);

//         let juno_tvl = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"ujuno".to_string()).unwrap();
//         let hope_tvl = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"hope".to_string()).unwrap();
//         let osmos_tvl = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"osmos".to_string()).unwrap();
//         let raw_tvl  = query_get_tvl(deps.as_ref(),"hope1_address".to_string(),"raw".to_string()).unwrap();
//         println!("{}","juno".to_string());
//         assert_eq!(juno_tvl,Uint128::new(1000000));
        
//         println!("{}","hope".to_string());
//         assert_eq!(hope_tvl,Uint128::new(2000000));

//         println!("{}","osmos".to_string());
//         assert_eq!(osmos_tvl,Uint128::new(2000000));

//          println!("{}","raw".to_string());
//         assert_eq!(raw_tvl,Uint128::new(2000000));

//         let collection_info = query_collection_info(deps.as_ref(), "hope1_address".to_string()).unwrap();
//         assert_eq!(collection_info.sale_id,4);
//         let _sale_history = query_get_history(deps.as_ref(), "hope1_address".to_string(), vec!["1".to_string(),"2".to_string(),"3".to_string(),"4".to_string()]).unwrap();
        
//         let tvl_all = query_all_tvl(deps.as_ref(), "hope1_address".to_string(), vec!["ujuno".to_string(),"hope".to_string(),"osmos".to_string(),"xyz".to_string()]).unwrap();
//         assert_eq!(tvl_all,vec![TvlInfo{
//             denom:"ujuno".to_string(),
//             amount:Uint128::new(1000000)
//         },TvlInfo{
//             denom:"hope".to_string(),
//             amount:Uint128::new(2000000)
//         },TvlInfo{
//             denom:"osmos".to_string(),
//             amount:Uint128::new(2000000)
//         },TvlInfo{
//             denom:"xyz".to_string(),
//             amount:Uint128::new(0)
//         }]);

//          let info = mock_info("owner", &[]);
//         let msg = ExecuteMsg::SetTvl { address: "hope1_address".to_string(), tvl: vec![TvlInfo{
//             denom:"ujuno".to_string(),
//             amount:Uint128::new(0)
//         }] };
//         execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//         let tvl_all = query_all_tvl(deps.as_ref(), "hope1_address".to_string(), vec!["ujuno".to_string()]).unwrap();
//         assert_eq!(tvl_all,vec![TvlInfo{
//             denom:"ujuno".to_string(),
//             amount:Uint128::new(0)
//         }]);
       
//     }
// }
    