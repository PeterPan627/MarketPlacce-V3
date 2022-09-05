#[cfg(test)]
use crate::error::ContractError;
use crate::contract::{execute, instantiate};
use crate::msg::{ExecuteMsg, InstantiateMsg, SellNft, BuyNft, CollectionOffset, CollectionOffsetBid, SaleHistoryOffset};
use crate::query::{query_ask_count, query_asks_by_seller, query_bids_by_bidder, query_state_info, query_ask, query_asks, query_bids, query_bids_by_seller, query_bids_by_bidder_sorted_by_expiry, query_tvl_by_collection, query_tvl_by_denom, query_sale_history, query_sale_history_by_token_id};
use crate::state::{ask_key, asks, bid_key, bids, Ask, Bid, SaleType, Asset, UserInfo};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, coins, Addr, DepsMut, Timestamp, Uint128,to_binary, Env, Decimal, CosmosMsg, WasmMsg, StdResult, Response, Coin, BankMsg};
use cw721::{Cw721ReceiveMsg,Cw721ExecuteMsg};
use cw20::{Cw20ReceiveMsg, Cw20ExecuteMsg};

fn setup_contract(deps: DepsMut){
   let instantiate_msg = InstantiateMsg {
        owner:"owner".to_string()
    };
    let info = mock_info("owner", &[]);
    let res = instantiate(deps, mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(res.messages.len(), 0);
}

fn add_contract(deps: DepsMut, env: Env, address:String ){
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::AddCollection {
       royalty_portion: Decimal::from_ratio(1 as u128, 10 as u128),
       nft_address: address,
       members: vec![UserInfo{
        address:"admin1".to_string(),
        portion: Decimal::from_ratio(7 as u128, 10 as u128)
       },
       UserInfo{
        address:"admin2".to_string(),
        portion: Decimal::from_ratio(3 as u128, 10 as u128)
       }] 
    };
    let res = execute(deps, env, info, msg).unwrap();
    assert_eq!(res.messages.len(),0)
}


fn add_coin(deps: DepsMut, env: Env, denom:String ){
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::AddCoin { symbol: denom };
    let res = execute(deps, env, info, msg).unwrap();
    assert_eq!(res.messages.len(),0)
}


fn add_token(deps: DepsMut, env: Env, denom: String, address: String ){
    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::AddTokenAddress { symbol: denom, address: address };
    let res = execute(deps, env, info, msg).unwrap();
    assert_eq!(res.messages.len(),0)
}

fn sell_nft(deps: DepsMut, env: Env,collection:&str, sender: String, denom: String, amount: Uint128, token_address: Option<String>, token_id:String){
  let sell_msg = SellNft{
    list_price:Asset{
      denom,
      amount
    },
    expire:Timestamp::from_seconds(env.block.time.seconds() + 300),
    token_address
  };

  let info = mock_info(collection, &[]);
  let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg{
      sender,
      token_id,
      msg:to_binary(&sell_msg).unwrap()
  });
  execute(deps, env, info.clone(), msg).unwrap();
}

fn bid_nft_with_token(deps: DepsMut,env: Env, collection: String, token_id: Option<String>, token_address: &str, sender:String, amount:Uint128) -> StdResult<Response>{
   let bid_msg = BuyNft{
     nft_address: collection,
     expire: Timestamp::from_seconds(env.block.time.seconds() + 300),
     sale_type: SaleType::Auction,
     token_id
   };

   let info = mock_info(token_address,&[]);
   let msg = ExecuteMsg::Receive(Cw20ReceiveMsg{
      sender,
      amount,
      msg: to_binary(&bid_msg).unwrap()
    });

   let res = execute(deps, env, info, msg).unwrap();
   Ok(res)
}


fn bid_nft_with_coin(deps: DepsMut,env: Env, collection: String, token_id: Option<String>, sender:&str, denom:String, amount:Uint128) -> StdResult<Response>{
  
   let info = mock_info(sender,&[Coin{
    denom: denom.clone(),
    amount: amount
   }]);
   let msg = ExecuteMsg::SetBidCoin { 
    nft_address: collection, 
    expire: Timestamp::from_seconds(env.block.time.seconds() + 300), 
    sale_type: SaleType::Auction, 
    token_id, 
    list_price: Asset { denom, amount } 
   };

   let res = execute(deps, env, info, msg).unwrap();
   Ok(res)
}


