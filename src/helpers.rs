use crate::ContractError;
use cosmwasm_std::Coin;

// validate if the funded coin is usei and return it
pub fn validate_and_extract_coin(sent_funds: &[Coin]) -> Result<Coin, ContractError> {
    if sent_funds.len() != 1 {
        return Err(ContractError::WrongCoinSent {});
    }
    if sent_funds[0].denom.ne("usei") {
        return Err(ContractError::WrongFundCoin {
            expected: String::from("usei"),
            got: sent_funds[0].denom.clone(),
        });
    }
    Ok(sent_funds[0].clone())
}
