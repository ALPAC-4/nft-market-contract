use cosmwasm_std::{StdError, OverflowError, Uint128};
use thiserror::Error;
use crate::asset::Asset;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("{0}")]
  OverflowError(#[from] OverflowError),

  #[error("Collection is already exist")]
  CollectionExist {},

  #[error("Unauthorized")]
  Unauthorized {},

  #[error("Asset is not supported")]
  Unsupport {},

  #[error("Sum of the royalty rate is higher than 100%")]
  InvalidRoyaltyRate {},

  #[error("Auction cancel fee rate is higher than 100%")]
  InvalidFeeRate {},

  #[error("The order doesn't have fixed price option")]
  NoFixedPrice {},

  #[error("The order is not auction")]
  NotAuction {},

  #[error("Token type or balance mismatch with price")]
  TokenMismatch {},

  #[error("Asset type mismatch")]
  AssetInfoMismatch {},

  #[error("Given expiration is already expired or order is already expired")]
  Expired {},

  #[error("Exceed max auction duration")]
  MaxDuration {},

  #[error("Expiration never is not allowed")]
  Never {},

  #[error("Auction is not expired")]
  NotExpired {},

  #[error("Cancel fee mismatch you must send {} ", fee_asset)]
  CancelFeeMismatch { fee_asset: Asset },

  #[error("You must bid higher or equal to {} (min bid amount)", min_bid_amount)]
  MinPrice { min_bid_amount: Uint128 },
}