#[test]
fn init_contract() {
    let mut deps = mock_dependencies();
    let instantiate_msg = InstantiateMsg {
        owner:"owner".to_string()
    };
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());
    let state = query_state_info(deps.as_ref()).unwrap();
    assert_eq!(state.owner,"owner".to_string());
}

#[test]
fn put_nft_sale() {
  let mut deps = mock_dependencies();
  let env = mock_env();

  //init contract
  setup_contract(deps.as_mut());

  //add collection
  add_contract(deps.as_mut(), env.clone() , "collection1".to_string() );
  add_contract(deps.as_mut(), env.clone() , "collection2".to_string() );

  //add coin
  add_coin(deps.as_mut(), env.clone(), "ujuno".to_string());
  add_token(deps.as_mut(), env.clone(), "hope".to_string(), "hope_address".to_string());


  sell_nft(
    deps.as_mut(),
    env.clone(),
    "collection1", 
    "seller1".to_string(), 
    "ujuno".to_string(), 
    Uint128::new(10000), 
    None, 
    "Hope.1".to_string()
  );
  
  sell_nft(
    deps.as_mut(),
    env.clone(), 
    "collection1", 
    "seller1".to_string(), 
    "hope".to_string(), 
    Uint128::new(10000), 
    Some("hope_address".to_string()), 
    "Punk.1".to_string()
  );

 let _individual_ask = query_ask(deps.as_ref(), "collection1".to_string(), "Hope.1".to_string()).unwrap();
 //  println!("{:?}",result)
 let _ask_by_seller = query_asks_by_seller(deps.as_ref(), "seller1".to_string(), None, Some(20)).unwrap();
 //println!("{:?}", ask_by_seller);
 let _ask_by_collection = query_asks(deps.as_ref(), "collection1".to_string(), Some("Hope.1".to_string()), Some(20)).unwrap();
 //  println!("{:?}", _ask_by_collection);
}


#[test]
fn test_bid() {
  let mut deps = mock_dependencies();
  let env = mock_env();

  //init contract
  setup_contract(deps.as_mut());

  //add collection
  add_contract(deps.as_mut(), env.clone() , "collection1".to_string() );
  add_contract(deps.as_mut(), env.clone() , "collection2".to_string() );

  //add coin
  add_coin(deps.as_mut(), env.clone(), "ujuno".to_string());
  add_token(deps.as_mut(), env.clone(), "hope".to_string(), "hope_address".to_string());


  sell_nft(
    deps.as_mut(),
    env.clone(),
    "collection1", 
    "seller1".to_string(), 
    "ujuno".to_string(), 
    Uint128::new(10000), 
    None, 
    "Hope.1".to_string()
  );
  
  sell_nft(
    deps.as_mut(),
    env.clone(), 
    "collection2", 
    "seller1".to_string(), 
    "hope".to_string(), 
    Uint128::new(10000), 
    Some("hope_address".to_string()), 
    "Punk.1".to_string()
  );

  let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection1".to_string(),
    Some("Hope.1".to_string()),
    "hope_address",
    "bider1".to_string(),
    Uint128::new(3000)
  ).unwrap();
  // println!("{:?}",result.messages.len());

  let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection1".to_string(),
    Some("Hope.1".to_string()),
    "hope_address",
    "bider2".to_string(),
    Uint128::new(5000)
  ).unwrap();
  // println!("{:?}",result.messages.len());

  let _result = bid_nft_with_coin(
     deps.as_mut(), 
    env.clone(), 
    "collection1".to_string(), 
    Some("Hope.1".to_string()), 
    "bider3", 
    "ujuno".to_string(), 
    Uint128::new(7000)
 
  ).unwrap();

 

  let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection2".to_string(), 
    Some("Punk.1".to_string()), 
    "bider1", 
    "ujuno".to_string(), 
    Uint128::new(3000)
  ).unwrap();

  
  let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection2".to_string(), 
    Some("Punk.1".to_string()), 
    "bider2", 
    "ujuno".to_string(), 
    Uint128::new(5000)
  ).unwrap();

  let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection2".to_string(), 
    Some("Punk.1".to_string()), 
    "bider3", 
    "ujuno".to_string(), 
    Uint128::new(5000)
  ).unwrap();


  
  let info = mock_info("bider3", &[]);
  let msg = ExecuteMsg::RemoveBid { nft_address: "collection1".to_string(), token_id: "Hope.1".to_string() };
  let res = execute(deps.as_mut(), env, info, msg).unwrap();
  assert_eq!(res.messages.len(),1);
  assert_eq!(res.messages[0].msg, CosmosMsg::Bank(BankMsg::Send { 
    to_address: "bider3".to_string(), 
    amount: vec![Coin{
      denom:"ujuno".to_string(),
      amount:Uint128::new(7000)
    }] }));

  let _bids_by_bidder = query_bids_by_bidder(deps.as_ref(), "bider3".to_string(), None, Some(20)).unwrap();
  println!("{:?}",_bids_by_bidder);

  let _bids_collection = query_bids(deps.as_ref(), "collection1".to_string() , "Hope.1".to_string(), None, Some(30));
  let _bids_by_seller = query_bids_by_seller(deps.as_ref(), "seller1".to_string(), Some(CollectionOffsetBid{
    collection: "collection1".to_string(),
    token_id: "Hope.1".to_string(),
    bidder: "bider1".to_string(),
  }), Some(10)).unwrap();

  // let _bids_expires = query_bids_by_bidder_sorted_by_expiry(deps.as_ref(), "bider1".to_string(), None, Some(10)).unwrap(); 
  // println!("{:?}",_bids_expires)
}

