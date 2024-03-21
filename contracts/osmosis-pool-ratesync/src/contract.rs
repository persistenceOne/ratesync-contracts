use cosmwasm_std::StdError;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    ensure, entry_point, to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QueryRequest, Response, StdResult, WasmQuery,
};
use cw2::set_contract_version;
use osmosis_std::types::osmosis::{
    gamm::poolmodels::stableswap::v1beta1::{
        MsgStableSwapAdjustScalingFactors, Pool as StableswapPool,
    },
    poolmanager::v1beta1::PoolmanagerQuerier,
};

use crate::{
    error::ContractError,
    helpers::{
        convert_redemption_rate_to_scaling_factors, denom_trace_to_hash, validate_channel_id,
        validate_pool_configuration,
    },
    msg::{
        ExecuteMsg, InstantiateMsg, LiquidStakeRateQueryMsg, Pools, QueryMsg,
        RedemptionRateResponse,
    },
    state::{Config, Pool, CONFIG, POOLS},
};

const CONTRACT_NAME: &str = "crates.io:osmosis-pool-ratesync";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner_address: deps.api.addr_validate(&msg.owner_address)?,
        lsr_contract_address: deps.api.addr_validate(&msg.lsr_contract_address)?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner_address", msg.owner_address)
        .add_attribute("lsr_contract_address", msg.lsr_contract_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner_address,
            lsr_contract_address,
        } => execute_update_config(deps, info, owner_address, lsr_contract_address),
        ExecuteMsg::AddPool {
            pool_id,
            stk_token_denom,
            transfer_port_id,
            transfer_channel_id,
            asset_ordering,
        } => {
            let pool = Pool {
                pool_id,
                stk_token_denom: stk_token_denom.clone(),
                transfer_port_id: transfer_port_id.clone(),
                transfer_channel_id: transfer_channel_id.clone(),
                ibc_hash_stk_denom: "".to_string(),
                asset_ordering: asset_ordering.clone(),
                last_updated: 0,
            };

            execute_add_pool(deps, env, info, pool)
        }
        ExecuteMsg::RemovePool { pool_id } => execute_remove_pool(deps, info, pool_id),
        ExecuteMsg::UpdateScalingFactor { pool_id } => {
            execute_update_scaling_factor(deps, env, pool_id)
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner_address: String,
    lsr_contract_address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner_address {
        return Err(ContractError::Unauthorized {});
    }

    let updated_config = Config {
        owner_address: deps.api.addr_validate(&owner_address)?,
        lsr_contract_address: deps.api.addr_validate(&lsr_contract_address)?,
    };

    CONFIG.save(deps.storage, &updated_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("owner_address", owner_address)
        .add_attribute("lsr_contract_address", lsr_contract_address))
}

pub fn execute_add_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut pool: Pool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.owner_address,
        ContractError::Unauthorized {}
    );

    let pool_id = pool.pool_id;

    // Ensure the pool does not already exist
    if POOLS.has(deps.storage, pool_id) {
        return Err(ContractError::PoolAlreadyExists { pool_id });
    }

    // Query the pool from the pool manager
    let query_pool_resp = PoolmanagerQuerier::new(&deps.querier).pool(pool_id)?;
    let stableswap_pool: StableswapPool = query_pool_resp
        .pool
        .ok_or(ContractError::PoolNotFoundOsmosis { pool_id })?
        .try_into()
        .map_err(|e| {
            StdError::parse_err(
                "osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::Pool",
                e,
            )
        })?;

    // Ensure the pool's scaling factor controller is the contract
    if stableswap_pool.scaling_factor_controller != env.contract.address {
        return Err(ContractError::InvalidScalingFactorController {
            pool_id,
            controller: stableswap_pool.scaling_factor_controller,
        });
    }

    let transfer_channel_id = pool.transfer_channel_id.clone();
    let asset_ordering = pool.asset_ordering.clone();

    validate_channel_id(&transfer_channel_id.clone())?;

    let ibc_hash_stk_denom = denom_trace_to_hash(
        &pool.stk_token_denom,
        &pool.transfer_port_id,
        &transfer_channel_id,
    )?;

    validate_pool_configuration(
        stableswap_pool,
        pool_id,
        ibc_hash_stk_denom.clone(),
        asset_ordering.clone(),
    )?;

    pool.ibc_hash_stk_denom = ibc_hash_stk_denom.clone();
    POOLS.save(deps.storage, pool_id, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "add_pool")
        .add_attribute("pool_id", pool_id.to_string())
        .add_attribute("pool_stk_token_denom", ibc_hash_stk_denom)
        .add_attribute("pool_asset_ordering", asset_ordering.to_string()))
}

