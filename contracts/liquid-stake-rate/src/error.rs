use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid channelID: {channel_id}")]
    InvalidChannelID { channel_id: String },

    #[error("Invalid denom: {reason}")]
    InvalidDenom { reason: String },

    #[error("Channel ID is missing")]
    MissingTransferChannelID {},

    #[error("The denom for the redemption rate metric must not be an IBC denom, {denom} provided")]
    InvalidRedemptionRateDenom { denom: String },
}
