use cosmwasm_std::{from_binary, to_binary, Addr, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, StdError, StdResult, Response, WasmMsg, Uint128};
use cw_storage_plus::U64Key;
use cw721::{Cw721ReceiveMsg, Cw721ExecuteMsg};
use cw20::Cw20ReceiveMsg;
use cw0::Expiration;

use crate::state::{AuctionInfo, Config, CollectionInfo, MarketContract, Order, Royalty};
use crate::msgs::{InstantiateMsg, ExecuteMsg, Cw20HookMsg, Cw721HookMsg};
use crate::error::ContractError;
use crate::asset::{Asset, AssetInfo};

impl<'a> MarketContract<'a> {
  pub fn instantiate(
    &self,
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg
  ) -> StdResult<Response> {
    
    if msg.auction_cancel_fee_rate > Decimal::one() {
      return Err(StdError::generic_err("Cancel fee rate can't exceed 1"))
    }

    let config = Config {
      owner: deps.api.addr_validate(msg.owner.as_str())?,
      min_increase: msg.min_increase,
      max_auction_duration_block: msg.max_auction_duration_block,
      max_auction_duration_second: msg.max_auction_duration_second,
      auction_cancel_fee_rate: msg.auction_cancel_fee_rate,
    };

    self.config.save(deps.storage, &config)?;

    let order_index = 1u64;
    self.order_index.save(deps.storage, &order_index)?;
    Ok(Response::new())
  }

  pub fn execute(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
  ) -> Result<Response, ContractError> {
    match msg {
      ExecuteMsg::ReceiveNft(msg) => self.receive_nft(deps, env, info, msg),
      ExecuteMsg::ReceiveToken(msg) => self.receive_token(deps, env, info, msg),
      ExecuteMsg::UpdateConfig { owner, min_increase, max_auction_duration_block, max_auction_duration_second, auction_cancel_fee_rate } 
        => self.update_config(deps, env, info, owner, min_increase, max_auction_duration_block, max_auction_duration_second, auction_cancel_fee_rate),
      ExecuteMsg::ExecuteOrder { order_id } => self.execute_order(deps, env, info.clone(), info.sender, order_id, None),
      ExecuteMsg::CancelOrder { order_id } => self.cancel_order(deps, env, info.clone(), info.sender, order_id, None),
      ExecuteMsg::AddCollection { nft_address, support_assets, royalties } 
        => self.add_collection(deps, env, info, nft_address, support_assets, royalties),
      ExecuteMsg::UpdateCollection { nft_address, support_assets, royalties } 
        => self.update_collection(deps, env, info, nft_address, support_assets, royalties),
      ExecuteMsg::Bid { order_id, bid_price } => self.bid(deps, env, info.clone(), info.sender, order_id, bid_price),
      ExecuteMsg::ExecuteAuction { order_id } => self.execute_auction(deps, env, info, order_id)
    }
  }
}

