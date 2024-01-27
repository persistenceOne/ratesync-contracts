use cosmwasm_std::{Decimal, Deps, QueryRequest};

use crate::{state::AssetOrdering, ContractError};
use osmosis_std::types::{
    ibc::applications::transfer::v1::{QueryDenomTraceRequest, QueryDenomTraceResponse},
    osmosis::gamm::poolmodels::stableswap::v1beta1::Pool as StableswapPool,
};

pub const DENOM_TRACE_QUERY_TYPE: &str = "/ibc.applications.transfer.v1.Query/DenomTrace";

pub fn convert_redemption_rate_to_scaling_factors(
    redemption_rate: Decimal,
    asset_ordering: AssetOrdering,
) -> Vec<u64> {
    let multiplier_int: u64 = 100_000;
    let multiplier_dec = Decimal::from_ratio(multiplier_int, 1u64);
    let scaling_factor = (redemption_rate * multiplier_dec).to_uint_floor().u128() as u64;

    match asset_ordering {
        AssetOrdering::StkTokenFirst => vec![multiplier_int, scaling_factor],
        AssetOrdering::NativeTokenFirst => vec![scaling_factor, multiplier_int],
    }
}

pub fn validate_pool_configuration(
    deps: Deps,
    stableswap_pool: StableswapPool,
    pool_id: u64,
    stk_token_denom: String,
    asset_ordering: AssetOrdering,
) -> Result<(), ContractError> {
    if pool_id != stableswap_pool.id {
        return Err(ContractError::PoolNotFoundOsmosis { pool_id });
    }
    if stableswap_pool.pool_liquidity.len() != 2 {
        return Err(ContractError::InvalidNumberOfPoolAssets {
            number: stableswap_pool.pool_liquidity.len() as u64,
        });
    }

    let expected_stk_token_index: usize = match asset_ordering {
        AssetOrdering::StkTokenFirst => 0,
        _ => 1,
    };
    let pool_base_denom = get_denom_trace(
        deps,
        stableswap_pool.pool_liquidity[expected_stk_token_index]
            .denom
            .clone(),
    )?;
    if stk_token_denom != pool_base_denom {
        return Err(ContractError::InvalidPoolAssetOrdering {});
    }

    Ok(())
}

