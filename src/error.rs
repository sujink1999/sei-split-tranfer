use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Quantity exceeds withdrawable amount")]
    ExceededQuantity {},

    #[error("Wrong coin sent")]
    WrongCoinSent {},

    #[error("Wrong fund coin (expected: {expected}, got: {got})")]
    WrongFundCoin { expected: String, got: String },

    #[error("Sender is not owner")]
    NotOwner {},
}
