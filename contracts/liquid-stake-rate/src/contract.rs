#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use ratesync::{
    lsr_helpers::validate_redemption_rate,
    lsr_msg::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, RedemptionRateResponse,
        RedemptionRates,
    },
    lsr_state::{
        AnomalyConfig, Config, History, RedemptionRate, ANOMALY_CONFIG_BY_DENOM, CONFIG,
        LIQUID_STAKE_RATES,
    },
};

use ratesync::{
    lsr_error::ContractError,
    lsr_helpers::{
        denom_trace_to_hash, option_string_to_addr, validate_channel_id, validate_native_denom,
    },
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:liquid-stake-rate";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_DEVIAITON_COUNT_LIMIT: u64 = 10;
const DEFAULT_DEVIAITON_THRESHOLD: Decimal = Decimal::percent(5);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_channel_id(&msg.transfer_channel_i_d)?;

    let count_limit = msg
        .deviation_count_limit
        .unwrap_or(DEFAULT_DEVIAITON_COUNT_LIMIT);
    let threshold = msg
        .deviation_threshold
        .unwrap_or(DEFAULT_DEVIAITON_THRESHOLD);

    CONFIG.save(
        deps.storage,
        &Config {
            owner: option_string_to_addr(deps.api, msg.admin, info.sender.clone())?,
            transfer_channel_i_d: msg.transfer_channel_i_d.clone(),
            transfer_port_i_d: msg.transfer_port_i_d.clone(),
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("transfer_channel_id", msg.transfer_channel_i_d)
        .add_attribute("transfer_port_id", msg.transfer_port_i_d)
        .add_attribute("deviation_count_limit", count_limit.to_string())
        .add_attribute("deviation_threshold", threshold.to_string()))
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
        } => execute_add_liquid_stake_rate(
            deps,
            env,
            info,
            default_bond_denom,
            stk_denom,
            c_value,
            controller_chain_time,
        ),

        ExecuteMsg::UpdateConfig {
            transfer_channel_i_d,
            transfer_port_i_d,
        } => execute_update_config(deps, env, info, transfer_channel_i_d, transfer_port_i_d),

        ExecuteMsg::SetAnomalyConfig {
            stk_denom,
            deviation_count_limit,
            deviation_threshold,
        } => execute_set_anomaly_config(
            deps,
            env,
            info,
            stk_denom,
            deviation_count_limit,
            deviation_threshold,
        ),
    }
}

// Set liquid stake rate
pub fn execute_add_liquid_stake_rate(
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

    // Validate denom
    validate_native_denom(&default_bond_denom.clone())?;

    // Convert stk_denom to ibc hash
    let stk_denom_ibc_hash = denom_trace_to_hash(
        &stk_denom,
        &config.transfer_port_i_d,
        &config.transfer_channel_i_d,
    )?;

    // check if anomaly config exists, else set default
    let anomaly_config =
        match ANOMALY_CONFIG_BY_DENOM.may_load(deps.storage, &stk_denom_ibc_hash)? {
            Some(config) => config,
            None => AnomalyConfig::default(),
        };
    ANOMALY_CONFIG_BY_DENOM.save(deps.storage, &stk_denom_ibc_hash.clone(), &anomaly_config)?;

    // Add liquid stake rate to historical state
    let mut new_liquid_stake_rate = RedemptionRate {
        denom: stk_denom_ibc_hash.clone(),
        redemption_rate: c_value,
        update_time: controller_chain_time,
        anomaly_detected: false,
    };

    let mut liquid_stake_rate_history =
        match LIQUID_STAKE_RATES.may_load(deps.storage, &stk_denom_ibc_hash.clone())? {
            Some(history) => {
                new_liquid_stake_rate.anomaly_detected =
                    validate_redemption_rate(deps.as_ref(), c_value, stk_denom_ibc_hash.clone())?;

                history
            }
            None => History::<RedemptionRate>::default(),
        };
    liquid_stake_rate_history.add(new_liquid_stake_rate.clone());
    LIQUID_STAKE_RATES.save(
        deps.storage,
        &stk_denom_ibc_hash.clone(),
        &liquid_stake_rate_history,
    )?;

    Ok(Response::new()
        .add_attribute("action", "set_liquid_stake_rate")
        .add_attribute("default_bond_denom", default_bond_denom)
        .add_attribute("stk_denom", stk_denom)
        .add_attribute("stk_denom_ibc_hash", stk_denom_ibc_hash)
        .add_attribute("c_value", c_value.to_string())
        .add_attribute("controller_chain_time", controller_chain_time.to_string())
        .add_attribute(
            "anomaly_detected",
            new_liquid_stake_rate.anomaly_detected.to_string(),
        ))
}