pub fn execute_remove_pool(
    deps: DepsMut,
    info: MessageInfo,
    pool_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        info.sender == config.owner_address,
        ContractError::Unauthorized {}
    );

    if !POOLS.has(deps.storage, pool_id) {
        return Err(ContractError::PoolNotFound { pool_id });
    }
    POOLS.remove(deps.storage, pool_id);

    Ok(Response::new()
        .add_attribute("action", "remove_pool")
        .add_attribute("pool_id", pool_id.to_string()))
}

pub fn execute_update_scaling_factor(
    deps: DepsMut,
    env: Env,
    pool_id: u64,
) -> Result<Response, ContractError> {
    if !POOLS.has(deps.storage, pool_id) {
        return Err(ContractError::PoolNotFound { pool_id });
    }
    let mut pool = POOLS.load(deps.storage, pool_id)?;

    let lsr_contract_address = &CONFIG.load(deps.storage)?.lsr_contract_address;

    let redemption_rate_query_msg = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lsr_contract_address.to_string(),
        msg: to_json_binary(&LiquidStakeRateQueryMsg::RedemptionRate {
            denom: pool.ibc_hash_stk_denom.clone(),
            params: None,
        })?,
    });

    let redemption_rate_response: RedemptionRateResponse = deps
        .querier
        .query(&redemption_rate_query_msg)
        .map_err(|err| ContractError::UnableToQueryRedemptionRate {
            stk_denom: pool.ibc_hash_stk_denom.clone(),
            error: err.to_string(),
        })?;

    let redemption_rate = redemption_rate_response.redemption_rate;
    let scaling_factors =
        convert_redemption_rate_to_scaling_factors(redemption_rate, pool.asset_ordering.clone());

    let adjust_factors_msg: CosmosMsg = MsgStableSwapAdjustScalingFactors {
        sender: env.contract.address.to_string(),
        pool_id,
        scaling_factors: scaling_factors.clone(),
    }
    .into();

    pool.last_updated = env.block.time.seconds();
    POOLS.save(deps.storage, pool_id, &pool)?;

    Ok(Response::new()
        .add_attribute("action", "update_scaling_factor")
        .add_attribute("pool_id", pool_id.to_string())
        .add_attribute("redemption_rate", redemption_rate.to_string())
        .add_attribute(
            "scaling_factors",
            format!("[{}, {}]", scaling_factors[0], scaling_factors[1]),
        )
        .add_message(adjust_factors_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Pool { pool_id } => to_json_binary(&POOLS.load(deps.storage, pool_id)?),
        QueryMsg::AllPools {} => to_json_binary(&query_all_pools(deps)?),
    }
}

