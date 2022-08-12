use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
}

// address -> withdrawable amount mapping
pub const AMOUNTS: Map<Addr, u128> = Map::new("amount");

// total fees collected
pub const FEE: Item<u128> = Item::new("fee");

// State to keep track of owner
pub const STATE: Item<State> = Item::new("state");