#[test] 
fn withdraw_nft() {
  let mut deps = mock_dependencies();
  let env = mock_env();

  //init contract
  setup_contract(deps.as_mut());

  //add collection
  add_contract(deps.as_mut(), env.clone() , "collection1".to_string() );
  add_contract(deps.as_mut(), env.clone() , "collection2".to_string() );

  //add coin
  add_coin(deps.as_mut(), env.clone(), "ujuno".to_string());
  add_token(deps.as_mut(), env.clone(), "hope".to_string(), "hope_address".to_string());


  sell_nft(
    deps.as_mut(),
    env.clone(),
    "collection1", 
    "seller1".to_string(), 
    "ujuno".to_string(), 
    Uint128::new(10000), 
    None, 
    "Hope.1".to_string()
  );
  
  sell_nft(
    deps.as_mut(),
    env.clone(), 
    "collection2", 
    "seller1".to_string(), 
    "hope".to_string(), 
    Uint128::new(10000), 
    Some("hope_address".to_string()), 
    "Punk.1".to_string()
  );

  let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection1".to_string(),
    Some("Hope.1".to_string()),
    "hope_address",
    "bider1".to_string(),
    Uint128::new(3000)
  ).unwrap();
  // println!("{:?}",result.messages.len());

   let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection1".to_string(),
    Some("Hope.1".to_string()),
    "hope_address",
    "bider2".to_string(),
    Uint128::new(5000)
  ).unwrap();

    let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection1".to_string(), 
    Some("Hope.1".to_string()), 
    "bider3", 
    "ujuno".to_string(), 
    Uint128::new(7000)
  ).unwrap();



  // println!("{:?}",result.messages.len()); 

  let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection2".to_string(), 
    Some("Punk.1".to_string()), 
    "bider1", 
    "ujuno".to_string(), 
    Uint128::new(3000)
  ).unwrap();

  
  let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection2".to_string(), 
    Some("Punk.1".to_string()), 
    "bider2", 
    "ujuno".to_string(), 
    Uint128::new(7000)
  ).unwrap();

  let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection2".to_string(),
    Some("Punk.1".to_string()),
    "hope_address",
    "bider3".to_string(),
    Uint128::new(7000)
  ).unwrap();
  

  let info = mock_info("seller1",&[]);
  let msg = ExecuteMsg::WithdrawNft { 
    nft_address: "collection1".to_string(), 
    token_id: "Hope.1".to_string() 
  };
  let res = execute(deps.as_mut(), env, info, msg).unwrap();
  assert_eq!(res.messages.len(),4);

  assert_eq!(res.messages[0].msg, CosmosMsg::Wasm(WasmMsg::Execute{
     contract_addr: "hope_address".to_string(), 
     msg: to_binary(&Cw20ExecuteMsg::Transfer{ recipient: "bider1".to_string(), amount: Uint128::new(3000) }).unwrap(), 
     funds: vec![] }));
  
  assert_eq!(res.messages[1].msg, CosmosMsg::Wasm(WasmMsg::Execute{
  contract_addr: "hope_address".to_string(), 
  msg: to_binary(&Cw20ExecuteMsg::Transfer{ recipient: "bider2".to_string(), amount: Uint128::new(5000) }).unwrap(), 
  funds: vec![] }));

  assert_eq!(res.messages[2].msg, CosmosMsg::Bank(BankMsg::Send{ to_address: "bider3".to_string(), amount: vec![Coin{
    denom : "ujuno".to_string(),
    amount : Uint128::new(7000)
  }] }));

  assert_eq!(res.messages[3].msg, CosmosMsg::Wasm(WasmMsg::Execute { contract_addr: "collection1".to_string(), 
  msg: to_binary(&Cw721ExecuteMsg::TransferNft{ recipient: "seller1".to_string(), token_id: "Hope.1".to_string() }).unwrap(), 
  funds: vec![] 
  }));

}



