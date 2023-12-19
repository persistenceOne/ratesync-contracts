#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::{option_string_to_addr, validate_native_denom};
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, LiquidStakeRateResponse, LiquidStakeRates, QueryMsg,
};
use crate::state::{Config, History, LiquidStakeRate, CONFIG, LIQUID_STAKE_RATES};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:liquid-stake-rate";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            owner: option_string_to_addr(deps.api, msg.admin, info.sender.clone())?,
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
        ExecuteMsg::LiquidStakeRate {
            default_bond_denom,
            stk_denom,
            c_value,
            controller_chain_time,
        } => execute_liquid_stake_rate(
            deps,
            env,
            info,
            default_bond_denom,
            stk_denom,
            c_value,
            controller_chain_time,
        ),
    }
}
// Set liquid stake rate
pub fn execute_liquid_stake_rate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    default_bond_denom: String,
    stk_denom: String,
    c_value: Decimal,
    controller_chain_time: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    validate_native_denom(&default_bond_denom.clone())?;

    let key = format!("{}:{}", default_bond_denom, stk_denom).into_bytes();
    let new_liquid_stake_rate = LiquidStakeRate {
        c_value,
        last_updated: controller_chain_time,
    };

    let mut liquid_stake_rate_history = match LIQUID_STAKE_RATES.may_load(deps.storage, &key)? {
        Some(history) => history,
        None => History::<LiquidStakeRate>::default(),
    };
    liquid_stake_rate_history.add(new_liquid_stake_rate);
    LIQUID_STAKE_RATES.save(deps.storage, &key, &liquid_stake_rate_history)?;

    Ok(Response::new()
        .add_attribute("action", "set_liquid_stake_rate")
        .add_attribute("default_bond_denom", default_bond_denom)
        .add_attribute("stk_denom", stk_denom)
        .add_attribute("c_value", c_value.to_string())
        .add_attribute("controller_chain_time", controller_chain_time.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),

        QueryMsg::LiquidStakeRate {
            default_bond_denom,
            stk_denom,
        } => to_binary(&get_latest_liquid_stake_rate(
            deps,
            default_bond_denom,
            stk_denom,
        )?),

        QueryMsg::HistoricalLiquidStakeRates {
            default_bond_denom,
            stk_denom,
            limit,
            ..
        } => to_binary(&get_historical_liquid_stake_rates(
            deps,
            default_bond_denom,
            stk_denom,
            limit,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner,
    })
}

pub fn get_latest_liquid_stake_rate(
    deps: Deps,
    default_bond_denom: String,
    stk_denom: String,
) -> StdResult<LiquidStakeRateResponse> {
    let key = format!("{}:{}", default_bond_denom, stk_denom).into_bytes();
    let liquid_stake_rates_history: History<LiquidStakeRate> =
        LIQUID_STAKE_RATES.load(deps.storage, &key)?;

    match liquid_stake_rates_history.get_latest() {
        Some(response) => Ok(LiquidStakeRateResponse {
            c_value: response.c_value,
            last_updated: response.last_updated,
        }),
        None => Err(StdError::generic_err("liquid stake rate not found")),
    }
}