impl<'a> MarketContract <'a> {
  pub fn receive_nft(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw721ReceiveMsg
  ) -> Result<Response, ContractError> {
    let contract_addr = info.sender.clone();

    let sender = deps.api.addr_validate(&msg.sender)?;

    let cw721_msg = from_binary::<Cw721HookMsg>(&msg.msg);

    match cw721_msg {
      Ok(Cw721HookMsg::MakeFixedPriceOrder {
        price
      }) => {
        let collection_info = self.collections.load(deps.storage, contract_addr.to_string())?;

        let support_assets = collection_info.support_assets;

        let mut support = false;
        
        for asset in support_assets.iter() {
          if asset == &price.info {
            support = true;
            break;
          }
        }

        if !support {
          return Err(ContractError::Unsupport {});
        }

        let id = self.order_index.load(deps.storage)?;

        let order = Order {
          id,
          seller_address: sender.clone(),
          nft_address: contract_addr.clone(),
          token_id: msg.token_id.clone(),
          price: Some(price.clone()),
          auction_info: None
        };

        let key = U64Key::new(id);

        self.orders.save(deps.storage, key, &order)?;
        self.order_index.save(deps.storage, &(id + 1))?;

        Ok(Response::new()
          .add_attribute("action", "make_fixed_price_order")
          .add_attribute("sender", sender)
          .add_attribute("order_id", id.to_string())
          .add_attribute("nft_address", contract_addr)
          .add_attribute("token_id", msg.token_id)
          .add_attribute("price", format!("{}", price))
        )
      }

      Ok(Cw721HookMsg::MakeAuctionOrder {
        start_price, fixed_price, expiration
      }) => {
        let collection_info = self.collections.load(deps.storage, contract_addr.to_string())?;

        let support_assets = collection_info.support_assets;

        let mut support = false;
        
        if let Some(fixed_price) = fixed_price.clone() {
          if fixed_price.info != start_price.info {
            return Err(ContractError::AssetInfoMismatch {})
          }
        }

        for asset in support_assets.iter() {
          if asset == &start_price.info {
            support = true;
            break;
          }
        }

        if !support {
          return Err(ContractError::Unsupport {});
        }

        let id = self.order_index.load(deps.storage)?;
        
        let config = self.config.load(deps.storage)?;

        // check expiration
        match expiration {
          Expiration::Never {} => {
            return Err(ContractError::Never {})
          }
          Expiration::AtHeight(height) => {
            if expiration.is_expired(&env.block) {
              return Err(ContractError::Expired {})
            }

            if (height - env.block.height) > config.max_auction_duration_block {
              return Err(ContractError::MaxDuration {})
            }
          }
          Expiration::AtTime(timestamp) => {
            if expiration.is_expired(&env.block) {
              return Err(ContractError::Expired {})
            }

            if (timestamp.seconds() - env.block.time.seconds()) > config.max_auction_duration_second {
              return Err(ContractError::MaxDuration {})
            }
          }
        }

        let auction_info = AuctionInfo {
          highest_bid: start_price.clone(),
          bidder: None,
          expiration
        };

        let order = Order {
          id,
          seller_address: sender.clone(),
          nft_address: contract_addr.clone(),
          token_id: msg.token_id.clone(),
          price: fixed_price.clone(),
          auction_info: Some(auction_info)
        };

        let key = U64Key::new(id);

        self.orders.save(deps.storage, key, &order)?;
        self.order_index.save(deps.storage, &(id + 1))?;

        Ok(Response::new()
          .add_attribute("action", "make_auction_order")
          .add_attribute("sender", sender)
          .add_attribute("order_id", id.to_string())
          .add_attribute("nft_address", contract_addr)
          .add_attribute("token_id", msg.token_id)
          .add_attribute("fixed_price", if let Some(fixed_price) = fixed_price {
            format!("{}", fixed_price)
          } else {
            "null".to_string()
          })
          .add_attribute("start_price", format!("{}", start_price))
          .add_attribute("expiration", format!("{}", expiration))
        )
      }

      Err(err) => Err(ContractError::Std(err)),
    }
  }

  pub fn receive_token(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg
  ) -> Result<Response, ContractError> {
    let contract_addr = info.sender.clone();

    let sender = deps.api.addr_validate(&msg.sender)?;

    let cw20_msg = from_binary::<Cw20HookMsg>(&msg.msg)?;

    let asset = Asset {
      info: AssetInfo::Token { contract_addr: contract_addr.to_string() },
      amount: msg.amount
    };

    match cw20_msg {
      Cw20HookMsg::ExecuteOrder { order_id } 
        => self.execute_order(deps, env, info, sender, order_id, Some(asset)),
      Cw20HookMsg::Bid { order_id } 
        => self.bid(deps, env, info, sender, order_id, asset),
      Cw20HookMsg::CancelOrder { order_id } 
        => self.cancel_order(deps, env, info, sender, order_id, Some(asset))
    }
  }

