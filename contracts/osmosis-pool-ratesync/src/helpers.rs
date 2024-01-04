use cosmwasm_std::Decimal;

use crate::{state::AssetOrdering, ContractError};
use osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::Pool as StableswapPool;

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
    if stk_token_denom != stableswap_pool.pool_liquidity[expected_stk_token_index].denom {
        return Err(ContractError::InvalidPoolAssetOrdering {});
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::vec;

    use cosmwasm_std::Decimal;
    use osmosis_std::types::cosmos::base::v1beta1::Coin;
    use osmosis_std::types::osmosis::gamm::poolmodels::stableswap::v1beta1::Pool as StableswapPool;

    use crate::{
        helpers::convert_redemption_rate_to_scaling_factors, state::AssetOrdering, ContractError,
    };

    use super::validate_pool_configuration;

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
        let stk_token_denom = "ibc/stk_token";
        let native_denom = "native";
        let asset_ordering = AssetOrdering::StkTokenFirst;

        let actual_pool = get_test_stableswap_pool(pool_id, vec![stk_token_denom, native_denom]);

        assert_eq!(
            validate_pool_configuration(
                actual_pool,
                pool_id,
                stk_token_denom.to_string(),
                asset_ordering
            ),
            Ok(())
        );
    }

    #[test]
    fn test_validate_pool_configuration_valid_native_token_first() {
        let pool_id = 2;
        let stk_token_denom = "ibc/stk_token";
        let native_denom = "native";
        let asset_ordering = AssetOrdering::NativeTokenFirst;

        let actual_pool = get_test_stableswap_pool(pool_id, vec![native_denom, stk_token_denom]);

        assert_eq!(
            validate_pool_configuration(
                actual_pool,
                pool_id,
                stk_token_denom.to_string(),
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

        assert_eq!(
            validate_pool_configuration(
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
        let stk_token_denom = "ibc/stk_token";
        let native_denom = "native";

        // Actual pool has native first, configured pool specifies stk_token first
        let configured_ordering = AssetOrdering::StkTokenFirst;
        let actual_pool = get_test_stableswap_pool(pool_id, vec![native_denom, stk_token_denom]);

        assert_eq!(
            validate_pool_configuration(
                actual_pool,
                pool_id,
                stk_token_denom.to_string(),
                configured_ordering
            ),
            Err(ContractError::InvalidPoolAssetOrdering {})
        );

        // Actual pool has stk_token first, configured pool specifies native first
        let configured_ordering = AssetOrdering::NativeTokenFirst;
        let actual_pool = get_test_stableswap_pool(pool_id, vec![stk_token_denom, native_denom]);

        assert_eq!(
            validate_pool_configuration(
                actual_pool,
                pool_id,
                stk_token_denom.to_string(),
                configured_ordering
            ),
            Err(ContractError::InvalidPoolAssetOrdering {})
        );
    }
}
