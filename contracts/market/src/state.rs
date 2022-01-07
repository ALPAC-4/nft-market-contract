use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal};

use cw_storage_plus::{Map, MultiIndex, Index, IndexedMap, IndexList, Item, U64Key};
use cw0::Expiration;

use crate::asset::{Asset, AssetInfo};

pub struct MarketContract<'a> {
  pub config: Item<'a, Config>,
  pub collections: Map<'a, String, CollectionInfo>,
  // change it to IndexedMap (with seller index)
  pub orders: IndexedMap<'a, U64Key, Order, OrderIndexes<'a>>,
  pub order_index: Item<'a, u64>
}

impl Default for MarketContract<'static> {
  fn default() -> Self {
    Self::new(
      "config",
      "collections",
      "auctions",
      "order_index",
      "seller_address",
    )
  }
}

impl<'a> MarketContract<'a> {
  fn new(
    config_key: &'a str,
    collections_key: &'a str,
    orders_key: &'a str,
    order_index_key: &'a str,
    seller_address_key: &'a str,
  ) -> Self {
    let order_indexes = OrderIndexes {
      seller_address: MultiIndex::new(seller_idx, orders_key, seller_address_key),
    };
    Self {
      config: Item::new(config_key),
      collections: Map::new(collections_key),
      orders: IndexedMap::new(orders_key, order_indexes),
      order_index: Item::new(order_index_key)
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
  pub owner: Addr,
  pub min_increase: Decimal,
  pub max_auction_duration_block: u64,
  pub max_auction_duration_second: u64,
  pub auction_cancel_fee_rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionInfo {
  pub nft_address: Addr,
  pub support_assets: Vec<AssetInfo>,
  pub royalties: Vec<Royalty>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Order {
  pub id: u64,
  pub seller_address: Addr,
  pub nft_address: Addr,
  pub token_id: String,
  pub price: Option<Asset>,
  pub auction_info: Option<AuctionInfo>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Royalty {
  pub address: Addr,
  pub royalty_rate: Decimal
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionInfo {
  pub highest_bid: Asset,
  // if None, no bid yet.
  pub bidder: Option<Addr>,
  pub expiration: Expiration
}

pub struct OrderIndexes<'a> {
  pub seller_address: MultiIndex<'a, (Addr, Vec<u8>), Order>
}

pub fn seller_idx(d: &Order, k: Vec<u8>) -> (Addr, Vec<u8>) {
  (d.seller_address.clone(), k)
}


impl<'a> IndexList<Order> for OrderIndexes<'a> {
  fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Order>> + '_> {
    let v: Vec<&dyn Index<Order>> = vec![&self.seller_address];
    Box::new(v.into_iter())
  }
}