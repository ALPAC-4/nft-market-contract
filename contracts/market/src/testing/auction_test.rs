use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{to_binary, from_binary, Addr, Coin, CosmosMsg, Decimal, SubMsg, Timestamp, WasmMsg, Uint128};
use cw_storage_plus::U64Key;
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw20::Cw20ReceiveMsg;
use cw0::Expiration;

use crate::{
  state::{AuctionInfo, MarketContract, Royalty},
  msgs::{InstantiateMsg, ExecuteMsg, Cw721HookMsg, QueryMsg},
  error::ContractError,
  asset::{Asset, AssetInfo},
  testing::mock_querier::mock_dependencies
};



#[test]
fn only_auction_order_test() {
  // instantiate
  let market = MarketContract::default();

  let mut deps = mock_dependencies(&[]);

  let instantiate_msg = InstantiateMsg {
    owner: "owner".to_string(),
    min_increase: Decimal::from_ratio(10u128, 100u128),
    max_auction_duration_block: 100,
    max_auction_duration_second: 1000,
    auction_cancel_fee_rate: Decimal::zero(),
  };

  let info = mock_info("owner", &[]);

  let _res = market.instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

  // need tax querier
  deps.querier.with_tax(
    Decimal::from_ratio(1u128, 100u128),
    &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
  );

  // some assetinfos
  let uusd: AssetInfo = AssetInfo::NativeToken { denom: "uusd".to_string()};
  let mir: AssetInfo = AssetInfo::Token { contract_addr: "mir_addr".to_string()};
  let shib: AssetInfo = AssetInfo::Token { contract_addr: "shib_addr".to_string()};

  // some royalties
  let nft_designer_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_designer"),
    royalty_rate: Decimal::from_ratio(2u128, 100u128)
  };

  let nft_pm_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_pm"),
    royalty_rate: Decimal::from_ratio(3u128, 100u128)
  };

  // add collection
  let info = mock_info("owner", &[]);
  let add_collection_msg = ExecuteMsg::AddCollection {
    nft_address: "spaceship".to_string(),
    support_assets: vec![uusd.clone(), mir.clone()],
    royalties: vec![nft_designer_royalty.clone(), nft_pm_royalty.clone()],
  };

  let _res = market.execute(deps.as_mut(), mock_env(), info, add_collection_msg).unwrap();

  let seller = "seller".to_string();

  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  // mock_env's info
  // height: 12_345,
  // time: Timestamp::from_nanos(1_571_797_419_879_305_533),

  // try to make auction order with already expired expiration (height)
  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_000),
    fixed_price: None
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));

  match res {
    Err(ContractError::Expired {}) => assert!(true),
    _ => panic!("Must return token mismatch error"),
  }

  // try to make auction order with already expired expiration (timestamp)
  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtTime(Timestamp::from_nanos(1_571_797_419_879_000_000)),
    fixed_price: None
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));

  match res {
    Err(ContractError::Expired {}) => assert!(true),
    _ => panic!("Must return token mismatch error"),
  }

  // try to make auction order with unsupport asset
  let start_price = Asset{
    info: shib.clone(),
    amount: Uint128::from(100000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(13_000),
    fixed_price: None
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));

  match res {
    Err(ContractError::Unsupport {}) => assert!(true),
    _ => panic!("Must return unsupport error"),
  }

  // try to make auction order with exceed max duration (block)
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(13_000),
    fixed_price: None
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));
  
  match res {
    Err(ContractError::MaxDuration {}) => assert!(true),
    _ => panic!("Must return max duration error"),
  }

  // try to make auction order with exceed max duration (time)
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtTime(Timestamp::from_nanos(1_571_799_419_879_305_533)),
    fixed_price: None
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));

  match res {
    Err(ContractError::MaxDuration {}) => assert!(true),
    _ => panic!("Must return max duration error"),
  }

  // make auction order
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: None
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));

  // check order
  let order = market.orders.load(&deps.storage, U64Key::new(1)).unwrap();

  assert_eq!(1, order.id);
  assert_eq!(Addr::unchecked("seller"), order.seller_address);
  assert_eq!(Addr::unchecked("spaceship"), order.nft_address);
  assert_eq!("no1".to_string(), order.token_id);
  assert_eq!(None, order.price);
  assert_eq!(
    Some(AuctionInfo {
      highest_bid: start_price.clone(),
      bidder: None,
      expiration: Expiration::AtHeight(12_400),
    }),
    order.auction_info
  );

  let mut mock_env = mock_env();
  mock_env.block.height = 12_370;

  // try to bid with lack amount
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(101000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 1,
    bid_price: bid_price,
  };

  let info = mock_info("bidder", &[Coin{ denom: "uusd".to_string(), amount: Uint128::from(101000000u128)}]);

  mock_env.block.height = 12_370;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg);

  match res {
    Err(ContractError::MinPrice { min_bid_amount: _ }) => assert!(true),
    _ => panic!("Must return min price error"),
  }

  // try_to bid with another asset
  let bid_price = Asset{
    info: AssetInfo::NativeToken { denom: "uluna".to_string()},
    amount: Uint128::from(120000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 1,
    bid_price: bid_price,
  };

  let info = mock_info("bidder", &[Coin{ denom: "uluna".to_string(), amount:  Uint128::from(120000000u128)}]);

  mock_env.block.height = 12_370;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg);

  match res {
    Err(ContractError::AssetInfoMismatch {}) => assert!(true),
    _ => panic!("Must return asset info mismatch error"),
  }

  // sent amount mismatch
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 1,
    bid_price: bid_price,
  };

  let info = mock_info("bidder", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(120000000u128)}]);

  mock_env.block.height = 12_370;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg);

  match res {
    Err(ContractError::Std(_) ) => assert!(true),
    _ => panic!("Must return std error"),
  }

  // try to bid after expired
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(120000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 1,
    bid_price: bid_price,
  };

  let info = mock_info("bidder", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(120000000u128)}]);

  mock_env.block.height = 12_500;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg);

  match res {
    Err(ContractError::Expired {} ) => assert!(true),
    _ => panic!("Must return expired error"),
  }

  // first bid (no fund refund)
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(120000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 1,
    bid_price: bid_price.clone(),
  };

  let info = mock_info("bidder1", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(120000000u128)}]);

  mock_env.block.height = 12346;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg).unwrap();

  // empty
  assert_eq!(res.messages, vec![]);

  // check auction info
  let order = market.orders.load(&deps.storage, U64Key::new(1)).unwrap();

  assert_eq!(
    Some(AuctionInfo {
      highest_bid: bid_price.clone(),
      bidder: Some(Addr::unchecked("bidder1")),
      expiration: Expiration::AtHeight(12_400),
    }),
    order.auction_info
  );

  // bid
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(150000000u128)
  };

  // former bid price
  let former_bid = Asset {
    info: uusd.clone(),
    amount: Uint128::from(120000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 1,
    bid_price: bid_price.clone(),
  };

  let info = mock_info("bidder2", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(150000000u128)}]);

  mock_env.block.height = 12347;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg).unwrap();

  assert_eq!(
    res.messages,
    vec![SubMsg::new(former_bid.into_msg(&deps.as_mut().querier, Addr::unchecked("bidder1")).unwrap())]
  );


  // check auction info
  let order = market.orders.load(&deps.storage, U64Key::new(1)).unwrap();

  assert_eq!(
    Some(AuctionInfo {
      highest_bid: bid_price.clone(),
      bidder: Some(Addr::unchecked("bidder2")),
      expiration: Expiration::AtHeight(12_400),
    }),
    order.auction_info
  );

  // try to bid after expired
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(200000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 1,
    bid_price: bid_price.clone(),
  };

  let info = mock_info("bidder2", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(200000000u128)}]);

  mock_env.block.height = 12500;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg);

  match res {
    Err(ContractError::Expired {} ) => assert!(true),
    _ => panic!("Must return expired error"),
  }

  // try to execute auction before expired
  let execute_auction_msg = ExecuteMsg::ExecuteAuction {
    order_id: 1,
  };

  let info = mock_info("anyone", &[]);

  mock_env.block.height = 12390;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, execute_auction_msg);

  match res {
    Err(ContractError::NotExpired {} ) => assert!(true),
    _ => panic!("Must return not expired error"),
  }

  // execute auction
  let execute_auction_msg = ExecuteMsg::ExecuteAuction {
    order_id: 1,
  };

  let info = mock_info("anyone", &[]);

  mock_env.block.height = 12500;

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, execute_auction_msg).unwrap();

  // royalty amounts
  let designer_royalty_asset = Asset {
    info: uusd.clone(),
    amount: Uint128::from(3000000u128)
  };

  let pm_royalty_asset = Asset {
    info: uusd.clone(),
    amount: Uint128::from(4500000u128)
  };

  // remain asset
  let remain_asset = Asset {
    info: uusd.clone(),
    amount: Uint128::from(142500000u128)
  };

  assert_eq!(
    res.messages,
    vec![
      // transfer nft to top bidder
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "bidder2".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
      // transfer royalties 
      SubMsg::new(designer_royalty_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("nft_designer")).unwrap()),
      SubMsg::new(pm_royalty_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("nft_pm")).unwrap()),
      // transfer remain to seller
      SubMsg::new(remain_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("seller")).unwrap()),
    ]
  );

  // check order removed
  let order = market.orders.may_load(&deps.storage, U64Key::new(1));

  assert_eq!(order, Ok(None));

  // auction that no bid execute test
  // make auction
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: None
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);
  mock_env.block.height = 12345;

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveNft(receive_msg));

  mock_env.block.height = 12500;

  let execute_auction_msg = ExecuteMsg::ExecuteAuction {
    order_id: 2,
  };

  let info = mock_info("anyone", &[]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, execute_auction_msg).unwrap();

  assert_eq!(
    res.messages,
    vec![
      // return nft to seller
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: seller.to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
    ]
  );

  // check order removed
  let order = market.orders.may_load(&deps.storage, U64Key::new(2));

  assert_eq!(order, Ok(None));

  // try to bid and execute to not auction order (only fixed price order)

  // make fixed price order
  let price = Asset {
    info: mir.clone(),
    amount: Uint128::from(100000000u128)
  };

  let make_fixed_price_order_msg = Cw721HookMsg::MakeFixedPriceOrder { price: price.clone() };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_fixed_price_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // try to bid
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(120000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 3,
    bid_price: bid_price.clone(),
  };

  let info = mock_info("bidder1", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(120000000u128)}]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg);
  
  match res {
    Err(ContractError::NotAuction {} ) => assert!(true),
    _ => panic!("Must return not auction error"),
  }

  // try to auction execute
  let execute_auction_msg = ExecuteMsg::ExecuteAuction {
    order_id: 3,
  };

  let info = mock_info("anyone", &[]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, execute_auction_msg);

  match res {
    Err(ContractError::NotAuction {} ) => assert!(true),
    _ => panic!("Must return not auction error"),
  }
}