pub fn get_denom_trace(deps: Deps, ibc_denom: String) -> Result<String, ContractError> {
    let query_denom_trace_request = QueryDenomTraceRequest {
        hash: ibc_denom.clone(),
    };
    let query_denom_trace_response: QueryDenomTraceResponse =
        deps.querier.query(&QueryRequest::Stargate {
            path: DENOM_TRACE_QUERY_TYPE.to_string(),
            data: query_denom_trace_request.into(),
        })?;

    let base_denom = match query_denom_trace_response.denom_trace {
        Some(denom_trace) => denom_trace.base_denom,
        None => {
            return Err(ContractError::InvalidDenom { denom: ibc_denom });
        }
    };

    Ok(base_denom)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::vec;

    use cosmwasm_std::testing::{MockApi, MockStorage};
    use cosmwasm_std::{
        from_json, to_json_binary, Decimal, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest,
        StdError, SystemError, SystemResult,
    };
    use osmosis_std::types::ibc::applications::transfer::v1::{DenomTrace, QueryDenomTraceRequest};
    use osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::Pool as StableswapPool;
    use osmosis_std::types::{
        cosmos::base::v1beta1::Coin, ibc::applications::transfer::v1::QueryDenomTraceResponse,
    };
    use prost::Message;

    use crate::{
        helpers::convert_redemption_rate_to_scaling_factors, state::AssetOrdering, ContractError,
    };

    use super::{validate_pool_configuration, DENOM_TRACE_QUERY_TYPE};

    fn get_test_stableswap_pool(pool_id: u64, liquidity_denoms: Vec<&str>) -> StableswapPool {
        let pool_liquidity = liquidity_denoms
            .into_iter()
            .map(|denom| Coin {
                denom: denom.to_string(),
                amount: "100000".to_string(),
            })
            .collect();

        StableswapPool {
            id: pool_id,
            pool_liquidity,
            ..Default::default()
        }
    }

    pub struct WasmMockQuerier {
        pub denom_trace: HashMap<String, QueryDenomTraceResponse>,
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
                denom_trace: HashMap::new(),
            }
        }

        fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
            match request {
                QueryRequest::Stargate { path, data } => {
                    if path == DENOM_TRACE_QUERY_TYPE {
                        let query_denom_trace_request =
                            QueryDenomTraceRequest::decode(data.as_slice()).unwrap();
                        match self.denom_trace.get(&query_denom_trace_request.hash) {
                            Some(resp) => SystemResult::Ok(to_json_binary(&resp).into()),
                            None => SystemResult::Err(SystemError::Unknown {}),
                        }
                    } else {
                        panic!("Mocked query not supported for stargate path {}", path);
                    }
                }
                _ => panic!("DO NOT ENTER HERE"),
            }
        }

        pub fn mock_denom_trace(&mut self, ibc_hash: String) {
            self.denom_trace.insert(
                ibc_hash.clone(),
                QueryDenomTraceResponse {
                    denom_trace: Some(DenomTrace {
                        path: "transfer/channel-0".to_string(),
                        base_denom: ibc_hash.split("/").last().unwrap().to_string(),
                    }),
                },
            );
        }
    }

    fn set_querier() -> Result<OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, ContractError> {
        let custom_querier: WasmMockQuerier = WasmMockQuerier::new();

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: custom_querier,
            custom_query_type: Default::default(),
        };

        deps.querier.mock_denom_trace("ibc/stk_token".to_string());
        deps.querier.mock_denom_trace("ibc/native".to_string());

        Ok(deps)
    }

    #[test]
    fn test_convert_to_scaling_factor_integer() {
        let redemption_rate = Decimal::from_str("1.0").unwrap();
        let asset_ordering = AssetOrdering::StkTokenFirst;
        assert_eq!(
            convert_redemption_rate_to_scaling_factors(redemption_rate, asset_ordering),
            vec![100000, 100000],
        );
    }

    #[test]
    fn test_convert_to_scaling_factor_one_decimal() {
        let redemption_rate = Decimal::from_str("1.2").unwrap();
        let asset_ordering = AssetOrdering::NativeTokenFirst;
        assert_eq!(
            convert_redemption_rate_to_scaling_factors(redemption_rate, asset_ordering),
            vec![120000, 100000],
        );
    }

    #[test]
    fn test_convert_to_scaling_factor_two_decimals() {
        let redemption_rate = Decimal::from_str("1.25").unwrap();
        let asset_ordering = AssetOrdering::StkTokenFirst;
        assert_eq!(
            convert_redemption_rate_to_scaling_factors(redemption_rate, asset_ordering),
            vec![100000, 125000],
        );
    }

    #[test]
    fn test_convert_to_scaling_factor_four_decimals() {
        let redemption_rate = Decimal::from_str("1.25236").unwrap();
        let asset_ordering = AssetOrdering::NativeTokenFirst;
        assert_eq!(
            convert_redemption_rate_to_scaling_factors(redemption_rate, asset_ordering),
            vec![125236, 100000],
        );
    }

    #[test]
    fn test_convert_to_scaling_factor_decimal_truncation() {
        let redemption_rate = Decimal::from_str("1.252369923948298234").unwrap();
        let asset_ordering = AssetOrdering::StkTokenFirst;
        assert_eq!(
            convert_redemption_rate_to_scaling_factors(redemption_rate, asset_ordering),
            vec![100000, 125236],
        );
    }

    #[test]
    fn test_convert_to_scaling_factor_lt_one() {
        let redemption_rate = Decimal::from_str("0.9837").unwrap();
        let asset_ordering = AssetOrdering::NativeTokenFirst;
        assert_eq!(
            convert_redemption_rate_to_scaling_factors(redemption_rate, asset_ordering),
            vec![98370, 100000],
        );
    }

    #[test]
    fn test_convert_to_scaling_factor_zero() {
        let redemption_rate = Decimal::from_str("0.0").unwrap();
        let asset_ordering = AssetOrdering::StkTokenFirst;
        assert_eq!(
            convert_redemption_rate_to_scaling_factors(redemption_rate, asset_ordering),
            vec![100000, 0],
        );
    }

    #[test]
    fn test_validate_pool_configuration_valid_stk_token_first() {
        let pool_id = 2;
        let stk_token_base_denom = "stk_token";
        let stk_token_denom = "ibc/stk_token";
        let native_denom = "native";
        let asset_ordering = AssetOrdering::StkTokenFirst;

        let actual_pool = get_test_stableswap_pool(pool_id, vec![stk_token_denom, native_denom]);

        let deps = set_querier().unwrap();

        assert_eq!(
            validate_pool_configuration(
                deps.as_ref(),
                actual_pool,
                pool_id,
                stk_token_base_denom.to_string(),
                asset_ordering
            ),
            Ok(())
        );
    }

    #[test]
    fn test_validate_pool_configuration_valid_native_token_first() {
        let pool_id = 2;
        let stk_token_base_denom = "stk_token";
        let stk_token_denom = "ibc/stk_token";
        let native_denom = "native";
        let asset_ordering = AssetOrdering::NativeTokenFirst;

        let actual_pool = get_test_stableswap_pool(pool_id, vec![native_denom, stk_token_denom]);

        let deps = set_querier().unwrap();

        assert_eq!(
            validate_pool_configuration(
                deps.as_ref(),
                actual_pool,
                pool_id,
                stk_token_base_denom.to_string(),
                asset_ordering
            ),
            Ok(())
        );
    }

    #[test]
    fn test_validate_pool_configuration_mismatch_pool_id() {
        let configured_pool_id = 2;
        let queried_pool_id = 3;
        let stk_token_denom = "ibc/stk_token";
        let native_denom = "native";
        let asset_ordering = AssetOrdering::StkTokenFirst;

        let actual_pool =
            get_test_stableswap_pool(queried_pool_id, vec![stk_token_denom, native_denom]);

        let deps = set_querier().unwrap();

        assert_eq!(
            validate_pool_configuration(
                deps.as_ref(),
                actual_pool,
                configured_pool_id,
                stk_token_denom.to_string(),
                asset_ordering
            ),
            Err(ContractError::PoolNotFoundOsmosis {
                pool_id: configured_pool_id
            })
        );
    }

    #[test]
    fn test_validate_pool_configuration_invalid_asset_ordering() {
        let pool_id = 2;
        let stk_token_base_denom = "stk_token";
        let stk_token_denom = "ibc/stk_token";
        let native_denom = "native";
        let ibc_native_denom = "ibc/native";

        // Actual pool has native first, configured pool specifies stk_token first
        let configured_ordering = AssetOrdering::StkTokenFirst;
        let actual_pool = get_test_stableswap_pool(pool_id, vec![native_denom, stk_token_denom]);

        let deps = set_querier().unwrap();

        assert_eq!(
            validate_pool_configuration(
                deps.as_ref(),
                actual_pool,
                pool_id,
                stk_token_base_denom.to_string(),
                configured_ordering
            ),
            Err(StdError::generic_err("Querier system error: Unknown system error").into())
        );

        // Actual pool has stk_token first, configured pool specifies native first
        let configured_ordering = AssetOrdering::NativeTokenFirst;
        let actual_pool = get_test_stableswap_pool(pool_id, vec![stk_token_denom, native_denom]);

        assert_eq!(
            validate_pool_configuration(
                deps.as_ref(),
                actual_pool,
                pool_id,
                stk_token_base_denom.to_string(),
                configured_ordering
            ),
            Err(StdError::generic_err("Querier system error: Unknown system error").into())
        );

        // Actual pool has ibc native first, configured pool specifies stk_token first
        let configured_ordering = AssetOrdering::StkTokenFirst;
        let actual_pool =
            get_test_stableswap_pool(pool_id, vec![ibc_native_denom, stk_token_denom]);

        let deps = set_querier().unwrap();

        assert_eq!(
            validate_pool_configuration(
                deps.as_ref(),
                actual_pool,
                pool_id,
                stk_token_base_denom.to_string(),
                configured_ordering
            ),
            Err(ContractError::InvalidPoolAssetOrdering {})
        );

        // Actual pool has stk_token first, configured pool specifies ibc native first
        let configured_ordering = AssetOrdering::NativeTokenFirst;
        let actual_pool =
            get_test_stableswap_pool(pool_id, vec![stk_token_denom, ibc_native_denom]);

        let deps = set_querier().unwrap();

        assert_eq!(
            validate_pool_configuration(
                deps.as_ref(),
                actual_pool,
                pool_id,
                stk_token_base_denom.to_string(),
                configured_ordering
            ),
            Err(ContractError::InvalidPoolAssetOrdering {})
        );
    }
}
