use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Decimal};

use cw_storage_plus::{Map, Item, U64Key};
use cw0::Expiration;

use crate::asset::{Asset, AssetInfo};

pub struct MarketContract<'a> {
  pub config: Item<'a, Config>,
  pub collections: Map<'a, String, CollectionInfo>,
  pub orders: Map<'a, U64Key, Order>,
  pub order_index: Item<'a, u64>
}

impl Default for MarketContract<'static> {
  fn default() -> Self {
    Self::new(
      "config",
      "collections",
      "auctions",
      "order_index"
    )
  }
}

impl<'a> MarketContract<'a> {
  fn new(
    config_key: &'a str,
    collections_key: &'a str,
    orders_key: &'a str,
    order_index: &'a str,
  ) -> Self {
    Self {
      config: Item::new(config_key),
      collections: Map::new(collections_key),
      orders: Map::new(orders_key),
      order_index: Item::new(order_index)
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
  pub owner: Addr,
  pub min_increase: Decimal,
  pub max_auction_duration_block: u64,
  pub max_auction_duration_second: u64,
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