#[test]
fn auction_cancel_test() {
  // instantiate
  let market = MarketContract::default();

  let mut deps = mock_dependencies(&[]);

  let instantiate_msg = InstantiateMsg {
    owner: "owner".to_string(),
    min_increase: Decimal::from_ratio(10u128, 100u128),
    max_auction_duration_block: 100,
    max_auction_duration_second: 1000,
    auction_cancel_fee_rate: Decimal::zero(),
  };

  let info = mock_info("owner", &[]);

  let _res = market.instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

  // need tax querier
  deps.querier.with_tax(
    Decimal::from_ratio(1u128, 100u128),
    &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
  );

  // some assetinfos
  let uusd: AssetInfo = AssetInfo::NativeToken { denom: "uusd".to_string()};
  let mir: AssetInfo = AssetInfo::Token { contract_addr: "mir_addr".to_string()};

  // some royalties
  let nft_designer_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_designer"),
    royalty_rate: Decimal::from_ratio(2u128, 100u128)
  };

  let nft_pm_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_pm"),
    royalty_rate: Decimal::from_ratio(3u128, 100u128)
  };

  // add collection
  let info = mock_info("owner", &[]);
  let add_collection_msg = ExecuteMsg::AddCollection {
    nft_address: "spaceship".to_string(),
    support_assets: vec![uusd.clone(), mir.clone()],
    royalties: vec![nft_designer_royalty.clone(), nft_pm_royalty.clone()],
  };

  let _res = market.execute(deps.as_mut(), mock_env(), info, add_collection_msg).unwrap();

  let seller = "seller".to_string();

  // mock_env's info
  // height: 12_345,
  // time: Timestamp::from_nanos(1_571_797_419_879_305_533),

  // make order
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: uusd.clone(),
    amount: Uint128::from(200000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price)
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // cancel (no bider)

  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 1
  };

  let info = mock_info("seller", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, cancel_msg).unwrap();

  assert_eq!(
    res.messages,
    vec![
      // return nft to seller
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "seller".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
    ]
  );

  // check order removed

  let order = market.orders.may_load(&deps.storage, U64Key::new(1));

  assert_eq!(order, Ok(None));

  // make order
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: uusd.clone(),
    amount: Uint128::from(200000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price)
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // bid
  let mut mock_env = mock_env();

  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(120000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 2,
    bid_price: bid_price.clone(),
  };

  let info = mock_info("bidder1", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(120000000u128)}]);

  mock_env.block.height = 12346;

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg).unwrap();

  // try to cancel auction with balance mismatch
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 2
  };

  let info = mock_info("seller", &[Coin{ denom: "uusd".to_string(), amount: Uint128::from(100u128) }]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, cancel_msg);
  
  match res {
    Err(ContractError::CancelFeeMismatch { fee_asset: _ } ) => assert!(true),
    _ => panic!("Must return cancel fee mismatch error"),
  }

  // cancel auction
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 2
  };

  let info = mock_info("seller", &[]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, cancel_msg).unwrap();

  assert_eq!(
    res.messages,
    vec![
      // return asset to bidder
      SubMsg::new(bid_price.into_msg(&deps.as_mut().querier, Addr::unchecked("bidder1")).unwrap()),
      // return nft to seller
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "seller".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
    ]
  );

  // make order with token
  let start_price = Asset{
    info: mir.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: mir.clone(),
    amount: Uint128::from(200000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price)
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // bid
  let bid_price = Asset{
    info: mir.clone(),
    amount: Uint128::from(150000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 3,
    bid_price: bid_price.clone(),
  };

  let mut mock_env = mock_env;

  mock_env.block.height = 12370;

  let info = mock_info("mir_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "bidder".to_string(),
    amount: Uint128::from(150000000u128),
    msg: to_binary(&bid_msg).unwrap()
  };

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveToken(receive_msg)).unwrap();

  // cancel auction
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 3
  };

  let info = mock_info("seller", &[]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, cancel_msg).unwrap();

  assert_eq!(
    res.messages,
    vec![
      // return asset to bidder
      SubMsg::new(bid_price.into_msg(&deps.as_mut().querier, Addr::unchecked("bidder")).unwrap()),
      // return nft to seller
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "seller".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
    ]
  );

  // update config non zero cancel fee
  mock_env.block.height = 12345;
  let info = mock_info("owner", &[]);
  let update_config_msg = ExecuteMsg::UpdateConfig {
    owner: None,
    min_increase: None,
    max_auction_duration_block: None,
    max_auction_duration_second: None,
    auction_cancel_fee_rate: Some(Decimal::from_ratio(5u128, 1000u128))
  };

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, update_config_msg).unwrap();

  // make order
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: uusd.clone(),
    amount: Uint128::from(200000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price)
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // bid
  let bid_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(120000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 4,
    bid_price: bid_price.clone(),
  };

  let info = mock_info("bidder1", &[Coin{ denom: "uusd".to_string(), amount:  Uint128::from(120000000u128)}]);

  mock_env.block.height = 12346;

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, bid_msg).unwrap();

  // try to cancel auction with balance mismatch
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 4
  };

  let info = mock_info("seller", &[Coin{ denom: "uusd".to_string(), amount: Uint128::from(100u128) }]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, cancel_msg);
  
  match res {
    Err(ContractError::CancelFeeMismatch { fee_asset: _ } ) => assert!(true),
    _ => panic!("Must return cancel fee mismatch error"),
  }

  // cancel auction
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 4
  };

  let fee_asset: Asset = from_binary(&market.query(deps.as_ref(), QueryMsg::CancelFee { order_id: 4 }).unwrap()).unwrap();

  let refund_asset: Asset = Asset {
    info: fee_asset.info.clone(),
    amount: bid_price.amount + fee_asset.amount
  };

  let info = mock_info("seller", &[Coin { denom: fee_asset.info.to_string(), amount: fee_asset.amount }]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, cancel_msg).unwrap();

  assert_eq!(
    res.messages,
    vec![
      // return asset to bidder
      SubMsg::new(refund_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("bidder1")).unwrap()),
      // return nft to seller
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "seller".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
    ]
  );

  // make order with token
  let start_price = Asset{
    info: mir.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: mir.clone(),
    amount: Uint128::from(200000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price)
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // bid
  let bid_price = Asset{
    info: mir.clone(),
    amount: Uint128::from(150000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 5,
    bid_price: bid_price.clone(),
  };

  let mut mock_env = mock_env;

  mock_env.block.height = 12370;

  let info = mock_info("mir_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "bidder".to_string(),
    amount: Uint128::from(150000000u128),
    msg: to_binary(&bid_msg).unwrap()
  };

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveToken(receive_msg)).unwrap();

  // try cancel auction without send
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 5
  };

  let info = mock_info("seller", &[]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, cancel_msg);

  match res {
    Err(ContractError::CancelFeeMismatch { fee_asset: _ } ) => assert!(true),
    _ => panic!("Must return cancel fee mismatch error"),
  }

  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 5
  };

  let fee_asset: Asset = from_binary(&market.query(deps.as_ref(), QueryMsg::CancelFee { order_id: 5 }).unwrap()).unwrap();

  let refund_asset: Asset = Asset {
    info: fee_asset.info.clone(),
    amount: bid_price.amount + fee_asset.amount
  };

  let receive_msg = Cw20ReceiveMsg {
    amount: fee_asset.amount,
    sender: seller.clone(),
    msg: to_binary(&cancel_msg).unwrap()
  };

  let info = mock_info("mir_addr", &[]);

  let res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveToken(receive_msg)).unwrap();

  assert_eq!(
    res.messages,
    vec![
      // return asset to bidder
      SubMsg::new(refund_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("bidder")).unwrap()),
      // return nft to seller
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "seller".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
    ]
  );
}

