use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, SubMsg, WasmMsg, Uint128};
use cw_storage_plus::U64Key;
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw20::Cw20ReceiveMsg;

use crate::{
  state::{MarketContract, Royalty},
  msgs::{InstantiateMsg, ExecuteMsg, Cw721HookMsg, Cw20HookMsg},
  error::ContractError,
  asset::{Asset, AssetInfo},
  testing::mock_querier::mock_dependencies
};

#[test]
fn fixed_price_order_test() {
  // instantiate
  let market = MarketContract::default();

  let mut deps = mock_dependencies(&[]);

  let instantiate_msg = InstantiateMsg {
    owner: "owner".to_string(),
    min_increase: Decimal::from_ratio(10u128, 100u128),
    max_auction_duration_block: 100,
    max_auction_duration_second: 1000,
    auction_cancel_fee_rate: Decimal::from_ratio(5u128, 1000u128),
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

  let price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  // make fixed_price order
  let make_fixed_price_order_msg = Cw721HookMsg::MakeFixedPriceOrder { price: price.clone() };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_fixed_price_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();
  
  // check index increase
  let order_index = market.order_index.load(&deps.storage).unwrap();

  assert_eq!(2, order_index);

  // check order
  let order = market.orders.load(&deps.storage, U64Key::new(1)).unwrap();

  assert_eq!(1, order.id);
  assert_eq!(Addr::unchecked("seller"), order.seller_address);
  assert_eq!(Addr::unchecked("spaceship"), order.nft_address);
  assert_eq!("no1".to_string(), order.token_id);
  assert_eq!(Some(price), order.price);
  assert_eq!(None, order.auction_info);

  // try to make order with unsupport asset
  let price = Asset {
    info: shib.clone(),
    amount: Uint128::from(999999u128)
  };

  let make_fixed_price_order_msg = Cw721HookMsg::MakeFixedPriceOrder { price: price.clone() };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "shib_no1".to_string(),
    msg: to_binary(&make_fixed_price_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg));

  match res {
    Err(ContractError::Unsupport {}) => assert!(true),
    _ => panic!("Must return unsupport error"),
  }

  // who is not seller try to cancel order 
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 1
  };

  let info = mock_info("not_seller", &[]);

  let res = market.execute(deps.as_mut(), mock_env(), info, cancel_msg);

  match res {
    Err(ContractError::Unauthorized {}) => assert!(true),
    _ => panic!("Must return unauthorized error"),
  }

  // cancel order
  let cancel_msg = ExecuteMsg::CancelOrder {
    order_id: 1
  };

  let info = mock_info("seller", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, cancel_msg);

  // check removed
  let order = market.orders.may_load(&deps.storage, U64Key::new(1));

  assert_eq!(order, Ok(None));

  // execute order case1. native asset
  let price = Asset{
    info: uusd.clone(),
    amount: Uint128::from(100000000u128)
  };

  // remake order for test
  let make_fixed_price_order_msg = Cw721HookMsg::MakeFixedPriceOrder { price: price.clone() };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_fixed_price_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // try to execute order with balance missmatch
  let execute_msg = ExecuteMsg::ExecuteOrder { order_id: 2 };

  let info = mock_info("buyer", &[Coin{ denom: "uusd".to_string(), amount: Uint128::from(12312412u128) }]);

  let res = market.execute(deps.as_mut(), mock_env(), info, execute_msg);

  match res {
    Err(ContractError::Std(_)) => assert!(true),
    _ => panic!("Must return error"),
  }

  // try to execute order with another native token
  let execute_msg = ExecuteMsg::ExecuteOrder { order_id: 2 };

  let info = mock_info("buyer", &[Coin{ denom: "uluna".to_string(), amount: Uint128::from(100000000u128) }]);

  let res = market.execute(deps.as_mut(), mock_env(), info, execute_msg);

  match res {
    Err(ContractError::Std(_)) => assert!(true),
    _ => panic!("Must return error"),
  }

  // execute order
  let execute_msg = ExecuteMsg::ExecuteOrder { order_id: 2 };

  let info = mock_info("buyer", &[Coin{ denom: "uusd".to_string(), amount: Uint128::from(100000000u128) }]);

  let res = market.execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap();

  // royalty amounts
  let designer_royalty_asset = Asset {
    info: uusd.clone(),
    amount: Uint128::from(2000000u128)
  };

  let pm_royalty_asset = Asset {
    info: uusd.clone(),
    amount: Uint128::from(3000000u128)
  };

  // remain asset
  let remain_asset = Asset {
    info: uusd.clone(),
    amount: Uint128::from(95000000u128)
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

  // check order removed
  let order = market.orders.may_load(&deps.storage, U64Key::new(2));

  assert_eq!(order, Ok(None));


  // execute order case2. non-native asset
  let price = Asset{
    info: mir.clone(),
    amount: Uint128::from(100000000u128)
  };

  // remake order for test
  let make_fixed_price_order_msg = Cw721HookMsg::MakeFixedPriceOrder { price: price.clone() };

  let receive_msg: Cw721ReceiveMsg = Cw721ReceiveMsg {
    sender: seller.clone(),
    token_id: "no1".to_string(),
    msg: to_binary(&make_fixed_price_order_msg).unwrap(),
  };

  let info = mock_info("spaceship", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveNft(receive_msg)).unwrap();

  // try to execute order with balance missmatch
  let execute_msg = Cw20HookMsg::ExecuteOrder { order_id: 3 };

  let info = mock_info("mir_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "buyer".to_string(),
    amount: Uint128::from(123123123u128),
    msg: to_binary(&execute_msg).unwrap()
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveToken(receive_msg));

  match res {
    Err(ContractError::TokenMismatch {}) => assert!(true),
    _ => panic!("Must return token mismatch error"),
  }

  // try to execute order with another token
  let execute_msg = Cw20HookMsg::ExecuteOrder { order_id: 3 };

  let info = mock_info("shib_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "buyer".to_string(),
    amount: Uint128::from(100000000u128),
    msg: to_binary(&execute_msg).unwrap()
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveToken(receive_msg));

  match res {
    Err(ContractError::TokenMismatch {}) => assert!(true),
    _ => panic!("Must return token mismatch error"),
  }

  // execute order
  let execute_msg = Cw20HookMsg::ExecuteOrder { order_id: 3 };

  let info = mock_info("mir_addr", &[]);

  let receive_msg: Cw20ReceiveMsg = Cw20ReceiveMsg {
    sender: "buyer".to_string(),
    amount: Uint128::from(100000000u128),
    msg: to_binary(&execute_msg).unwrap()
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, ExecuteMsg::ReceiveToken(receive_msg)).unwrap();

  // royalty amounts
  let designer_royalty_asset = Asset {
    info: mir.clone(),
    amount: Uint128::from(2000000u128)
  };

  let pm_royalty_asset = Asset {
    info: mir.clone(),
    amount: Uint128::from(3000000u128)
  };

  // remain asset
  let remain_asset = Asset {
    info: mir.clone(),
    amount: Uint128::from(95000000u128)
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

  // check order removed
  let order = market.orders.may_load(&deps.storage, U64Key::new(3));

  assert_eq!(order, Ok(None));
}
