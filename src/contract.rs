use std::ops::Div;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::helpers::validate_and_extract_coin;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, AMOUNTS, FEE, STATE};
use cosmwasm_std::{Addr, Coin};

const CONTRACT_NAME: &str = "crates.io:split-transfer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // save the state and initialize fee
    STATE.save(deps.storage, &state)?;
    FEE.save(deps.storage, &0)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Split {
            recipient1,
            recipient2,
        } => split(deps, info, recipient1, recipient2),
        ExecuteMsg::Withdraw { quantity } => withdraw(deps, info, quantity),
        ExecuteMsg::WithdrawFees {} => withdraw_fees(deps, info),
    }
}

fn split(
    deps: DepsMut,
    info: MessageInfo,
    recipient1: Addr,
    recipient2: Addr,
) -> Result<Response, ContractError> {
    let sent_coin = validate_and_extract_coin(&info.funds)?;

    // collect 1% as fee and store it
    let fee = sent_coin.amount.u128().div(100);
    let total_fee = FEE.load(deps.storage)? + fee;
    FEE.save(deps.storage, &total_fee)?;

    // split the amount into two
    let split_amount = (sent_coin.amount.u128() - fee) / 2;

    // if balance already present, update it or else, initialize
    let amount = |d: Option<u128>| -> StdResult<u128> {
        match d {
            Some(old_amount) => Ok(old_amount + split_amount),
            None => Ok(split_amount.clone()),
        }
    };

    AMOUNTS.update(deps.storage, recipient1.clone(), amount)?;
    AMOUNTS.update(deps.storage, recipient2.clone(), amount)?;

    Ok(Response::new().add_attribute("method", "split"))
}

fn withdraw_fees(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // check if the sender is the owner
    if info.sender != STATE.load(deps.storage)?.owner {
        return Err(ContractError::NotOwner {});
    }

    // fetch the collected fee and transfer it to the owner
    let amount = FEE.load(deps.storage)?;
    FEE.save(deps.storage, &0)?;
    return Ok(send_tokens(
        info.sender.clone(),
        vec![coin(amount.into(), "usei")],
        "withdraw",
    ));
}

fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    quantity: Option<u128>,
) -> Result<Response, ContractError> {
    let amount = AMOUNTS.load(deps.storage, info.sender.clone())?;

    // check if quantity is present
    if let Some(quantity) = quantity {
        // check if quantity is valid
        if quantity > amount {
            return Err(ContractError::ExceededQuantity {});
        } else {
            // update the store and send the tokens
            AMOUNTS.save(deps.storage, info.sender.clone(), &(amount - quantity))?;
            return Ok(send_tokens(
                info.sender.clone(),
                vec![coin(quantity.into(), "usei")],
                "withdraw",
            ));
        }
    } else {
        // update the store and send the tokens
        AMOUNTS.remove(deps.storage, info.sender.clone());
        Ok(send_tokens(
            info.sender.clone(),
            vec![coin(amount.into(), "usei")],
            "withdraw",
        ))
    }
}

// this is a helper to move the tokens, so the business logic is easy to read
fn send_tokens(to_address: Addr, amount: Vec<Coin>, action: &str) -> Response {
    Response::new()
        .add_message(BankMsg::Send {
            to_address: to_address.clone().into(),
            amount,
        })
        .add_attribute("action", action)
        .add_attribute("to", to_address)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::OwnerQuery {} => to_binary(&query_state(deps)?),
        QueryMsg::WithdrawableAmount { address } => to_binary(&withdrawable_amount(deps, address)?),
    }
}

fn query_state(deps: Deps) -> StdResult<State> {
    // returns the state
    STATE.load(deps.storage)
}