#[test]
fn auction_with_fixed_price_order_test() {
  // instantiate
  let market = MarketContract::default();

  let mut deps = mock_dependencies(&[]);

  let instantiate_msg = InstantiateMsg {
    owner: "owner".to_string(),
    min_increase: Decimal::from_ratio(10u128, 100u128),
    max_auction_duration_block: 100,
    max_auction_duration_second: 1000,
    auction_cancel_fee_rate: Decimal::from_ratio(5u128, 1000u128)
  };

  let info = mock_info("owner", &[]);

  let _res = market.instantiate(deps.as_mut(), mock_env(), info.clone(), instantiate_msg).unwrap();

  // need tax querier
  deps.querier.with_tax(
    Decimal::from_ratio(1u128, 100u128),
    &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
  );

  // some assetinfos
  let uusd: AssetInfo = AssetInfo::NativeToken { denom: "uusd".to_string()};
  let mir: AssetInfo = AssetInfo::Token { contract_addr: "mir_addr".to_string()};

  // some royalties
  let nft_designer_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_designer"),
    royalty_rate: Decimal::from_ratio(2u128, 100u128)
  };

  let nft_pm_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_pm"),
    royalty_rate: Decimal::from_ratio(3u128, 100u128)
  };

  // add collection
  let info = mock_info("owner", &[]);
  let add_collection_msg = ExecuteMsg::AddCollection {
    nft_address: "spaceship".to_string(),
    support_assets: vec![uusd.clone(), mir.clone()],
    royalties: vec![nft_designer_royalty.clone(), nft_pm_royalty.clone()],
  };

  let _res = market.execute(deps.as_mut(), mock_env(), info, add_collection_msg).unwrap();

  let seller = "seller".to_string();

  // mock_env's info
  // height: 12_345,
  // time: Timestamp::from_nanos(1_571_797_419_879_305_533),

  // try to make auction order with diffrent asset btw auction and fixed price
  let start_price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: mir.clone(),
    amount: Uint128::from(100000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price)
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));

  match res {
    Err(ContractError::AssetInfoMismatch {} ) => assert!(true),
    _ => panic!("Must return asset info mismatch error"),
  }

  // make order 
  let start_price = Asset{
    info: mir.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: mir.clone(),
    amount: Uint128::from(200000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price.clone())
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // check order
  let order = market.orders.load(&deps.storage, U64Key::new(1)).unwrap();

  assert_eq!(1, order.id);
  assert_eq!(Addr::unchecked("seller"), order.seller_address);
  assert_eq!(Addr::unchecked("spaceship"), order.nft_address);
  assert_eq!("no1".to_string(), order.token_id);
  assert_eq!(Some(fixed_price.clone()), order.price);
  assert_eq!(
    Some(AuctionInfo {
      highest_bid: start_price.clone(),
      bidder: None,
      expiration: Expiration::AtHeight(12_400),
    }),
    order.auction_info
  );

  // execute order before bid
  let execute_msg = ExecuteMsg::ExecuteOrder { order_id: 1 };

  let info = mock_info("mir_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "buyer".to_string(),
    amount: Uint128::from(200000000u128),
    msg: to_binary(&execute_msg).unwrap()
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveToken(receive_msg)).unwrap();

    // royalty amounts
    let designer_royalty_asset = Asset {
      info: mir.clone(),
      amount: Uint128::from(4000000u128)
    };
  
    let pm_royalty_asset = Asset {
      info: mir.clone(),
      amount: Uint128::from(6000000u128)
    };
  
    // remain asset
    let remain_asset = Asset {
      info: mir.clone(),
      amount: Uint128::from(190000000u128)
    };

  assert_eq!(
    res.messages,
    vec![
      // transfer nft to buyer
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "buyer".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
      // transfer royalties 
      SubMsg::new(designer_royalty_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("nft_designer")).unwrap()),
      SubMsg::new(pm_royalty_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("nft_pm")).unwrap()),
      // transfer remain to seller
      SubMsg::new(remain_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("seller")).unwrap()),
    ]
  );

  // make order 
  let start_price = Asset{
    info: mir.clone(),
    amount: Uint128::from(100000000u128)
  };

  let fixed_price = Asset {
    info: mir.clone(),
    amount: Uint128::from(200000000u128)
  };

  let make_auction_order_msg = Cw721HookMsg::MakeAuctionOrder { 
    start_price: start_price.clone(),
    expiration: Expiration::AtHeight(12_400),
    fixed_price: Some(fixed_price.clone())
  };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_auction_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // bid
  let bid_price = Asset{
    info: mir.clone(),
    amount: Uint128::from(150000000u128)
  };

  let bid_msg = ExecuteMsg::Bid {
    order_id: 2,
    bid_price: bid_price.clone(),
  };

  let mut mock_env = mock_env();

  mock_env.block.height = 12370;

  let info = mock_info("mir_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "bidder".to_string(),
    amount: Uint128::from(150000000u128),
    msg: to_binary(&bid_msg).unwrap()
  };

  let _res = market.execute(deps.as_mut(), mock_env.clone(), info, ExecuteMsg::ReceiveToken(receive_msg)).unwrap();

  // execute order
  let execute_msg = ExecuteMsg::ExecuteOrder { order_id: 2 };

  let info = mock_info("mir_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "buyer".to_string(),
    amount: Uint128::from(200000000u128),
    msg: to_binary(&execute_msg).unwrap()
  };

  let res = market.execute(deps.as_mut(), mock_env, info, ExecuteMsg::ReceiveToken(receive_msg)).unwrap();

    // royalty amounts
    let designer_royalty_asset = Asset {
      info: mir.clone(),
      amount: Uint128::from(4000000u128)
    };
  
    let pm_royalty_asset = Asset {
      info: mir.clone(),
      amount: Uint128::from(6000000u128)
    };
  
    // remain asset
    let remain_asset = Asset {
      info: mir.clone(),
      amount: Uint128::from(190000000u128)
    };

  assert_eq!(
    res.messages,
    vec![
      // refund asset to last bidder
      SubMsg::new(bid_price.into_msg(&deps.as_mut().querier, Addr::unchecked("bidder")).unwrap()),
      // transfer nft to buyer
      SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: "spaceship".to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
          recipient: "buyer".to_string(), 
          token_id: "no1".to_string()
        }).unwrap(),
        funds: vec![]
      })),
      // transfer royalties 
      SubMsg::new(designer_royalty_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("nft_designer")).unwrap()),
      SubMsg::new(pm_royalty_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("nft_pm")).unwrap()),
      // transfer remain to seller
      SubMsg::new(remain_asset.into_msg(&deps.as_mut().querier, Addr::unchecked("seller")).unwrap()),
    ]
  );
}