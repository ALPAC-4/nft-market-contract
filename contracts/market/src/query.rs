use cosmwasm_std::{to_binary, Addr, Binary, Deps, StdResult, Order::Ascending as Ascending, Uint128};
use cw_storage_plus::{Bound, U64Key};
use std::marker::PhantomData;

use crate::state::{MarketContract, CollectionInfo, Order};
use crate::msgs::QueryMsg;
use crate::asset::Asset;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

impl<'a> MarketContract<'a> {
  fn orders(&self, deps: Deps, seller_address: Option<Addr>, start_after: Option<u64>, limit: Option<u32>) -> StdResult<Vec<Order>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = if let Some(start_after) = start_after {
      Some(Bound::exclusive(U64Key::new(start_after)))
    } else {
      None
    };

    let orders: Vec<Order> = if let Some(seller_address) = seller_address {
      let pks: Vec<_> = self
      .orders
      .idx
      .seller_address
      .prefix(seller_address)
      .keys(deps.storage, start, None, Ascending)
      .take(limit)
      .collect();

      pks.iter().map(|v|  {
        let restruct_int_key = U64Key {
          wrapped: v.clone(),
          data: PhantomData
        };
        let order = self.orders.load(deps.storage, restruct_int_key).unwrap();
        return order
      }).collect()
    } else {
      self.orders
      .range(deps.storage, start, None, Ascending)
      .take(limit)
      .map(|item| {
        let(_, v) = item.unwrap();
        v
      })
      .collect()
    };

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

  fn cancel_fee(&self, deps: Deps, order_id: u64) -> StdResult<Asset> {
    let order = self.orders.load(deps.storage, U64Key::new(order_id))?;

    let fee: Asset;

    let auction_info = order.auction_info;

    // if it is auction
    if let Some(auction_info) = auction_info {
      let config = self.config.load(deps.storage)?;

      fee = Asset {
        info: auction_info.highest_bid.info,
        amount: auction_info.highest_bid.amount * config.auction_cancel_fee_rate
      }
    // if it is not auction return 0 amount asset
    } else {
      fee = Asset {
        info: order.price.unwrap().info,
        amount: Uint128::zero()
      }
    }

    Ok(fee)
  }
}

impl<'a> MarketContract<'a> {
  pub fn query(&self, deps: Deps, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
      QueryMsg::Config {} => to_binary(&self.config.load(deps.storage)?),
      QueryMsg::Order { order_id } => to_binary(&self.orders.load(deps.storage, U64Key::new(order_id))?),
      QueryMsg::Orders { seller_address, start_after, limit } 
        => to_binary(&self.orders(deps, seller_address, start_after, limit)?),
      QueryMsg::CollectionInfo { nft_address } 
        => to_binary(&self.collections.load(deps.storage, nft_address)?),
      QueryMsg::CollectionInfos { start_after, limit }
        => to_binary(&self.collection_infos(deps, start_after, limit)?),
      QueryMsg::CancelFee { order_id }
        => to_binary(&self.cancel_fee(deps, order_id)?) 
    }
  }
}