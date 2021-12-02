use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{Addr, Decimal};

use crate::{
  state::{MarketContract, Royalty},
  msgs::{InstantiateMsg, ExecuteMsg},
  error::ContractError,
  asset::AssetInfo,
  testing::mock_querier::mock_dependencies
};

#[test]
fn update_config_test() {
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

  // update config, owner change
  let update_config_msg = ExecuteMsg::UpdateConfig {
    owner: Some("next_owner".to_string()),
    min_increase: None,
    max_auction_duration_block: None,
    max_auction_duration_second: None,
    auction_cancel_fee_rate: None,
  };

  let _res = market.execute(deps.as_mut(), mock_env(), info.clone(), update_config_msg).unwrap();

  let config = market.config.load(&deps.storage).unwrap();

  // check onwer changed
  assert_eq!("next_owner".to_string(), config.owner);

  // former owner try to update config
  
  let update_config_msg = ExecuteMsg::UpdateConfig {
    owner: Some("owner".to_string()),
    min_increase: None,
    max_auction_duration_block: None,
    max_auction_duration_second: None,
    auction_cancel_fee_rate: None,
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, update_config_msg);

  match res {
    Err(ContractError::Unauthorized {}) => assert!(true),
    _ => panic!("Must return unauthorized error"),
  }

  // update config. other options
  let update_config_msg = ExecuteMsg::UpdateConfig {
    owner: None,
    min_increase: Some(Decimal::from_ratio(5u128, 100u128)),
    max_auction_duration_block: Some(123),
    max_auction_duration_second: Some(1234),
    auction_cancel_fee_rate: Some(Decimal::from_ratio(3u128, 1000u128)),
  };

  let info = mock_info("next_owner", &[]);

  let _res = market.execute(deps.as_mut(), mock_env(), info.clone(), update_config_msg).unwrap();

  let config = market.config.load(&deps.storage).unwrap();

  assert_eq!(Decimal::from_ratio(5u128, 100u128), config.min_increase);
  assert_eq!(123, config.max_auction_duration_block);
  assert_eq!(1234, config.max_auction_duration_second);
}

#[test]
fn add_and_update_collection_test() {
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

  let _res = market.instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

  // some assetinfos
  let uusd: AssetInfo = AssetInfo::NativeToken { denom: "uusd".to_string()};
  let mir: AssetInfo = AssetInfo::Token { contract_addr: "mir_addr".to_string()};

  // some royalties
  let normal_user_royalty: Royalty = Royalty {
    address: Addr::unchecked("normal_user"),
    royalty_rate: Decimal::from_ratio(1u128, 1u128)
  };

  let nft_designer_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_designer"),
    royalty_rate: Decimal::from_ratio(2u128, 100u128)
  };

  let nft_pm_royalty: Royalty = Royalty {
    address: Addr::unchecked("nft_pm"),
    royalty_rate: Decimal::from_ratio(3u128, 100u128)
  };

  let nft_designer_royalty_invalid: Royalty = Royalty {
    address: Addr::unchecked("nft_designer"),
    royalty_rate: Decimal::from_ratio(60u128, 100u128)
  };

  let nft_pm_royalty_invalid: Royalty = Royalty {
    address: Addr::unchecked("nft_pm"),
    royalty_rate: Decimal::from_ratio(80u128, 100u128)
  };

  // normal user(not the owner) try to add collection 
  let info = mock_info("normal_user", &[]);
  let add_collection_msg = ExecuteMsg::AddCollection {
    nft_address: "spaceship".to_string(),
    support_assets: vec![uusd.clone()],
    royalties: vec![normal_user_royalty],
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, add_collection_msg);

  match res {
    Err(ContractError::Unauthorized {}) => assert!(true),
    _ => panic!("Must return unauthorized error"),
  }

  // try to add collection with invalid royalty rate
  let info = mock_info("owner", &[]);
  let add_collection_msg = ExecuteMsg::AddCollection {
    nft_address: "spaceship".to_string(),
    support_assets: vec![uusd.clone(), mir.clone()],
    royalties: vec![nft_designer_royalty_invalid.clone(), nft_pm_royalty_invalid.clone()],
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, add_collection_msg);

  match res {
    Err(ContractError::InvalidRoyaltyRate {}) => assert!(true),
    _ => panic!("Must return invalid royalty rate error"),
  }

  // add collection
  let info = mock_info("owner", &[]);
  let add_collection_msg = ExecuteMsg::AddCollection {
    nft_address: "spaceship".to_string(),
    support_assets: vec![uusd.clone(), mir.clone()],
    royalties: vec![nft_designer_royalty.clone(), nft_pm_royalty.clone()],
  };

  let _res = market.execute(deps.as_mut(), mock_env(), info, add_collection_msg).unwrap();

  let collection = market.collections.load(&deps.storage, "spaceship".to_string()).unwrap();

  assert_eq!(Addr::unchecked("spaceship"), collection.nft_address);
  assert_eq!(vec![uusd.clone(), mir.clone()], collection.support_assets);
  assert_eq!(vec![nft_designer_royalty.clone(), nft_pm_royalty.clone()], collection.royalties);

  // try to add collection that already exist
  // dev note: do i have to remove this limit and remove update collection function?
  let info = mock_info("owner", &[]);
  let add_collection_msg = ExecuteMsg::AddCollection {
    nft_address: "spaceship".to_string(),
    support_assets: vec![uusd.clone(), mir.clone()],
    royalties: vec![nft_designer_royalty.clone(), nft_pm_royalty.clone()],
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, add_collection_msg);

  match res {
    Err(ContractError::CollectionExist {}) => assert!(true),
    _ => panic!("Must return collection exist error"),
  }

  // who is not the owner try to update collection 
  let info = mock_info("nft_designer", &[]);
  let update_collection_msg = ExecuteMsg::UpdateCollection {
    nft_address: "spaceship".to_string(),
    support_assets: Some(vec![uusd.clone(), mir.clone()]),
    royalties: Some(vec![nft_designer_royalty.clone()]),
  };

  let res = market.execute(deps.as_mut(), mock_env(), info, update_collection_msg);

  match res {
    Err(ContractError::Unauthorized {}) => assert!(true),
    _ => panic!("Must return unauthorized error"),
  }

  // try to update collection with invalid royalty rate
  let info = mock_info("owner", &[]);
  let update_collection_msg = ExecuteMsg::UpdateCollection {
    nft_address: "spaceship".to_string(),
    support_assets: None,
    royalties: Some(vec![nft_designer_royalty_invalid.clone(), nft_pm_royalty_invalid.clone()]),
  };

  let res = market.execute(deps.as_mut(), mock_env(), info.clone(), update_collection_msg);

  match res {
    Err(ContractError::InvalidRoyaltyRate {}) => assert!(true),
    _ => panic!("Must return invalid royalty rate error"),
  }

  // update collection
  let update_collection_msg = ExecuteMsg::UpdateCollection {
    nft_address: "spaceship".to_string(),
    support_assets: Some(vec![uusd.clone()]),
    royalties: Some(vec![nft_designer_royalty.clone()]),
  };

  let _res = market.execute(deps.as_mut(), mock_env(), info, update_collection_msg);

  let collection = market.collections.load(&deps.storage, "spaceship".to_string()).unwrap();

  assert_eq!(Addr::unchecked("spaceship"), collection.nft_address);
  assert_eq!(vec![uusd], collection.support_assets);
  assert_eq!(vec![nft_designer_royalty], collection.royalties);
}
