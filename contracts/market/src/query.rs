use cosmwasm_std::{to_binary, Binary, Deps, StdResult, Order::Ascending as Ascending};
use cw_storage_plus::{Bound, U64Key};

use crate::state::{MarketContract, CollectionInfo, Order};
use crate::msgs::QueryMsg;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

impl<'a> MarketContract<'a> {
  fn orders(&self, deps: Deps, start_after: Option<u64>, limit: Option<u32>) -> StdResult<Vec<Order>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = if let Some(start_after) = start_after {
      Some(Bound::exclusive(U64Key::new(start_after)))
    } else {
      None
    };

    let orders: Vec<Order> = self.orders
      .range(deps.storage, start, None, Ascending)
      .take(limit)
      .map(|item| {
        let(_, v) = item.unwrap();
        v
      })
      .collect();

    Ok(orders)
  }

  fn collection_infos(&self, deps: Deps, start_after: Option<String>, limit: Option<u32>) -> StdResult<Vec<CollectionInfo>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = if let Some(start_after) = start_after {
      Some(Bound::exclusive(start_after))
    } else {
      None
    };

    let collection_infos: Vec<CollectionInfo> = self.collections
      .range(deps.storage, start, None, Ascending)
      .take(limit)
      .map(|item| {
        let(_, v) = item.unwrap();
        v
      })
      .collect();

    Ok(collection_infos)
  }
}

impl<'a> MarketContract<'a> {
  pub fn query(&self, deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
      QueryMsg::Config {} => to_binary(&self.config.load(deps.storage)?),
      QueryMsg::Order { order_id } => to_binary(&self.orders.load(deps.storage, U64Key::new(order_id))?),
      QueryMsg::Orders { start_after, limit } => to_binary(&self.orders(deps, start_after, limit)?),
      QueryMsg::CollectionInfo { nft_address } 
        => to_binary(&self.collections.load(deps.storage, nft_address)?),
      QueryMsg::CollectionInfos { start_after, limit }
       => to_binary(&self.collection_infos(deps, start_after, limit)?)
    }
  }
}