use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_binary, Decimal};

use crate::{
  state::{MarketContract, Config},
  msgs::{InstantiateMsg, QueryMsg},
  testing::mock_querier::mock_dependencies
};

#[test]
fn instantiate_test() {
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

  let config: Config = from_binary(
    &market.query(deps.as_ref(), QueryMsg::Config {}).unwrap()
  ).unwrap();

  assert_eq!("owner".to_string(), config.owner);
  assert_eq!(Decimal::from_ratio(10u128, 100u128), config.min_increase);
  assert_eq!(100, config.max_auction_duration_block);
  assert_eq!(1000, config.max_auction_duration_second);

  let order_index = market.order_index.load(&deps.storage).unwrap();

  assert_eq!(1, order_index);
}