// Update config
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    transfer_channel_i_d: Option<String>,
    transfer_port_i_d: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(channel_id) = transfer_channel_i_d.clone() {
        validate_channel_id(&channel_id)?;
        config.transfer_channel_i_d = channel_id;
    }

    if let Some(port_id) = transfer_port_i_d.clone() {
        config.transfer_port_i_d = port_id;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute(
            "transfer_channel_id",
            transfer_channel_i_d.unwrap_or_default(),
        )
        .add_attribute("transfer_port_id", transfer_port_i_d.unwrap_or_default()))
}

// Set anomaly config
pub fn execute_set_anomaly_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stk_denom: String,
    deviation_count_limit: u64,
    deviation_threshold: Decimal,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let stk_denom_ibc_hash = denom_trace_to_hash(
        &stk_denom,
        &config.transfer_port_i_d,
        &config.transfer_channel_i_d,
    )?;

    ANOMALY_CONFIG_BY_DENOM.save(
        deps.storage,
        &stk_denom_ibc_hash,
        &AnomalyConfig {
            count_limit: deviation_count_limit,
            threshold: deviation_threshold,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "set_anomaly_config")
        .add_attribute("stk_denom", stk_denom)
        .add_attribute("stk_denom_ibc_hash", stk_denom_ibc_hash)
        .add_attribute("deviation_count_limit", deviation_count_limit.to_string())
        .add_attribute("deviation_threshold", deviation_threshold.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),

        QueryMsg::AnomalyConfig { denom } => to_json_binary(&query_anomaly_config(deps, denom)?),

        QueryMsg::RedemptionRate { denom, params } => {
            to_json_binary(&get_latest_liquid_stake_rate(deps, denom, params)?)
        }

        QueryMsg::HistoricalRedemptionRates {
            denom,
            params,
            limit,
            ..
        } => to_json_binary(&get_historical_liquid_stake_rates(
            deps, denom, params, limit,
        )?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner,
        transfer_channel_i_d: config.transfer_channel_i_d,
        transfer_port_i_d: config.transfer_port_i_d,
    })
}

fn query_anomaly_config(deps: Deps, denom: String) -> Result<AnomalyConfig, ContractError> {
    let anomaly_config = ANOMALY_CONFIG_BY_DENOM.load(deps.storage, &denom)?;

    Ok(anomaly_config)
}

pub fn get_latest_liquid_stake_rate(
    deps: Deps,
    ibc_denom: String,
    extra: Option<Binary>,
) -> Result<RedemptionRateResponse, ContractError> {
    if extra.is_some() {
        return Err(ContractError::InvalidQueryRequest {
            reason: "params must be None".to_string(),
        });
    }

    let liquid_stake_rates_history = LIQUID_STAKE_RATES.load(deps.storage, &ibc_denom)?;

    match liquid_stake_rates_history.get_latest() {
        Some(response) => Ok(RedemptionRateResponse {
            redemption_rate: response.redemption_rate,
            update_time: response.update_time,
        }),
        None => Err(ContractError::InvalidQueryRequest {
            reason: "liquid stake rate not found".to_string(),
        }),
    }
}

