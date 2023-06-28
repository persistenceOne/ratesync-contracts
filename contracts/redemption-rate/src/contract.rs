#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::option_string_to_addr;
use crate::msg::{
    ConfigParams, ConfigResponse, ExecuteMsg, InstantiateMsg, Price, QueryMsg,
    RedemptionRateResponse,
};
use crate::state::{Config, RedemptionRate, CONFIG, PRICES};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:redemption-rate";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let ConfigParams { owner } = msg.config;

    CONFIG.save(
        deps.storage,
        &Config {
            owner: option_string_to_addr(deps.api, owner, info.sender.clone())?,
            new_owner: Addr::unchecked(""),
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { config } => execute_update_config(deps, info, config),

        ExecuteMsg::AcceptOwnership {} => execute_accept_ownership(deps, info.sender),

        ExecuteMsg::CancelOwnership {} => execute_cancel_ownership(deps, info.sender),

        ExecuteMsg::SetRedemptionRate {
            price,
            exchange_rate,
        } => execute_set_redemption_rate(deps, env, info, price, exchange_rate),
    }
}

// Update config
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: ConfigParams,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let ConfigParams { owner } = new_config;

    // Update config
    let new_owner = option_string_to_addr(deps.api, owner, config.owner.clone())?;
    if new_owner != config.owner {
        config.new_owner = new_owner;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("owner", config.owner))
}

// Accept a new owner
pub fn execute_accept_ownership(
    deps: DepsMut,
    sender_address: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if sender_address != config.new_owner {
        return Err(ContractError::Unauthorized {});
    }

    config.owner = config.new_owner.clone();
    config.new_owner = Addr::unchecked("");

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "accept_owner"))
}

// Cancel a new owner
pub fn execute_cancel_ownership(
    deps: DepsMut,
    sender_address: Addr,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if sender_address != config.new_owner {
        return Err(ContractError::Unauthorized {});
    }

    config.new_owner = Addr::unchecked("");

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "cancel_owner"))
}

// Set redemption rate
pub fn execute_set_redemption_rate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    price: Price,
    exchange_rate: Decimal,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let key = format!("{}:{}", price.denom, price.base_denom).into_bytes();
    let redemption_rate = RedemptionRate {
        exchange_rate,
        last_updated: env.block.time.seconds(),
    };

    PRICES.save(deps.storage, &key, &redemption_rate)?;

    Ok(Response::new().add_attribute("action", "set_redemption_rate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),

        QueryMsg::RedemptionRateRequest { price } => {
            to_binary(&query_redemption_rate(deps, price)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner,
    })
}

fn query_redemption_rate(deps: Deps, price: Price) -> StdResult<RedemptionRateResponse> {
    let key = format!("{}:{}", price.denom, price.base_denom).into_bytes();
    let redemption_rate = PRICES.load(deps.storage, &key)?;

    Ok(RedemptionRateResponse {
        exchange_rate: redemption_rate.exchange_rate,
        last_updated: redemption_rate.last_updated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            config: ConfigParams {
                owner: Some("owner".to_string()),
            },
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner", value.owner);
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            config: ConfigParams {
                owner: Some("owner".to_string()),
            },
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("owner", &coins(1000, "earth"));
        let msg = ExecuteMsg::UpdateConfig {
            config: ConfigParams {
                owner: Some("new_owner".to_string()),
            },
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, but owner is not yet updated. let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner", value.owner);

        // accept ownership
        let info = mock_info("new_owner", &coins(1000, "earth"));
        let msg = ExecuteMsg::AcceptOwnership {};

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("new_owner", value.owner);
    }

    #[test]
    fn cancel_ownership() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            config: ConfigParams {
                owner: Some("owner".to_string()),
            },
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("owner", &coins(1000, "earth"));
        let msg = ExecuteMsg::UpdateConfig {
            config: ConfigParams {
                owner: Some("new_owner".to_string()),
            },
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, but owner is not yet updated. let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner", value.owner);

        // cancel ownership
        let info = mock_info("new_owner", &coins(1000, "earth"));
        let msg = ExecuteMsg::CancelOwnership {};

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner", value.owner);
    }

    #[test]
    fn set_redemption_rate() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            config: ConfigParams {
                owner: Some("owner".to_string()),
            },
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("owner", &coins(1000, "earth"));
        let msg = ExecuteMsg::SetRedemptionRate {
            price: Price {
                denom: "somecoin1".to_string(),
                base_denom: "somecoin2".to_string(),
            },
            exchange_rate: Decimal::percent(1),
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::RedemptionRateRequest {
                price: Price {
                    denom: "somecoin1".to_string(),
                    base_denom: "somecoin2".to_string(),
                },
            },
        )
        .unwrap();
        let value: RedemptionRateResponse = from_binary(&res).unwrap();
        assert_eq!(Decimal::percent(1), value.exchange_rate);
    }
}