  pub fn update_config(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    min_increase: Option<Decimal>,
    max_auction_duration_block: Option<u64>,
    max_auction_duration_second: Option<u64>,
    auction_cancel_fee_rate: Option<Decimal>
  ) -> Result<Response, ContractError> {
    let mut config: Config = self.config.load(deps.storage)?;
    
    if info.sender != config.owner {
      return Err(ContractError::Unauthorized {})
    }

    if let Some(owner) = owner {
      config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(min_increase) = min_increase {
      config.min_increase = min_increase;
    }

    if let Some(max_auction_duration_block) = max_auction_duration_block {
      config.max_auction_duration_block = max_auction_duration_block;
    }

    if let Some(max_auction_duration_second) = max_auction_duration_second {
      config.max_auction_duration_second = max_auction_duration_second;
    }

    if let Some(auction_cancel_fee_rate) = auction_cancel_fee_rate {
      if auction_cancel_fee_rate > Decimal::one() {
        return Err(ContractError::InvalidFeeRate {})
      }

      config.auction_cancel_fee_rate = auction_cancel_fee_rate;
    }

    self.config.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
  }

  pub fn execute_order(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    sender: Addr,
    order_id: u64,
    // for cw20
    asset: Option<Asset>,
  ) -> Result<Response, ContractError> {
    let key = U64Key::new(order_id);
    let order = self.orders.load(deps.storage, key)?;
    let price = order.clone().price;

    if let Some(price) = price {
      if let Some(asset) = asset {
        if price != asset {
          return Err(ContractError::TokenMismatch {})
        }
      } else {
        // native sent balance check
        price.assert_sent_native_token_balance(&info)?;
      }

      let (messages, remain_amount) = self.execute_order_(deps, order.clone(), sender.clone(), price.clone(), false)?;

      Ok(Response::new().add_messages(messages)
        .add_attribute("action", "execute_order")
        .add_attribute("sender", sender.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("buyer", sender.to_string())
        .add_attribute("seller", order.seller_address)
        .add_attribute("price", format!("{}", price))
        .add_attribute("royalty_amount", price.amount - remain_amount)
      )
    } else {
      return Err(ContractError::NoFixedPrice {})
    }
  }

  pub fn execute_auction(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    order_id: u64
  ) -> Result<Response, ContractError> {
    let key = U64Key::new(order_id);
    let order = self.orders.load(deps.storage, key)?;

    // check is auction
    let auction_info = order.clone().auction_info;
    if let Some(auction_info) = auction_info {
      // check expiration
      if !auction_info.expiration.is_expired(&env.block) {
        return Err(ContractError::NotExpired {})
      }

      let bidder = auction_info.bidder.clone();
    
      let (messages, remain_amount, buyer): (Vec<CosmosMsg>, Uint128, String);
      
      if let Some(bidder) = bidder {
        let (messages_, remain_amount_) = self.execute_order_(deps, order.clone(), bidder.clone(), auction_info.highest_bid.clone(), true)?;
        messages = messages_;
        remain_amount = remain_amount_;
        buyer = bidder.to_string()
        // no bidder
      } else {
        // return nft to seller
        messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
          contract_addr: order.nft_address.to_string(),
          msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: order.seller_address.to_string(), 
            token_id: order.token_id
          })?,
          funds: vec![]
        })];

        remain_amount = auction_info.highest_bid.amount;
        buyer = "null".to_string();