// returns the withdrawable amount for an address
fn withdrawable_amount(deps: Deps, address: Addr) -> StdResult<u128> {
    let amount = AMOUNTS.may_load(deps.storage, address)?;
    if let Some(amount) = amount {
        return Ok(amount);
    } else {
        return Ok(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::OwnerResponse;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, CosmosMsg};

    // checks if initialization was successful
    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(0, "usei"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::OwnerQuery {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();

        // check if the owner is set after initialization
        assert_eq!(info.sender, value.owner);
    }

    // checks if the sent fund was split and balance of recipient updated
    #[test]
    fn split_transfer() {
        let mut deps = mock_dependencies();

        let instantiate_msg = InstantiateMsg {};
        let creator_info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), creator_info, instantiate_msg).unwrap();

        // send 200 coins from the sender
        let sender_info = mock_info("sender", &coins(200, "usei"));
        let split_msg = ExecuteMsg::Split {
            recipient1: Addr::unchecked("person1"),
            recipient2: Addr::unchecked("person2"),
        };
        let _res = execute(deps.as_mut(), mock_env(), sender_info, split_msg).unwrap();

        // check the balance of recipient 1
        let res_1 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::WithdrawableAmount {
                address: Addr::unchecked("person1"),
            },
        )
        .unwrap();
        let user_1_balance: u128 = from_binary(&res_1).unwrap();
        assert_eq!(99, user_1_balance);

        // check the balance of recipient 2
        let res_2 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::WithdrawableAmount {
                address: Addr::unchecked("person2"),
            },
        )
        .unwrap();
        let user_2_balance: u128 = from_binary(&res_2).unwrap();
        assert_eq!(99, user_2_balance);
    }

    // checks if the old balance is updated for the same recipients
    #[test]
    fn update_old_balance() {
        let mut deps = mock_dependencies();

        let instantiate_msg = InstantiateMsg {};
        let creator_info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), creator_info, instantiate_msg).unwrap();

        // sender sends 200 coins twice to person1 and person2
        let sender_info = mock_info("sender", &coins(200, "usei"));
        let split_msg = ExecuteMsg::Split {
            recipient1: Addr::unchecked("person1"),
            recipient2: Addr::unchecked("person2"),
        };

        let _res1 = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            split_msg.clone(),
        )
        .unwrap();
        let _res2 = execute(
            deps.as_mut(),
            mock_env(),
            sender_info.clone(),
            split_msg.clone(),
        )
        .unwrap();

        // check the updated balance of person2
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::WithdrawableAmount {
                address: Addr::unchecked("person2"),
            },
        )
        .unwrap();
        let user_balance: u128 = from_binary(&res).unwrap();
        assert_eq!(198, user_balance);
    }

    #[test]
    fn withdraw() {
        let mut deps = mock_dependencies();

        let instantiate_msg = InstantiateMsg {};
        let creator_info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), creator_info, instantiate_msg).unwrap();

        // sender sends 200 coins to person1 (99) and person2 (99)
        let sender_info = mock_info("sender", &coins(200, "usei"));
        let split_msg = ExecuteMsg::Split {
            recipient1: Addr::unchecked("person1"),
            recipient2: Addr::unchecked("person2"),
        };
        let _res = execute(deps.as_mut(), mock_env(), sender_info, split_msg).unwrap();

        // person1 withdraws 50 coins
        let user_info = mock_info("person1", &[]);
        let mut msg = ExecuteMsg::Withdraw { quantity: Some(50) };
        let execute_res = execute(deps.as_mut(), mock_env(), user_info.clone(), msg).unwrap();
        assert_eq!(1, execute_res.messages.len());

        let sub_msg = execute_res.messages.get(0).expect("no message");
        assert_eq!(
            sub_msg.msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "person1".into(),
                amount: coins(50, "usei"),
            })
        );

        // checks the balance of person1 after the partial withdraw
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::WithdrawableAmount {
                address: Addr::unchecked("person1"),
            },
        )
        .unwrap();
        let user_balance: u128 = from_binary(&res).unwrap();
        assert_eq!(49, user_balance);

        // person1 withdraws the entire balance (49)
        msg = ExecuteMsg::Withdraw { quantity: None };
        let execute_res = execute(deps.as_mut(), mock_env(), user_info, msg).unwrap();
        assert_eq!(1, execute_res.messages.len());

        let sub_msg = execute_res.messages.get(0).expect("no message");
        assert_eq!(
            sub_msg.msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "person1".into(),
                amount: coins(49, "usei"),
            })
        );

        // checks the balance of person1 after withdrawal
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::WithdrawableAmount {
                address: Addr::unchecked("person1"),
            },
        )
        .unwrap();
        let user_balance: u128 = from_binary(&res).unwrap();
        assert_eq!(0, user_balance);
    }

    // BONUS - The owner can withdraw fees collected from the contract
    #[test]
    fn withdraw_fees() {
        let mut deps = mock_dependencies();

        let instantiate_msg = InstantiateMsg {};
        let creator_info = mock_info("creator", &[]);
        let _res = instantiate(
            deps.as_mut(),
            mock_env(),
            creator_info.clone(),
            instantiate_msg,
        )
        .unwrap();

        // sender sends 200 coin and 2 coins are collected as fees
        let sender_info = mock_info("sender", &coins(200, "usei"));
        let split_msg = ExecuteMsg::Split {
            recipient1: Addr::unchecked("person1"),
            recipient2: Addr::unchecked("person2"),
        };
        let _res = execute(deps.as_mut(), mock_env(), sender_info, split_msg).unwrap();

        // person1 tries to withdraw fees and fails
        let user_info = mock_info("person1", &[]);
        let msg = ExecuteMsg::WithdrawFees {};
        let execute_res = execute(deps.as_mut(), mock_env(), user_info, msg.clone());
        match execute_res.unwrap_err() {
            ContractError::NotOwner { .. } => {}
            e => panic!("unexpected error: {:?}", e),
        }

        // owner withdraws fees from the contract
        let execute_res = execute(deps.as_mut(), mock_env(), creator_info.clone(), msg).unwrap();
        let sub_msg = execute_res.messages.get(0).expect("no message");
        assert_eq!(
            sub_msg.msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "creator".into(),
                amount: coins(2, "usei"),
            })
        );
    }
}