pub fn get_historical_liquid_stake_rates(
    deps: Deps,
    default_bond_denom: String,
    stk_denom: String,
    limit: Option<u64>,
) -> StdResult<LiquidStakeRates> {
    let key = format!("{}:{}", default_bond_denom, stk_denom).into_bytes();
    let liquid_stake_rates_history = LIQUID_STAKE_RATES.load(deps.storage, &key)?;

    let c_value_rates = match limit {
        Some(limit) => liquid_stake_rates_history.get_latest_range(limit as usize),
        None => liquid_stake_rates_history.get_all(),
    };
    Ok(LiquidStakeRates { c_value_rates })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coins, from_binary, Empty, OwnedDeps, Timestamp};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            admin: Some("owner".to_string()),
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
    fn set_liq_stake_rate() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            admin: Some("owner".to_string()),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("owner", &coins(1000, "earth"));
        let msg = ExecuteMsg::LiquidStakeRate {
            default_bond_denom: "somecoin1".to_string(),
            stk_denom: "somecoin2".to_string(),
            c_value: Decimal::percent(1),
            controller_chain_time: 1,
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::LiquidStakeRate {
                default_bond_denom: "somecoin1".to_string(),
                stk_denom: "somecoin2".to_string(),
            },
        )
        .unwrap();
        let value: LiquidStakeRateResponse = from_binary(&res).unwrap();
        assert_eq!(Decimal::percent(1), value.c_value);
    }

    #[test]
    fn set_liquid_stake_rate_should_fail() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            admin: Some("owner".to_string()),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // unauthorized attempt
        let info = mock_info("anyone", &coins(1000, "earth"));
        let msg = ExecuteMsg::LiquidStakeRate {
            default_bond_denom: "somecoin1".to_string(),
            stk_denom: "somecoin2".to_string(),
            c_value: Decimal::percent(1),
            controller_chain_time: 1,
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    fn default_instantiate() -> (
        OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        Env,
        MessageInfo,
    ) {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            admin: Some("owner".to_string()),
        };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("owner", &coins(1000, "earth"));

        (deps, mock_env(), info)
    }

    fn get_test_liquid_stake_rate(value: &str, time: u64) -> LiquidStakeRate {
        LiquidStakeRate {
            c_value: Decimal::from_str(value).unwrap(),
            last_updated: time,
        }
    }

    fn get_post_metric_msg(
        default_bond_denom: String,
        stk_denom: String,
        c_value: &str,
        controller_chain_time: u64,
    ) -> ExecuteMsg {
        ExecuteMsg::LiquidStakeRate {
            default_bond_denom,
            stk_denom,
            c_value: Decimal::from_str(c_value).unwrap(),
            controller_chain_time,
        }
    }

    #[test]
    fn test_historical_queries() {
        // Instantiate contract
        let (mut deps, env, info) = default_instantiate();

        let default_bond_denom = "somecoin1".to_string();
        let stk_denom = "somecoin2".to_string();

        let msg1 = get_post_metric_msg(default_bond_denom.clone(), stk_denom.clone(), "1", 1);
        let msg2 = get_post_metric_msg(default_bond_denom.clone(), stk_denom.clone(), "2", 2);
        let msg3 = get_post_metric_msg(default_bond_denom.clone(), stk_denom.clone(), "3", 2);
        let msg4 = get_post_metric_msg(default_bond_denom.clone(), stk_denom.clone(), "4", 3);

        let rr1 = get_test_liquid_stake_rate("1", 1);
        let rr2 = get_test_liquid_stake_rate("3", 2);
        let rr3 = get_test_liquid_stake_rate("4", 3);

        // Execute each message out of order, and with msg2 coming before msg3
        execute(deps.as_mut(), env.clone(), info.clone(), msg2).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), msg1).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), msg3).unwrap();
        execute(deps.as_mut(), env.clone(), info, msg4).unwrap();

        // Check the corresponding liquid stake rate query
        let msg = QueryMsg::HistoricalLiquidStakeRates {
            default_bond_denom: default_bond_denom.clone(),
            stk_denom: stk_denom.clone(),
            limit: None,
        };
        let resp = query(deps.as_ref(), env.clone(), msg).unwrap();
        let history_response: LiquidStakeRates = from_binary(&resp).unwrap();
        assert_eq!(
            history_response,
            LiquidStakeRates {
                c_value_rates: vec![rr3.clone(), rr2.clone(), rr1.clone()]
            }
        );

        // Check the liquid stake rate query with a limit
        let msg = QueryMsg::HistoricalLiquidStakeRates {
            default_bond_denom: default_bond_denom.clone(),
            stk_denom: stk_denom.clone(),
            limit: Some(2),
        };
        let resp = query(deps.as_ref(), env, msg).unwrap();
        let history_response: LiquidStakeRates = from_binary(&resp).unwrap();
        assert_eq!(
            history_response,
            LiquidStakeRates {
                c_value_rates: vec![rr3, rr2]
            }
        );
    }

    #[test]
    fn test_timestamp_u64() {
        let time = Timestamp::from_nanos(1);
        let time_u64: u64 = time.nanos();
        assert_eq!(time_u64, 1);
    }
}