/// Queries the list of all pools controlled by the contract
pub fn query_all_pools(deps: Deps) -> StdResult<Pools> {
    let pools: Vec<Pool> = POOLS
        .range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| item.ok().map(|(_, pool)| pool))
        .collect();

    Ok(Pools { pools })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::vec;

    use super::*;

    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        attr, from_json, to_json_binary, Addr, CosmosMsg, Decimal, Empty, Env, MessageInfo,
        OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult, Timestamp,
        WasmQuery,
    };
    use osmosis_std::types::cosmos::base::v1beta1::Coin;
    use osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::{
        MsgStableSwapAdjustScalingFactors, Pool as StableswapPool,
    };
    use osmosis_std::types::osmosis::poolmanager::v1beta1::PoolRequest;
    use prost::Message;
    use serde::{Deserialize, Serialize};

    use crate::contract::{execute, instantiate, query};
    use crate::state::{AssetOrdering, Config, Pool};
    use crate::ContractError;

    const ADMIN_ADDRESS: &str = "admin";
    const LSR_CONTRACT_ADDRESS: &str = "lsr";

    const OSMOSIS_POOL_QUERY_TYPE: &str = "/osmosis.poolmanager.v1beta1.Query/Pool";

    pub struct WasmMockQuerier {
        base_querier: MockQuerier<Empty>,
        lsr_redemption_rates: HashMap<String, RedemptionRateResponse>,
        pools: HashMap<u64, PoolQueryResponse>,
    }

    // Custom Osmosis pool query response to get avoid Any proto type
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    pub struct PoolQueryResponse {
        pub pool: StableswapPool,
    }

    // Implements the Querier trait to be used as a MockQuery object
    impl Querier for WasmMockQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            let request: QueryRequest<Empty> = match from_json(bin_request) {
                Ok(v) => v,
                Err(e) => {
                    return SystemResult::Err(SystemError::InvalidRequest {
                        error: format!("Parsing query request: {}", e),
                        request: bin_request.into(),
                    })
                }
            };
            self.handle_query(&request)
        }
    }

    impl WasmMockQuerier {
        pub fn new() -> Self {
            WasmMockQuerier {
                base_querier: MockQuerier::new(&[]),
                lsr_redemption_rates: HashMap::new(),
                pools: HashMap::new(),
            }
        }

        // The only supported queries are oracle redemption rate queries (to the oracle contract address)
        // stargate pool queries, or generic base queries
        pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
            match &request {
                QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                    if contract_addr == LSR_CONTRACT_ADDRESS {
                        match from_json(msg).unwrap() {
                            LiquidStakeRateQueryMsg::RedemptionRate { denom, .. } => {
                                match self.lsr_redemption_rates.get(&denom) {
                                    Some(resp) => SystemResult::Ok(to_json_binary(&resp).into()),
                                    None => SystemResult::Err(SystemError::Unknown {}),
                                }
                            }
                        }
                    } else {
                        panic!("Mocked query not supported for contract {}", contract_addr);
                    }
                }
                QueryRequest::Stargate { path, data } => {
                    if path == OSMOSIS_POOL_QUERY_TYPE {
                        let pool_request: PoolRequest = Message::decode(data.as_slice()).unwrap();
                        match self.pools.get(&pool_request.pool_id) {
                            Some(resp) => SystemResult::Ok(to_json_binary(&resp).into()),
                            None => SystemResult::Err(SystemError::Unknown {}),
                        }
                    } else {
                        panic!("Mocked query not supported for stargate path {}", path);
                    }
                }
                _ => self.base_querier.handle_query(request),
            }
        }

        // Adds a mocked entry to the querier such that queries with the specified denom
        // return a query response with the given redemption rate
        pub fn mock_lsr_redemption_rate(&mut self, denom: String, c_value: Decimal) {
            self.lsr_redemption_rates.insert(
                denom,
                RedemptionRateResponse {
                    redemption_rate: c_value,
                    update_time: 1,
                },
            );
        }

        // Adds a mocked entry to the querier such that queries with the specified pool ID
        // return a stableswap pool with specified liquidity
        pub fn mock_stableswap_pool(&mut self, pool_id: u64, pool: &Pool) {
            let pool_assets = match pool.asset_ordering {
                AssetOrdering::StkTokenFirst => {
                    vec![pool.ibc_hash_stk_denom.clone(), "native_denom".to_string()]
                }
                AssetOrdering::NativeTokenFirst => {
                    vec!["native_denom".to_string(), pool.ibc_hash_stk_denom.clone()]
                }
            };

            let pool_liquidity = pool_assets
                .into_iter()
                .map(|denom| Coin {
                    amount: "1000000".to_string(),
                    denom,
                })
                .collect();

            let stableswap_pool = StableswapPool {
                id: pool_id,
                pool_liquidity,
                scaling_factor_controller: mock_env().contract.address.to_string(),
                ..Default::default()
            };

            self.pools.insert(
                pool_id,
                PoolQueryResponse {
                    pool: stableswap_pool,
                },
            );
        }

        // Helper function for if we want to explicitly set a pool that's misconfigured
        pub fn mock_invalid_stableswap_pool(&mut self, pool_id: u64, pool: StableswapPool) {
            self.pools.insert(pool_id, PoolQueryResponse { pool });
        }
    }

    // Helper function to instantiate the contract using the default admin and oracle addresses
    fn default_instantiate() -> (
        OwnedDeps<MockStorage, MockApi, WasmMockQuerier, Empty>,
        Env,
        MessageInfo,
    ) {
        let env = mock_env();
        let info = mock_info(ADMIN_ADDRESS, &[]);

        let custom_querier: WasmMockQuerier = WasmMockQuerier::new();

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: custom_querier,
            custom_query_type: Default::default(),
        };

        let msg = InstantiateMsg {
            owner_address: ADMIN_ADDRESS.to_string(),
            lsr_contract_address: LSR_CONTRACT_ADDRESS.to_string(),
        };

        let resp = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            resp.attributes,
            vec![
                attr("action", "instantiate"),
                attr("owner_address", ADMIN_ADDRESS.to_string()),
                attr("lsr_contract_address", LSR_CONTRACT_ADDRESS.to_string()),
            ]
        );

        (deps, env, info)
    }

    // Helper function to create a test Pool object
    fn get_test_pool(
        pool_id: u64,
        stk_token_denom: &str,
        transfer_port_id: &str,
        transfer_channel_id: &str,
        asset_ordering: AssetOrdering,
    ) -> Pool {
        let ibc_hash_stk_denom =
            denom_trace_to_hash(stk_token_denom, transfer_port_id, transfer_channel_id).unwrap();
        Pool {
            pool_id,
            stk_token_denom: stk_token_denom.to_string(),
            transfer_port_id: transfer_port_id.to_string(),
            transfer_channel_id: transfer_channel_id.to_string(),
            ibc_hash_stk_denom: ibc_hash_stk_denom.to_string(),
            asset_ordering,
            last_updated: 0,
        }
    }

    // Helper function to get an add-pool message from a pool object
    fn get_add_pool_msg(pool_id: u64, pool: Pool) -> crate::msg::ExecuteMsg {
        ExecuteMsg::AddPool {
            pool_id,
            stk_token_denom: pool.stk_token_denom,
            transfer_port_id: "transfer".to_string(),
            transfer_channel_id: "channel-0".to_string(),
            asset_ordering: pool.asset_ordering,
        }
    }

    #[test]
    fn test_instantiate() {
        let (deps, env, _) = default_instantiate();

        // Confirm addresses were set properly
        let msg = QueryMsg::Config {};
        let resp = query(deps.as_ref(), env, msg).unwrap();
        let config: Config = from_json(resp).unwrap();
        assert_eq!(
            config,
            Config {
                owner_address: Addr::unchecked(ADMIN_ADDRESS.to_string()),
                lsr_contract_address: Addr::unchecked(LSR_CONTRACT_ADDRESS.to_string())
            }
        )
    }

    #[test]
    fn test_update_config() {
        let (mut deps, env, info) = default_instantiate();

        // Update the admin and oracle addresses
        let updated_admin = "update_admin";
        let updated_lsr_contract = "updated_lsr_contract";

        let update_msg = ExecuteMsg::UpdateConfig {
            owner_address: updated_admin.to_string(),
            lsr_contract_address: updated_lsr_contract.to_string(),
        };
        let resp = execute(deps.as_mut(), env.clone(), info, update_msg).unwrap();
        assert_eq!(
            resp.attributes,
            vec![
                attr("action", "update_config"),
                attr("owner_address", updated_admin.to_string()),
                attr("lsr_contract_address", updated_lsr_contract.to_string()),
            ]
        );

        // Confirm config updated
        let query_config_msg = QueryMsg::Config {};
        let query_resp = query(deps.as_ref(), env, query_config_msg).unwrap();
        let updated_config: Config = from_json(query_resp).unwrap();
        assert_eq!(
            updated_config,
            Config {
                owner_address: Addr::unchecked(updated_admin.to_string()),
                lsr_contract_address: Addr::unchecked(updated_lsr_contract.to_string())
            }
        )
    }

    #[test]
    fn test_add_remove_pools() {
        let (mut deps, env, info) = default_instantiate();

        // Create 3 dummy pools
        let pool1 = get_test_pool(
            1,
            "stkA",
            "transfer",
            "channel-0",
            AssetOrdering::StkTokenFirst,
        );
        let pool2 = get_test_pool(
            2,
            "stB",
            "transfer",
            "channel-0",
            AssetOrdering::NativeTokenFirst,
        );
        let pool3 = get_test_pool(
            3,
            "stC",
            "transfer",
            "channel-0",
            AssetOrdering::StkTokenFirst,
        );

        // Mock each pool in the querier
        deps.querier.mock_stableswap_pool(1, &pool1);
        deps.querier.mock_stableswap_pool(2, &pool2);
        deps.querier.mock_stableswap_pool(3, &pool3);

        // Add each pool, and confirm the attributes and pool-query for each
        for pool in vec![pool1.clone(), pool2.clone(), pool3.clone()] {
            let add_msg = get_add_pool_msg(pool.pool_id, pool.clone());
            let add_msg_resp = execute(deps.as_mut(), env.clone(), info.clone(), add_msg).unwrap();

            assert_eq!(
                add_msg_resp.attributes,
                vec![
                    attr("action", "add_pool"),
                    attr("pool_id", pool.pool_id.to_string()),
                    attr("pool_stk_token_denom", pool.ibc_hash_stk_denom.clone()),
                    attr("pool_asset_ordering", pool.asset_ordering.to_string()),
                ]
            );

            let query_pool_msg = QueryMsg::Pool {
                pool_id: pool.pool_id,
            };
            let query_resp = query(deps.as_ref(), env.clone(), query_pool_msg).unwrap();
            let pool_resp: Pool = from_json(query_resp).unwrap();

            assert_eq!(pool_resp, pool);
        }

        // Test the AllPools query
        let all_pools_query_msg = QueryMsg::AllPools {};
        let query_pools_resp = query(deps.as_ref(), env.clone(), all_pools_query_msg).unwrap();
        let all_pools_resp: Pools = from_json(query_pools_resp).unwrap();

        assert_eq!(
            all_pools_resp,
            Pools {
                pools: vec![pool1.clone(), pool2, pool3.clone()]
            }
        );

        // Remove pool 2
        let remove_pool_msg = ExecuteMsg::RemovePool { pool_id: 2 };
        let remove_pool_resp =
            execute(deps.as_mut(), env.clone(), info.clone(), remove_pool_msg).unwrap();

        assert_eq!(
            remove_pool_resp.attributes,
            vec![attr("action", "remove_pool"), attr("pool_id", "2")]
        );

        // Query AllPools again, it should only return pools 1 and 3
        let all_pools_query_msg = QueryMsg::AllPools {};
        let query_pools_resp = query(deps.as_ref(), env.clone(), all_pools_query_msg).unwrap();
        let all_pools_resp: Pools = from_json(query_pools_resp).unwrap();

        assert_eq!(
            all_pools_resp,
            Pools {
                pools: vec![pool1, pool3]
            }
        );

        // Try to add pool 1 again, it should fail
        let add_duplicate_pool_msg = ExecuteMsg::AddPool {
            pool_id: 1,
            stk_token_denom: "".to_string(),
            transfer_port_id: "".to_string(),
            transfer_channel_id: "".to_string(),
            asset_ordering: AssetOrdering::StkTokenFirst,
        };
        let add_duplicate_pool_resp = execute(deps.as_mut(), env, info, add_duplicate_pool_msg);
        assert_eq!(
            add_duplicate_pool_resp,
            Err(ContractError::PoolAlreadyExists { pool_id: 1 })
        )
    }

    #[test]
    fn test_add_misconfigured_pool_id_mismatch() {
        let (mut deps, env, info) = default_instantiate();

        let queried_id = 1;
        let misconfigured_pool_id = 999;

        // Create a pool configuration and message
        let pool = get_test_pool(
            queried_id,
            "stk_token",
            "transfer",
            "channel-0",
            AssetOrdering::StkTokenFirst,
        );
        let add_msg = get_add_pool_msg(queried_id, pool.clone());

        // Mock out the query response so that the returned pool has a different pool ID
        deps.querier.mock_invalid_stableswap_pool(
            queried_id,
            StableswapPool {
                id: misconfigured_pool_id,
                scaling_factor_controller: env.contract.address.to_string(),
                ..Default::default()
            },
        );

        // Attempt to add the pool, it should error since the ID does not match
        let resp = execute(deps.as_mut(), env.clone(), info.clone(), add_msg);
        assert_eq!(
            resp,
            Err(ContractError::PoolNotFoundOsmosis {
                pool_id: queried_id
            })
        );
    }

    #[test]
    fn test_add_misconfigured_pool_number_of_assets() {
        let (mut deps, env, info) = default_instantiate();

        let pool_id = 1;
        let pool = get_test_pool(
            pool_id,
            "stk_token",
            "transfer",
            "channel-0",
            AssetOrdering::StkTokenFirst,
        );
        let add_msg = get_add_pool_msg(pool_id, pool.clone());

        deps.querier.mock_invalid_stableswap_pool(
            pool_id,
            StableswapPool {
                id: pool_id,
                pool_liquidity: vec![
                    Coin {
                        denom: "denom1".to_string(),
                        amount: "1000000".to_string(),
                    },
                    Coin {
                        denom: "denom2".to_string(),
                        amount: "1000000".to_string(),
                    },
                    Coin {
                        denom: "denom3".to_string(),
                        amount: "1000000".to_string(),
                    },
                ],
                scaling_factor_controller: env.contract.address.to_string(),
                ..Default::default()
            },
        );

        let resp = execute(deps.as_mut(), env.clone(), info.clone(), add_msg);
        assert_eq!(
            resp,
            Err(ContractError::InvalidNumberOfPoolAssets { number: 3 })
        );
    }

    #[test]
    fn test_add_misconfigured_pool_asset_ordering() {
        let (mut deps, env, info) = default_instantiate();

        let pool1 = get_test_pool(
            1,
            "stk_token",
            "transfer",
            "channel-0",
            AssetOrdering::StkTokenFirst,
        );
        let pool2 = get_test_pool(
            2,
            "stk_token",
            "transfer",
            "channel-0",
            AssetOrdering::NativeTokenFirst,
        );

        deps.querier.mock_stableswap_pool(1, &pool1);
        deps.querier.mock_stableswap_pool(2, &pool2);

        let add_msg1 = get_add_pool_msg(2, pool1.clone());
        let add_msg2 = get_add_pool_msg(1, pool2.clone());

        let add_resp1 = execute(deps.as_mut(), env.clone(), info.clone(), add_msg1);
        assert_eq!(add_resp1, Err(ContractError::InvalidPoolAssetOrdering {}));

        let add_resp2 = execute(deps.as_mut(), env.clone(), info.clone(), add_msg2);
        assert_eq!(add_resp2, Err(ContractError::InvalidPoolAssetOrdering {}));
    }

    #[test]
    fn test_unauthorized() {
        let (mut deps, env, _) = default_instantiate();

        let invalid_info: MessageInfo = mock_info("not_admin", &[]);

        let pool_id = 1;
        let pool = get_test_pool(
            pool_id,
            "stkA",
            "transfer",
            "channel-0",
            AssetOrdering::StkTokenFirst,
        );
        let add_msg = get_add_pool_msg(pool_id, pool);
        let add_resp = execute(deps.as_mut(), env.clone(), invalid_info.clone(), add_msg);

        assert_eq!(add_resp, Err(ContractError::Unauthorized {}));

        let remove_msg = ExecuteMsg::RemovePool { pool_id: 1 };
        let remove_resp = execute(deps.as_mut(), env.clone(), invalid_info.clone(), remove_msg);

        assert_eq!(remove_resp, Err(ContractError::Unauthorized {}));
    }

    #[test]
    fn test_update_scaling_factor() {
        let pool_id = 2;
        let stk_token_denom = "stk/uosmo";
        let asset_ordering = AssetOrdering::StkTokenFirst;
        let pool = get_test_pool(
            pool_id,
            stk_token_denom,
            "transfer",
            "channel-0",
            asset_ordering,
        );

        let block_time = 1_000_000;
        let redemption_rate = Decimal::from_str("1.2").unwrap();
        let expected_scaling_factors = vec![100000, 120000];

        let (mut deps, mut env, info) = default_instantiate();
        env.block.time = Timestamp::from_seconds(block_time);
        deps.querier
            .mock_lsr_redemption_rate(pool.ibc_hash_stk_denom.to_string(), redemption_rate);

        deps.querier.mock_stableswap_pool(pool_id, &pool);

        let add_pool_msg = get_add_pool_msg(pool_id, pool);
        execute(deps.as_mut(), env.clone(), info.clone(), add_pool_msg).unwrap();

        let update_msg = ExecuteMsg::UpdateScalingFactor { pool_id: 2 };
        let update_pool_resp =
            execute(deps.as_mut(), env.clone(), info.clone(), update_msg).unwrap();

        assert_eq!(
            update_pool_resp.attributes,
            vec![
                attr("action", "update_scaling_factor"),
                attr("pool_id", "2"),
                attr("redemption_rate", "1.2"),
                attr("scaling_factors", "[100000, 120000]")
            ]
        );

        let query_pool_msg = QueryMsg::Pool { pool_id: 2 };
        let query_pool_resp = query(deps.as_ref(), env.clone(), query_pool_msg).unwrap();

        let queried_pool: Pool = from_json(query_pool_resp).unwrap();
        let queried_pool_cloned = queried_pool.clone();
        let expected_pool = Pool {
            last_updated: block_time,
            ..queried_pool_cloned
        };

        assert_eq!(queried_pool, expected_pool);

        let expected_update_msg: CosmosMsg = MsgStableSwapAdjustScalingFactors {
            sender: env.contract.address.to_string(),
            pool_id: 2,
            scaling_factors: expected_scaling_factors,
        }
        .into();

        assert_eq!(update_pool_resp.messages.len(), 1);
        assert_eq!(update_pool_resp.messages[0].msg, expected_update_msg);

        let update_msg = ExecuteMsg::UpdateScalingFactor { pool_id: 1 };
        let update_pool_resp = execute(deps.as_mut(), env.clone(), info, update_msg);
        assert_eq!(
            update_pool_resp,
            Err(ContractError::PoolNotFound { pool_id: 1 })
        );
    }
}
