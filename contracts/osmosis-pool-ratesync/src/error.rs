use cosmwasm_std::StdError;
use ratesync::lsr_error::ContractError as LsrContractError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Unable to query redemption rate of {stk_denom} from lsr contract, {error}")]
    UnableToQueryRedemptionRate { stk_denom: String, error: String },

    #[error("Pool {pool_id} is not configured in the contract")]
    PoolNotFound { pool_id: u64 },

    #[error("Pool {pool_id} is already configured in the contract")]
    PoolAlreadyExists { pool_id: u64 },

    #[error("Pool {pool_id} not found on Osmosis")]
    PoolNotFoundOsmosis { pool_id: u64 },

    #[error("Invalid denom: {denom}")]
    InvalidDenom { denom: String },

    #[error("The specified asset ordering does not match the underlying pool")]
    InvalidPoolAssetOrdering {},

    #[error("The underlying pool has {number} of assets, only 2 is allowed")]
    InvalidNumberOfPoolAssets { number: u64 },

    #[error("The scaling factor controller for pool {pool_id} is invalid: {controller}")]
    InvalidScalingFactorController { pool_id: u64, controller: String },

    #[error("LSR error: {0}")]
    LsrError(String),
}

impl From<LsrContractError> for ContractError {
    fn from(error: LsrContractError) -> Self {
        ContractError::LsrError(error.to_string())
    }
}
