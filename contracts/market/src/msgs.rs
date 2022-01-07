use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw721::Cw721ReceiveMsg;
use cw20::Cw20ReceiveMsg;
use cw0::Expiration;
use cosmwasm_std::{Addr, Decimal};

use crate::asset::{Asset, AssetInfo};
use crate::state::Royalty;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InstantiateMsg {
  pub owner: String,
  // min bid price increase percent
  pub min_increase: Decimal,
  pub max_auction_duration_block: u64,
  pub max_auction_duration_second: u64,
  pub auction_cancel_fee_rate: Decimal,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
  ReceiveNft(Cw721ReceiveMsg),

  ReceiveToken(Cw20ReceiveMsg),

  UpdateConfig {
    owner: Option<String>,
    min_increase: Option<Decimal>,
    max_auction_duration_block: Option<u64>,
    max_auction_duration_second: Option<u64>,
    auction_cancel_fee_rate: Option<Decimal>,
  },

  AddCollection {
    nft_address: String,
    support_assets: Vec<AssetInfo>,
    royalties: Vec<Royalty>,
  },

  // if you want to delist/remove the collection, set support_asset = vec![]
  // The reason that I didn't put remove_collection function is to avoid error from the order that already made.
  UpdateCollection {
    nft_address: String,
    support_assets: Option<Vec<AssetInfo>>,
    royalties: Option<Vec<Royalty>>,
  },

  // buy nft at fixed price.
  ExecuteOrder {
    order_id: u64
  },

  // execute expired auction.
  ExecuteAuction {
    order_id: u64
  },

  CancelOrder {
    order_id: u64
  },

  Bid {
    order_id: u64,
    bid_price: Asset
  },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
  ExecuteOrder {
    order_id: u64
  },

  Bid {
    order_id: u64,
  },

  CancelOrder {
    order_id: u64,
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721HookMsg {
  MakeFixedPriceOrder {
    price: Asset
  },

  MakeAuctionOrder {
    start_price: Asset,
    expiration: Expiration,
    fixed_price: Option<Asset>,
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
  Config {},

  CollectionInfo {
    nft_address: String
  },

  CollectionInfos {
    start_after: Option<String>,
    limit: Option<u32>
  },

  Order {
    order_id: u64
  },

  Orders {
    seller_address: Option<Addr>,
    start_after: Option<u64>,
    limit: Option<u32>
  },

  CancelFee {
    order_id: u64
  }
}