        // remove order
        self.orders.remove(deps.storage, U64Key::new(order.id))?;
      }

      Ok(Response::new().add_messages(messages)
        .add_attribute("action", "execute_auction")
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("buyer", buyer)
        .add_attribute("seller", order.seller_address)
        .add_attribute("price", format!("{}", auction_info.highest_bid))
        .add_attribute("royalty_amount", auction_info.highest_bid.amount - remain_amount)
      )
    } else {
      return Err(ContractError::NotAuction {})
    }
  }

  pub fn cancel_order(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    order_id: u64,
    cancel_fee: Option<Asset>
  ) -> Result<Response, ContractError> {
    let key = U64Key::new(order_id);

    let order = self.orders.load(deps.storage, key.clone())?;

    // only seller can execute
    if order.seller_address != sender.clone() {
      return Err(ContractError::Unauthorized {})
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    // if auction refund latest bid first
    let auction_info = order.clone().auction_info;
    if let Some(auction_info) = auction_info {
      // can not cancel expired auction
      if auction_info.expiration.is_expired(&env.block) {
        return Err(ContractError::Expired {})
      }
      messages = self.refund_bid(deps.as_ref(), info.clone(), order.clone(), cancel_fee)?;
    }

    // return nft to seller
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: order.nft_address.to_string(),
      msg: to_binary(&Cw721ExecuteMsg::TransferNft {
        recipient: order.seller_address.to_string(),
        token_id: order.token_id
      })?,
      funds: vec![]
    }));

    // remove order
    self.orders.remove(deps.storage, key)?;

    Ok(Response::new().add_messages(messages)
      .add_attribute("action", "cancel_order")
      .add_attribute("sender", sender)
      .add_attribute("order_id", order_id.to_string())
    )
  }

  pub fn add_collection(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    nft_address: String,
    support_assets: Vec<AssetInfo>,
    royalties: Vec<Royalty>,
  ) -> Result<Response, ContractError> {
    // only owner can execute this
    let config = self.config.load(deps.storage)?;

    if config.owner != info.sender {
      return Err(ContractError::Unauthorized {})
    }

    if let Ok(Some(_)) = self.collections.may_load(deps.storage, nft_address.clone()) {
      return Err(ContractError::CollectionExist {})
    }

    let mut sum_rotalty_rate = Decimal::zero();

    for royalty in royalties.iter() {
      sum_rotalty_rate = sum_rotalty_rate + royalty.royalty_rate;
    }

    if sum_rotalty_rate > Decimal::one() {
      return Err(ContractError::InvalidRoyaltyRate {})
    }

    if sum_rotalty_rate > Decimal::one() {
      return Err(ContractError::InvalidRoyaltyRate {})
    }

    let collection_info = CollectionInfo {
      nft_address: deps.api.addr_validate(&nft_address)?,
      royalties,
      support_assets,
    };

    self.collections.save(deps.storage, nft_address.clone(), &collection_info)?;

    Ok(Response::new()
      .add_attribute("action", "add_collection")
      .add_attribute("sender", info.sender)
      .add_attribute("nft_address", nft_address)
    )
  }

  pub fn update_collection(
    &self,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    nft_address: String,
    support_assets: Option<Vec<AssetInfo>>,
    royalties: Option<Vec<Royalty>>,
  ) -> Result<Response, ContractError> {
    // only owner can execute this
    let config = self.config.load(deps.storage)?;

    if config.owner != info.sender {
      return Err(ContractError::Unauthorized {})
    }

    let mut collection = self.collections.load(deps.storage, nft_address.clone())?;

    if let Some(support_assets) = support_assets {
      collection.support_assets = support_assets;
    }

    if let Some(royalties) = royalties {
      let mut sum_rotalty_rate = Decimal::zero();

      for royalty in royalties.iter() {
        sum_rotalty_rate = sum_rotalty_rate + royalty.royalty_rate;
      }

      if sum_rotalty_rate > Decimal::one() {
        return Err(ContractError::InvalidRoyaltyRate {})
      }

      collection.royalties = royalties;
    }

    self.collections.save(deps.storage, nft_address.clone(), &collection)?;

    Ok(Response::new()
      .add_attribute("action", "update_collection")
      .add_attribute("sender", info.sender)
      .add_attribute("nft_address", nft_address)
    )
  }

  pub fn bid(
    &self,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: Addr,
    order_id: u64,
    bid_price: Asset,
  ) -> Result<Response, ContractError> {
    let mut order = self.orders.load(deps.storage, U64Key::new(order_id))?;

    let auction_info = order.auction_info;

    if let Some(auction_info) = auction_info {
      // native sent balance check
      bid_price.assert_sent_native_token_balance(&info)?;

      let config = self.config.load(deps.storage)?;

      if bid_price.info != auction_info.highest_bid.info {
        return Err(ContractError::AssetInfoMismatch {})
      }

      if auction_info.expiration.is_expired(&env.block) {
        return Err(ContractError::Expired {})
      }

      let min_bid_amount = auction_info.highest_bid.amount * (config.min_increase + Decimal::one());

      if min_bid_amount > bid_price.amount {
        return Err(ContractError::MinPrice { min_bid_amount })
      }

      let mut messages: Vec<CosmosMsg> = vec![];

      let bidder = auction_info.clone().bidder;

      // refund former bid
      if let Some(bidder) = bidder {
        messages.push(auction_info.clone().highest_bid.into_msg(&deps.querier, bidder)?);
      }

      // update highest bid
      let mut auction_info = auction_info;
      auction_info.highest_bid = bid_price.clone();
      auction_info.bidder = Some(sender.clone());

      order.auction_info = Some(auction_info);

      self.orders.save(deps.storage, U64Key::new(order_id), &order)?;

      Ok(Response::new().add_messages(messages)
      .add_attribute("action", "bid")
      .add_attribute("sender", sender.to_string())
      .add_attribute("order_id", order_id.to_string())
      .add_attribute("bidder", sender.to_string())
      .add_attribute("bid_price", format!("{}", bid_price))
    )
    } else {
      return Err(ContractError::NotAuction {})
    }
  }
}