#[test] 
fn accept_bid() {
  let mut deps = mock_dependencies();
  let env = mock_env();

  //init contract
  setup_contract(deps.as_mut());

  //add collection
  add_contract(deps.as_mut(), env.clone() , "collection1".to_string() );
  add_contract(deps.as_mut(), env.clone() , "collection2".to_string() );

  //add coin
  add_coin(deps.as_mut(), env.clone(), "ujuno".to_string());
  add_token(deps.as_mut(), env.clone(), "hope".to_string(), "hope_address".to_string());


  sell_nft(
    deps.as_mut(),
    env.clone(),
    "collection1", 
    "seller1".to_string(), 
    "ujuno".to_string(), 
    Uint128::new(10000), 
    None, 
    "Hope.1".to_string()
  );
  
  sell_nft(
    deps.as_mut(),
    env.clone(), 
    "collection2", 
    "seller1".to_string(), 
    "hope".to_string(), 
    Uint128::new(10000), 
    Some("hope_address".to_string()), 
    "Punk.1".to_string()
  );

  let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection1".to_string(),
    Some("Hope.1".to_string()),
    "hope_address",
    "bider1".to_string(),
    Uint128::new(3000)
  ).unwrap();
  // println!("{:?}",result.messages.len());

   let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection1".to_string(),
    Some("Hope.1".to_string()),
    "hope_address",
    "bider2".to_string(),
    Uint128::new(5000)
  ).unwrap();

    let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection1".to_string(), 
    Some("Hope.1".to_string()), 
    "bider3", 
    "ujuno".to_string(), 
    Uint128::new(7000)
  ).unwrap();



  // println!("{:?}",result.messages.len()); 

  let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection2".to_string(), 
    Some("Punk.1".to_string()), 
    "bider1", 
    "ujuno".to_string(), 
    Uint128::new(3000)
  ).unwrap();

  
  let _result = bid_nft_with_coin(
    deps.as_mut(), 
    env.clone(), 
    "collection2".to_string(), 
    Some("Punk.1".to_string()), 
    "bider2", 
    "ujuno".to_string(), 
    Uint128::new(5000)
  ).unwrap();

  let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection2".to_string(),
    Some("Punk.1".to_string()),
    "hope_address",
    "bider3".to_string(),
    Uint128::new(7000)
  ).unwrap();


  

  let info = mock_info("seller1",&[]);
  let msg = ExecuteMsg::AcceptBid { nft_address: "collection1".to_string(), token_id: "Hope.1".to_string(), bidder: "bider1".to_string() };
  let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

  assert_eq!(res.messages[0].msg, CosmosMsg::Wasm(WasmMsg::Execute{
     contract_addr: "hope_address".to_string(), 
     msg: to_binary(&Cw20ExecuteMsg::Transfer{ recipient: "bider2".to_string(), amount: Uint128::new(5000) }).unwrap(), 
     funds: vec![] }));

  assert_eq!(res.messages[1].msg, CosmosMsg::Bank(BankMsg::Send{ to_address: "bider3".to_string(), amount: vec![Coin{
    denom : "ujuno".to_string(),
    amount : Uint128::new(7000)
  }] }));

  assert_eq!(res.messages[2].msg, CosmosMsg::Wasm(WasmMsg::Execute{
     contract_addr: "hope_address".to_string(), 
     msg: to_binary(&Cw20ExecuteMsg::Transfer{ recipient: "admin1".to_string(), amount: Uint128::new(210) }).unwrap(), 
     funds: vec![] }));

     
  assert_eq!(res.messages[3].msg, CosmosMsg::Wasm(WasmMsg::Execute{
     contract_addr: "hope_address".to_string(), 
     msg: to_binary(&Cw20ExecuteMsg::Transfer{ recipient: "admin2".to_string(), amount: Uint128::new(90) }).unwrap(), 
     funds: vec![] }));

  assert_eq!(res.messages[4].msg, CosmosMsg::Wasm(WasmMsg::Execute{
     contract_addr: "hope_address".to_string(), 
     msg: to_binary(&Cw20ExecuteMsg::Transfer{ recipient: "seller1".to_string(), amount: Uint128::new(2700) }).unwrap(), 
     funds: vec![] }));

  assert_eq!(res.messages[5].msg, CosmosMsg::Wasm(WasmMsg::Execute{
    contract_addr: "collection1".to_string(), 
    msg: to_binary(&Cw721ExecuteMsg::TransferNft { recipient: "bider1".to_string(), token_id: "Hope.1".to_string() }).unwrap(), 
    funds: vec![] }));

 
  let info = mock_info("seller1",&[]);
  let msg = ExecuteMsg::AcceptBid { nft_address: "collection2".to_string(), token_id: "Punk.1".to_string(), bidder: "bider3".to_string() };
  let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
  assert_eq!(res.messages.len(),6);

  let _ask_by_collection = query_asks(deps.as_ref(), "collection1".to_string(), Some("Hope.1".to_string()), Some(20)).unwrap();
  //println!("{:?}",_ask_by_collection);
  let _ask_by_collection = query_asks(deps.as_ref(), "collection2".to_string(), Some("Hope.1".to_string()), Some(20)).unwrap();
  //println!("{:?}",_ask_by_collection);

  
  sell_nft(
    deps.as_mut(),
    env.clone(),
    "collection1", 
    "seller1".to_string(), 
    "ujuno".to_string(), 
    Uint128::new(10000), 
    None, 
    "Hope.1".to_string()
  );

  let _result = bid_nft_with_token(
    deps.as_mut(),
    env.clone(), 
    "collection1".to_string(),
    Some("Hope.1".to_string()),
    "hope_address",
    "bider1".to_string(),
    Uint128::new(5000)
  ).unwrap();


  let info = mock_info("seller1",&[]);
  let msg = ExecuteMsg::AcceptBid { nft_address: "collection1".to_string(), token_id: "Hope.1".to_string(), bidder: "bider1".to_string() };
  let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

  let tvl = query_tvl_by_collection(deps.as_ref(), "collection1".to_string(), None, Some(30)).unwrap();
  println!("{:?}",tvl);
  let tvl = query_tvl_by_denom(deps.as_ref(), "hope".to_string(), None, Some(20)).unwrap();

  let  sale_history_by_token_id = query_sale_history_by_token_id(deps.as_ref(), "collection1".to_string(), "Hope.1".to_string(), None, Some(20)).unwrap();
  println!("{:?}", sale_history_by_token_id);

  // let sale_history__collection_token_id = query_sale_history_by_token_id(deps, collection, token_id, start_after, limit)


  // assert_eq!(res.messages[3].msg, CosmosMsg::Wasm(WasmMsg::Execute { contract_addr: "collection1".to_string(), 
  // msg: to_binary(&Cw721ExecuteMsg::TransferNft{ recipient: "seller1".to_string(), token_id: "Hope.1".to_string() }).unwrap(), 
  // funds: vec![] 
  // }));

  // let _bids_by_bidder = query_bids_by_bidder(deps.as_ref(), "bider3".to_string(), None, Some(20)).unwrap();
  // println!("{:?}",_bids_by_bidder);

  // let _ask_by_seller = query_asks_by_seller(deps.as_ref(), "seller1".to_string(), None, Some(20)).unwrap();
  // println!("{:?}",_ask_by_seller)
}