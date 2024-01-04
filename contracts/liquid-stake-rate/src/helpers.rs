use cosmwasm_std::{Addr, Api, StdResult};

use crate::ContractError;

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

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Addr;

    use crate::helpers::{option_string_to_addr, validate_native_denom};
    use crate::ContractError;

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
            validate_native_denom("fdtycan4k33uu4ph8hhr0bnjdx94pndw7j09i3jm2afiv0980brzdn1xy7nyky0mfkxwnrtrb6d4vh1vqg2abtwvwzpa3r2ydr0wevp2d7uqqpywrpnq1627id48tm7rt");
        assert_eq!(
            res,
            Err(ContractError::InvalidDenom {
                reason: "Invalid denom length".to_string()
            }),
        )
    }

    #[test]
    fn first_char_not_alphabetical() {
        let res = validate_native_denom("7asdkjnfe7");
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
}
