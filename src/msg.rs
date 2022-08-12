use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::State;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// User can transfer amount to two addresses
    Split { recipient1: Addr, recipient2: Addr },

    /// User can withdraw any amount transferred to his address
    Withdraw { quantity: Option<u128> },

    /// Withdraw fees collected through the transactions
    WithdrawFees {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// The amount withdrawable by the user
    WithdrawableAmount { address: Addr },

    /// Query the owner (creator) of the contract
    OwnerQuery {},
}

pub type OwnerResponse = State;
