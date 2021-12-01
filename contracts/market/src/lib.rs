mod state;
mod asset;
mod msgs;
mod execute;
mod error;
mod query;

#[cfg(test)]
mod testing;

use crate::msgs::{InstantiateMsg, ExecuteMsg, QueryMsg};
use crate::state::MarketContract;
use crate::error::ContractError;

#[cfg(not(feature = "library"))]
pub mod entry {
  use super::*;

  use cosmwasm_std::entry_point;
  use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

  #[entry_point]
  pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
  ) -> StdResult<Response> {
    let tract = MarketContract::default();
    tract.instantiate(deps, env, info, msg)
  }

  #[entry_point]
  pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
  ) -> Result<Response, ContractError> {
    let tract = MarketContract::default();
    tract.execute(deps, env, info, msg)
  }

  #[entry_point]
  pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let tract = MarketContract::default();
      tract.query(deps, msg)
  }
}