// helper
impl<'a> MarketContract <'a> {
  fn execute_order_(
    &self,
    deps: DepsMut,
    order: Order,
    buyer: Addr,
    price: Asset,
    is_auction_execute: bool,
  ) -> Result<(Vec<CosmosMsg>, Uint128), ContractError> {
    let mut messages: Vec<CosmosMsg> = vec![];

    // refund asset to last bidder
    let auction_info = order.auction_info;

    if let Some(auction_info) = auction_info {
      if !is_auction_execute {
        let bid_price = auction_info.highest_bid;
        let bidder = auction_info.bidder;
        
        if let Some(bidder) = bidder {
          messages.push(bid_price.into_msg(&deps.querier, bidder)?)
        }
      }
    }

    // transfer nft to buyer
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
      contract_addr: order.nft_address.to_string(),
      msg: to_binary(&Cw721ExecuteMsg::TransferNft {
        recipient: buyer.to_string(), 
        token_id: order.token_id
      })?,
      funds: vec![]
    }));

    // get royalty
    let collection_info = self.collections.load(deps.storage, order.nft_address.to_string())?;

    let mut remain_amount = price.amount;

    // transfer royalty
    for royalty in collection_info.royalties.iter() {
      messages.push(
        (Asset {
          info: price.info.clone(),
          amount: price.amount * royalty.royalty_rate
        }).into_msg(&deps.querier, royalty.address.clone())?
      );

      remain_amount = remain_amount.checked_sub(price.amount * royalty.royalty_rate)?;
    }

    // transfer remain amount to seller
    messages.push(
      (Asset {
        info: price.clone().info,
        amount: remain_amount
      }).into_msg(&deps.querier, order.seller_address.clone())?
    );

    // remove order
    self.orders.remove(deps.storage, U64Key::new(order.id))?;

    Ok((messages, remain_amount))
  }

  fn refund_bid(
    &self,
    deps: Deps,
    info: MessageInfo,
    order: Order,
    cancel_fee: Option<Asset>
  ) -> Result<Vec<CosmosMsg>, ContractError> {
    let auction_info = order.auction_info.unwrap();
    let bidder = auction_info.bidder;
    let config = self.config.load(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    // if bid exist refund bid amount + cancel fee
    if let Some(bidder) = bidder {
      let bid_price = auction_info.highest_bid;
      let cancel_fee_amount = bid_price.amount * config.auction_cancel_fee_rate;

      let fee_asset = Asset {
        info: bid_price.info.clone(),
        amount: cancel_fee_amount
      };
      
      let refund_asset = Asset {
        info: bid_price.info,
        amount: bid_price.amount + cancel_fee_amount
      };

      // check balance
      match fee_asset.info.clone() {
        AssetInfo::NativeToken { denom } => {
          match info.funds.iter().find(|x| x.denom == denom) {
            Some(coin) => {
              if fee_asset.amount != coin.amount {
                return Err(ContractError::CancelFeeMismatch{ fee_asset })
              }
            }
            None => {
              if !fee_asset.amount.is_zero() {
                return Err(ContractError::CancelFeeMismatch{ fee_asset })
              }
            }
          }
        }
        AssetInfo::Token { .. } => {
          if let Some(cancel_fee) = cancel_fee {
            if cancel_fee != fee_asset {
              return Err(ContractError::CancelFeeMismatch{ fee_asset })
            }
          } else {
            if !cancel_fee_amount.is_zero() {
              return Err(ContractError::CancelFeeMismatch{ fee_asset })
            }
          }
        }
      }

      messages.push(refund_asset.into_msg(&deps.querier, bidder)?);
    }

    Ok(messages)
  }
}
