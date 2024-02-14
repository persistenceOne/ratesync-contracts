use std::str::FromStr;

use cosmwasm_std::{Addr, Api, Decimal, Deps, StdResult};
use sha2::{Digest, Sha256};

use crate::{
    lsr_error::ContractError,
    lsr_state::{RedemptionRate, CONFIG, LIQUID_STAKE_RATES},
};

const CHANNEL_ID_PERFIX: &str = "channel";

/// This helper function is to validate an optional string passed for address
pub fn option_string_to_addr(
    api: &dyn Api,
    option_string: Option<String>,
    default: Addr,
) -> StdResult<Addr> {
    match option_string {
        Some(input_addr) => api.addr_validate(&input_addr),
        None => Ok(default),
    }
}

pub fn validate_native_denom(denom: &str) -> Result<(), ContractError> {
    if denom.len() < 3 || denom.len() > 128 {
        return Err(ContractError::InvalidDenom {
            reason: "Invalid denom length".to_string(),
        });
    }

    let mut chars = denom.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() {
        return Err(ContractError::InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string(),
        });
    }

    let set = ['/', ':', '.', '_', '-'];
    for c in chars {
        if !(c.is_ascii_alphanumeric() || set.contains(&c)) {
            return Err(ContractError::InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string(),
            });
        }
    }

    Ok(())
}

pub fn validate_redemption_rate(deps: Deps, rr: RedemptionRate) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let c_value_rates = LIQUID_STAKE_RATES
        .load(deps.storage, &rr.denom)?
        .get_latest_range(config.count_limit as usize);
    let moving_average = calculate_average_redemption_rate(c_value_rates)?;
    let deviation = moving_average.abs_diff(rr.redemption_rate);

    if moving_average != Decimal::zero() && deviation > config.threshold {
        return Err(ContractError::InvalidCValueDeviation { value: deviation });
    }

    Ok(())
}

fn calculate_average_redemption_rate(
    redemption_rates: Vec<RedemptionRate>,
) -> Result<Decimal, ContractError> {
    let count = Decimal::from_str(&redemption_rates.len().to_string())?;

    if count.is_zero() {
        Ok(Decimal::zero())
    } else {
        let total_redemption_rate: Decimal =
            redemption_rates.iter().map(|rr| rr.redemption_rate).sum();
        Ok(total_redemption_rate / count)
    }
}

// Validates that the channel ID is of the form `channel-N`
pub fn validate_channel_id(channel_id: &str) -> Result<(), ContractError> {
    let Some((prefix, id)) = channel_id.split_once('-') else {
        return Err(ContractError::InvalidChannelID {
            channel_id: channel_id.to_string(),
        });
    };

    if prefix != CHANNEL_ID_PERFIX {
        return Err(ContractError::InvalidChannelID {
            channel_id: channel_id.to_string(),
        });
    }

    if id.parse::<u64>().is_err() {
        return Err(ContractError::InvalidChannelID {
            channel_id: channel_id.to_string(),
        });
    }

    Ok(())
}

// Given a base denom and channelID, returns the IBC denom hash
// E.g. base_denom: uosmo, channel_id: channel-0 => ibc/{hash(transfer/channel-0/uosmo)}
// Note: This function only supports ibc denom's that originated on the controller chain
pub fn denom_trace_to_hash(
    base_denom: &str,
    transfer_port_id: &str,
    channel_id: &str,
) -> Result<String, ContractError> {
    if base_denom.starts_with("ibc/") {
        return Err(ContractError::InvalidRedemptionRateDenom {
            denom: base_denom.to_string(),
        });
    }

    let denom_trace = format!("{transfer_port_id}/{channel_id}/{base_denom}");

    let mut hasher = Sha256::new();
    hasher.update(denom_trace.as_bytes());
    let result = hasher.finalize();
    let hash = hex::encode(result);

    let ibc_hash = format!("ibc/{}", hash.to_uppercase());
    Ok(ibc_hash)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Addr;

    use crate::{
        lsr_error::ContractError,
        lsr_helpers::{
            denom_trace_to_hash, option_string_to_addr, validate_channel_id, validate_native_denom,
        },
    };

    #[test]
    fn validate_option_string_to_addr() {
        let deps = mock_dependencies();
        let res = option_string_to_addr(&deps.api, None, Addr::unchecked("cosmos2"));
        assert_eq!(res, Ok(Addr::unchecked("cosmos2")),);
    }

    #[test]
    fn length_below_three() {
        let res = validate_native_denom("su");
        assert_eq!(
            res,
            Err(ContractError::InvalidDenom {
                reason: "Invalid denom length".to_string()
            }),
        )
    }

    #[test]
    fn length_above_128() {
        let res =
            validate_native_denom("yzfozgkmmynosebgnltjxisgmotytxnslobsntrcwlszpuafznkgyfqbuflbianinezllsguewvmqunvikjkvnrudeffplzprgefubrmspbvcsbxkibhquuxralneiczy");
        assert_eq!(
            res,
            Err(ContractError::InvalidDenom {
                reason: "Invalid denom length".to_string()
            }),
        )
    }

    #[test]
    fn first_char_not_alphabetical() {
        let res = validate_native_denom("9kjhtwkurkm");
        assert_eq!(
            res,
            Err(ContractError::InvalidDenom {
                reason: "First character is not ASCII alphabetic".to_string()
            }),
        )
    }

    #[test]
    fn invalid_character() {
        let res = validate_native_denom("fakjfh&asd!#");
        assert_eq!(
            res,
            Err(ContractError::InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string()
            }),
        )
    }

    #[test]
    fn correct_denom() {
        let res = validate_native_denom("umars");
        assert_eq!(res, Ok(()));

        let res = validate_native_denom(
            "ibc/NXH1JLDU56SGDRE3DUPTS45AN76QZEM604USXVFXDVYF9AUHD6G93ZC8GE0T0QQU",
        );
        assert_eq!(res, Ok(()));
    }

    #[test]
    fn test_validate_channel_id() {
        assert_eq!(validate_channel_id("channel-0"), Ok(()));
        assert_eq!(validate_channel_id("channel-100"), Ok(()));
        assert_eq!(validate_channel_id("channel-999"), Ok(()));

        assert_eq!(
            validate_channel_id("channel-"),
            Err(ContractError::InvalidChannelID {
                channel_id: "channel-".to_string()
            })
        );

        assert_eq!(
            validate_channel_id("chan-0"),
            Err(ContractError::InvalidChannelID {
                channel_id: "chan-0".to_string()
            })
        );

        assert_eq!(
            validate_channel_id("Xchannel-0"),
            Err(ContractError::InvalidChannelID {
                channel_id: "Xchannel-0".to_string()
            })
        );

        assert_eq!(
            validate_channel_id("channel-0X"),
            Err(ContractError::InvalidChannelID {
                channel_id: "channel-0X".to_string()
            })
        );
    }

    #[test]
    fn test_denom_trace_to_hash() {
        assert_eq!(
            denom_trace_to_hash("uatom", "transfer", "channel-0"),
            Ok("ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2".to_string()),
        );

        assert_eq!(
            denom_trace_to_hash("uatom", "transfer", "channel-3"),
            Ok("ibc/A4DB47A9D3CF9A068D454513891B526702455D3EF08FB9EB558C561F9DC2B701".to_string()),
        );
    }
}