pub fn get_historical_liquid_stake_rates(
    deps: Deps,
    ibc_denom: String,
    extra: Option<Binary>,
    limit: Option<u64>,
) -> Result<RedemptionRates, ContractError> {
    if extra.is_some() {
        return Err(ContractError::InvalidQueryRequest {
            reason: "params must be None".to_string(),
        });
    }

    let liquid_stake_rates_history = LIQUID_STAKE_RATES.load(deps.storage, &ibc_denom)?;

    let c_value_rates = match limit {
        Some(limit) => liquid_stake_rates_history.get_latest_range(limit as usize),
        None => liquid_stake_rates_history.get_all(),
    };
    Ok(RedemptionRates {
        redemption_rates: c_value_rates,
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{attr, coins, from_json, Empty, OwnedDeps};

    const OWNER_ADDRESS: &str = "creator";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            admin: Some("owner".to_string()),
            transfer_channel_i_d: "channel-0".to_string(),
            transfer_port_i_d: "transfer".to_string(),
            deviation_count_limit: None,
            deviation_threshold: None,
        };
        let info = mock_info(OWNER_ADDRESS, &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("method", "instantiate"),
                attr("owner", OWNER_ADDRESS.to_string()),
                attr("transfer_channel_id", "channel-0".to_string()),
                attr("transfer_port_id", "transfer".to_string()),
                attr("deviation_count_limit", "10".to_string()),
                attr("deviation_threshold", "0.05".to_string()),
            ]
        );

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_json(res).unwrap();
        assert_eq!("owner", value.owner);
    }

    #[test]
    fn test_update_config() {
        let (mut deps, env, info) = default_instantiate();

        let msg = ExecuteMsg::UpdateConfig {
            transfer_channel_i_d: Some("channel-1".to_string()),
            transfer_port_i_d: Some("transfer".to_string()),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "update_config"),
                attr("transfer_channel_id", "channel-1".to_string()),
                attr("transfer_port_id", "transfer".to_string()),
            ]
        );

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_json(res).unwrap();
        assert_eq!("creator", value.owner);
    }

    // set amnomaly config for denom
    #[test]
    fn test_set_anomaly_config() {
        let (mut deps, env, info) = default_instantiate();

        let msg = ExecuteMsg::SetAnomalyConfig {
            stk_denom: "somecoin1".to_string(),
            deviation_count_limit: 10,
            deviation_threshold: Decimal::percent(5),
        };

        let expected_ibc_hash =
            "ibc/2CC566890930B4BCBA686137D36CDEEFB221AE4726408D7DA0F9B2A40E9CA2AB".to_string();

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_anomaly_config"),
                attr("stk_denom", "somecoin1".to_string()),
                attr("stk_denom_ibc_hash", expected_ibc_hash.clone()),
                attr("deviation_count_limit", "10".to_string()),
                attr("deviation_threshold", "0.05".to_string()),
            ]
        );

        // it worked, let's query the state
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::AnomalyConfig {
                denom: expected_ibc_hash,
            },
        )
        .unwrap();
        let value: AnomalyConfig = from_json(res).unwrap();
        assert_eq!(10, value.count_limit);
        assert_eq!(Decimal::percent(5), value.threshold);
    }

    #[test]
    fn test_unauthorized_update_config() {
        let (mut deps, env, _info) = default_instantiate();

        // unauthorized attempt
        let info = mock_info("anyone", &coins(1000, "earth"));
        let msg = ExecuteMsg::UpdateConfig {
            transfer_channel_i_d: Some("channel-1".to_string()),
            transfer_port_i_d: Some("transfer".to_string()),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // unauthorized attempt for anomaly config
        let msg = ExecuteMsg::SetAnomalyConfig {
            stk_denom: "somecoin1".to_string(),
            deviation_count_limit: 10,
            deviation_threshold: Decimal::percent(5),
        };

        let res = execute(deps.as_mut(), env, info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn set_liq_stake_rate() {
        let (mut deps, env, info) = default_instantiate();

        let msg = ExecuteMsg::LiquidStakeRate {
            default_bond_denom: "somecoin1".to_string(),
            stk_denom: "somecoin2".to_string(),
            c_value: Decimal::percent(1),
            controller_chain_time: 1,
        };

        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let ibc_hash_denom = denom_trace_to_hash("somecoin2", "transfer", "channel-0").unwrap();

        // it worked, let's query the state
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::RedemptionRate {
                denom: ibc_hash_denom,
                params: None,
            },
        )
        .unwrap();
        let value: RedemptionRateResponse = from_json(res).unwrap();
        assert_eq!(Decimal::percent(1), value.redemption_rate);
    }

    #[test]
    fn set_liquid_stake_rate_should_fail() {
        let (mut deps, env, _info) = default_instantiate();

        // unauthorized attempt
        let info = mock_info("anyone", &coins(1000, "earth"));
        let msg = ExecuteMsg::LiquidStakeRate {
            default_bond_denom: "somecoin1".to_string(),
            stk_denom: "somecoin2".to_string(),
            c_value: Decimal::percent(1),
            controller_chain_time: 1,
        };

        let res = execute(deps.as_mut(), env, info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn test_historical_queries() {
        // Instantiate contract
        let (mut deps, env, info) = default_instantiate();

        let default_bond_denom = "somecoin1".to_string();
        let stk_denom = "stk/somecoin1".to_string();
        let ibc_hash_denom = denom_trace_to_hash(&stk_denom, "transfer", "channel-0").unwrap();

        let msg1 = get_execute_msg(default_bond_denom.clone(), stk_denom.clone(), "1.01", 1);
        let msg2 = get_execute_msg(default_bond_denom.clone(), stk_denom.clone(), "1.02", 2);
        let msg3 = get_execute_msg(default_bond_denom.clone(), stk_denom.clone(), "1.03", 2);
        let msg4 = get_execute_msg(default_bond_denom.clone(), stk_denom.clone(), "1.04", 3);

        let rr1 = get_test_liquid_stake_rate("1.01", 1);
        let rr2 = get_test_liquid_stake_rate("1.03", 2);
        let rr3 = get_test_liquid_stake_rate("1.04", 3);

        // Execute each message out of order, and with msg2 coming before msg3
        execute(deps.as_mut(), env.clone(), info.clone(), msg2).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), msg1).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), msg3).unwrap();
        execute(deps.as_mut(), env.clone(), info, msg4).unwrap();

        // Check the corresponding liquid stake rate query
        let msg = QueryMsg::HistoricalRedemptionRates {
            denom: ibc_hash_denom.clone(),
            params: None,
            limit: None,
        };
        let resp = query(deps.as_ref(), env.clone(), msg).unwrap();
        let history_response: RedemptionRates = from_json(resp).unwrap();
        assert_eq!(
            history_response,
            RedemptionRates {
                redemption_rates: vec![rr3.clone(), rr2.clone(), rr1.clone()]
            }
        );

        // Check the liquid stake rate query with a limit
        let msg = QueryMsg::HistoricalRedemptionRates {
            denom: ibc_hash_denom.clone(),
            params: None,
            limit: Some(2),
        };
        let resp = query(deps.as_ref(), env, msg).unwrap();
        let history_response: RedemptionRates = from_json(resp).unwrap();
        assert_eq!(
            history_response,
            RedemptionRates {
                redemption_rates: vec![rr3, rr2]
            }
        );
    }

    #[test]
    fn test_all_latest_msgs() {
        // Instantiate contract
        let (mut deps, env, info) = default_instantiate();

        // Build three msgs - each with a new and an old record
        let msg1_old =
            get_execute_msg("somecoin1".to_string(), "stk/somecoin1".to_string(), "1", 0);
        let msg2_old =
            get_execute_msg("somecoin2".to_string(), "stk/somecoin2".to_string(), "2", 0);
        let msg3_old =
            get_execute_msg("somecoin3".to_string(), "stk/somecoin3".to_string(), "3", 0);

        let msg1_new =
            get_execute_msg("somecoin1".to_string(), "stk/somecoin1".to_string(), "1", 1);
        let msg2_new =
            get_execute_msg("somecoin2".to_string(), "stk/somecoin2".to_string(), "2", 2);
        let msg3_new =
            get_execute_msg("somecoin3".to_string(), "stk/somecoin3".to_string(), "3", 3);

        // Execute each message
        execute(deps.as_mut(), env.clone(), info.clone(), msg1_old).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), msg2_old).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), msg3_old).unwrap();

        execute(deps.as_mut(), env.clone(), info.clone(), msg1_new).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), msg2_new).unwrap();
        execute(deps.as_mut(), env.clone(), info, msg3_new).unwrap();

        let ibc_hash_denom1 =
            denom_trace_to_hash("stk/somecoin1", "transfer", "channel-0").unwrap();
        let ibc_hash_denom2 =
            denom_trace_to_hash("stk/somecoin2", "transfer", "channel-0").unwrap();
        let ibc_hash_denom3 =
            denom_trace_to_hash("stk/somecoin3", "transfer", "channel-0").unwrap();

        // Confirm all msgs are preset and are sorted
        let query_msg1 = QueryMsg::RedemptionRate {
            denom: ibc_hash_denom1.clone(),
            params: None,
        };
        let query_msg2 = QueryMsg::RedemptionRate {
            denom: ibc_hash_denom2.clone(),
            params: None,
        };
        let query_msg3 = QueryMsg::RedemptionRate {
            denom: ibc_hash_denom3.clone(),
            params: None,
        };

        let resp1 = query(deps.as_ref(), env.clone(), query_msg1).unwrap();
        let resp2 = query(deps.as_ref(), env.clone(), query_msg2).unwrap();
        let resp3 = query(deps.as_ref(), env, query_msg3).unwrap();

        let msg_responses1: RedemptionRateResponse = from_json(resp1).unwrap();
        let msg_responses2: RedemptionRateResponse = from_json(resp2).unwrap();
        let msg_responses3: RedemptionRateResponse = from_json(resp3).unwrap();

        assert_eq!(
            msg_responses1,
            RedemptionRateResponse {
                redemption_rate: Decimal::from_str("1").unwrap(),
                update_time: 1,
            }
        );
        assert_eq!(
            msg_responses2,
            RedemptionRateResponse {
                redemption_rate: Decimal::from_str("2").unwrap(),
                update_time: 2,
            }
        );
        assert_eq!(
            msg_responses3,
            RedemptionRateResponse {
                redemption_rate: Decimal::from_str("3").unwrap(),
                update_time: 3,
            }
        )
    }

    #[test]
    fn test_anomaly_detected() {
        // Instantiate contract
        let (mut deps, env, info) = default_instantiate();

        let default_bond_denom = "somecoin1".to_string();
        let stk_denom = "stk/somecoin1".to_string();
        let ibc_hash_denom = denom_trace_to_hash(&stk_denom, "transfer", "channel-0").unwrap();

        // add liquid stake rate
        let msg = ExecuteMsg::LiquidStakeRate {
            default_bond_denom: default_bond_denom.clone(),
            stk_denom: stk_denom.clone(),
            c_value: Decimal::percent(1),
            controller_chain_time: 1,
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // it worked, let's query the state
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::HistoricalRedemptionRates {
                denom: ibc_hash_denom.clone(),
                params: None,
                limit: None,
            },
        )
        .unwrap();

        let history_response: RedemptionRates = from_json(res).unwrap();
        assert_eq!(1, history_response.redemption_rates.len());
        assert_eq!(
            Decimal::percent(1),
            history_response.redemption_rates[0].redemption_rate
        );
        assert_eq!(1, history_response.redemption_rates[0].update_time);
        assert_eq!(false, history_response.redemption_rates[0].anomaly_detected);

        // set anomaly config
        set_anomaly_config(
            &mut deps,
            env.clone(),
            info.clone(),
            stk_denom.clone(),
            1,
            Decimal::percent(1),
        );

        // add liquid stake rate
        let msg = ExecuteMsg::LiquidStakeRate {
            default_bond_denom: default_bond_denom.clone(),
            stk_denom: stk_denom.clone(),
            c_value: Decimal::from_str("1.1").unwrap(),
            controller_chain_time: 2,
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // it worked, let's query the state
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::HistoricalRedemptionRates {
                denom: ibc_hash_denom.clone(),
                params: None,
                limit: None,
            },
        )
        .unwrap();

        let history_response: RedemptionRates = from_json(res).unwrap();
        assert_eq!(2, history_response.redemption_rates.len());
        assert_eq!(
            Decimal::from_str("1.1").unwrap(),
            history_response.redemption_rates[0].redemption_rate
        );
        assert_eq!(2, history_response.redemption_rates[0].update_time);
        assert_eq!(true, history_response.redemption_rates[0].anomaly_detected);
    }

    // helper function to instantiate contract
    fn default_instantiate() -> (
        OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        Env,
        MessageInfo,
    ) {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(OWNER_ADDRESS, &coins(1000, "earth"));

        let msg = InstantiateMsg {
            admin: Some(OWNER_ADDRESS.to_string()),
            transfer_channel_i_d: "channel-0".to_string(),
            transfer_port_i_d: "transfer".to_string(),
            deviation_count_limit: None,
            deviation_threshold: None,
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        (deps, env, info)
    }

    // helper function to get a test liquid stake rate
    fn get_test_liquid_stake_rate(value: &str, time: u64) -> RedemptionRate {
        let ibc_hash_denom = denom_trace_to_hash("stk/somecoin1", "transfer", "channel-0").unwrap();
        RedemptionRate {
            denom: ibc_hash_denom.clone(),
            redemption_rate: Decimal::from_str(value).unwrap(),
            update_time: time,
            anomaly_detected: false,
        }
    }

    // helper function to get a test execute message
    fn get_execute_msg(
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

    // helper function to set anomaly config for denom
    fn set_anomaly_config(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        env: Env,
        info: MessageInfo,
        stk_denom: String,
        deviation_count_limit: u64,
        deviation_threshold: Decimal,
    ) {
        let msg = ExecuteMsg::SetAnomalyConfig {
            stk_denom,
            deviation_count_limit,
            deviation_threshold,
        };

        let _res = execute(deps.as_mut(), env, info, msg).unwrap();
    }
}
