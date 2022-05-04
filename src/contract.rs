#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
    Storage, Uint128, WasmMsg,
};

use cw2::set_contract_version;
use cw20::{
    AllowanceResponse, BalanceResponse, Cw20Coin, Cw20ExecuteMsg, Cw20ReceiveMsg, Expiration,
};

use crate::allowances::{
    deduct_allowance, execute_burn_from, execute_decrease_allowance, execute_increase_allowance,
    execute_send_from, execute_transfer_from, query_allowance,
};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    ClubOwnershipDetails, ClubPreviousOwnerDetails, ClubStakingDetails, ClubBondingDetails, Config, 
    CLUB_OWNERSHIP_DETAILS, CLUB_PREVIOUS_OWNER_DETAILS, CLUB_STAKING_DETAILS, CLUB_BONDING_DETAILS,
    CONFIG, CONTRACT_WALLET, REWARD,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:club-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAIN_WALLET: &str = "terra1t3czdl5h4w4qwgkzs80fdstj0z7rfv9v2j6uh3";

const CLUB_PRICE: u128 = 1000000000u128;

const INCREASE_STAKE: bool = true;
const DECREASE_STAKE: bool = false;
const IMMEDIATE_WITHDRAWAL: bool = true;
const NO_IMMEDIATE_WITHDRAWAL: bool = false;

// Reward to club owner for buying
const CLUB_BUYING_REWARD_AMOUNT: u128 = 100u128;

// Reward to club staker for staking 
const CLUB_STAKING_REWARD_AMOUNT: u128 = 0u128;

// This is 21 day locking period in seconds, after buying a club 
const CLUB_LOCKING_DURATION: u64 = 1814400u64;

// This is locking period in seconds, after staking in club. 
// No longer applicable so setting it to 0
const CLUB_STAKING_DURATION: u64 = 0u64;

// this is 7 day bonding period in seconds, after withdrawing a stake
const CLUB_BONDING_DURATION: u64 = 604800u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        cw20_token_address: deps.api.addr_validate(&msg.cw20_token_address)?,
        admin_address: deps.api.addr_validate(&msg.admin_address)?,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BuyAClub { buyer, seller, club_name } => {
            buy_a_club(deps, env, info, buyer, seller, club_name, Uint128::from(CLUB_PRICE))
        }
        ExecuteMsg::ReleaseClub { owner, club_name } => {
            release_club(deps, env, info, owner, club_name)
        }
        ExecuteMsg::StakeOnAClub {
            staker,
            club_name,
            amount,
        } => stake_on_a_club(deps, env, info, staker, club_name, amount),
        ExecuteMsg::ClaimOwnerRewards {
            owner,
            club_name,
            amount,
        } => claim_owner_rewards(deps, info, owner, club_name, amount),
        ExecuteMsg::ClaimPreviousOwnerRewards {
            previous_owner,
            club_name,
            amount,
        } => claim_previous_owner_rewards(deps, info, previous_owner, club_name, amount),
        ExecuteMsg::StakeWithdrawFromAClub {
            staker,
            club_name,
            amount,
            immediate_withdrawal,
        } => withdraw_stake_from_a_club(deps, env, info, staker, club_name, amount, immediate_withdrawal),
        ExecuteMsg::SetRewardAmount { amount } => set_reward_amount(deps, info, amount),
        ExecuteMsg::CalculateAndDistributeRewards {} => {
            calculate_and_distribute_rewards(deps, env, info)
        }
        ExecuteMsg::ClaimRewards {
            staker,
            club_name,
            amount,
        } => claim_rewards(deps, info, staker, club_name, amount),
        ExecuteMsg::PeriodicallyRefundStakeouts {} => {
            periodically_refund_stakeouts(deps, env, info)
        }
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::BurnFrom { owner, amount } => execute_burn_from(deps, env, info, owner, amount),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => execute_send_from(deps, env, info, owner, contract, amount, msg),
    }
}

fn claim_previous_owner_rewards(
    deps: DepsMut,
    info: MessageInfo,
    previous_owner: String,
    club_name: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let previous_owner_addr = deps.api.addr_validate(&previous_owner)?;
    //Check if withdrawer is same as invoker
    if previous_owner_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let previous_ownership_details;
    let previous_ownership_details_result = CLUB_PREVIOUS_OWNER_DETAILS.may_load(deps.storage, club_name.clone());
    match previous_ownership_details_result {
        Ok(od) => {
            previous_ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    if !(previous_ownership_details.is_none()) {
        for previous_owner_detail in previous_ownership_details {
            if previous_owner_detail.previous_owner_address == previous_owner.clone() {
                if amount > previous_owner_detail.reward_amount {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: String::from("Insufficient rewards"),
                    }));
                }

                // TODO: Add amount to the owners wallet
                // TODO: Assuming that above operation went good. Need to implement this


                // Now save the previous ownership details
                CLUB_PREVIOUS_OWNER_DETAILS.save(
                    deps.storage,
                    club_name.clone(),
                    &ClubPreviousOwnerDetails {
                        club_name: previous_owner_detail.club_name,
                        previous_owner_address: previous_owner_detail.previous_owner_address,
                        reward_amount: previous_owner_detail.reward_amount - amount,
                    },
                )?;
            }
        }
    }
    return Ok(Response::default());
}

fn claim_owner_rewards (
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
    club_name: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    //Check if withdrawer is same as invoker
    if owner_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    if !(ownership_details.is_none()) {
        for owner_detail in ownership_details {
            if owner_detail.owner_address == owner.clone() {
                if amount > owner_detail.reward_amount {
                    return Err(ContractError::Std(StdError::GenericErr {
                        msg: String::from("Insufficient rewards"),
                    }));
                }

                // TODO: Add amount to the owners wallet
                // TODO: Assuming that above operation went good. Need to implement this

                // Now save the ownership details
                CLUB_OWNERSHIP_DETAILS.save(
                    deps.storage,
                    club_name.clone(),
                    &ClubOwnershipDetails {
                        club_name: owner_detail.club_name,
                        start_timestamp: owner_detail.start_timestamp,
                        locking_period: owner_detail.locking_period,
                        owner_address: owner_detail.owner_address,
                        price_paid: owner_detail.price_paid,
                        reward_amount: owner_detail.reward_amount - amount,
                        owner_released: owner_detail.owner_released,
                    },
                )?;
            }
        }
    }

    return Ok(Response::default());
}

fn periodically_refund_stakeouts(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    //capture the current system time
    let now = env.block.time;

    let distribute_from = String::from(MAIN_WALLET);
    let address = deps.api.addr_validate(distribute_from.clone().as_str())?;

    //Check if the sender (one who is executing this contract) is main
    if address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Fetch all bonding details 
    let all_clubs: Vec<String> = CLUB_BONDING_DETAILS
            .keys(deps.storage, None, None, Order::Ascending)
            .map(|k| String::from_utf8(k).unwrap())
            .collect();
    for club_name in all_clubs {
        let mut all_bonds = Vec::new();
        let bonding_details = CLUB_BONDING_DETAILS.load(deps.storage, club_name.clone())?;
        for mut bond in bonding_details {
            let mut duration = bond.bonding_duration; 
            let now_minus_duration_timestamp = now.minus_seconds(duration);
            if now_minus_duration_timestamp < bond.bonding_start_timestamp {
                all_bonds.push(bond);
            } else {
                // TODO : transfer to staker wallet
            }
        }
        CLUB_BONDING_DETAILS.save(deps.storage, club_name, &all_bonds)?;
    }
    return Ok(Response::default());
}

fn buy_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    buyer: String,
    seller: String,
    club_name: String,
    price: Uint128,
) -> Result<Response, ContractError> {
    let buyer_addr = deps.api.addr_validate(&buyer)?;
    //Check if buyer is same as invoker
    if buyer_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    let mut previous_owners_reward_amount = Uint128::from(0u128);
    if !(ownership_details.is_none()) {
        for owner in ownership_details {
            if owner.owner_released == false {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Owner has not released the club"),
                }));
            }
            else if seller != "".to_string() && owner.owner_address != seller {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Seller is not the owner for the club"),
                }));
            }
            previous_owners_reward_amount = owner.reward_amount;
        }
    }

    // Deduct amount from the buyers wallet
    // TODO: Assuming that above operation went good. Need to implement this

    // Now save the ownership details
    CLUB_OWNERSHIP_DETAILS.save(
        deps.storage,
        club_name.clone(),
        &ClubOwnershipDetails {
            club_name: club_name.clone(),
            start_timestamp: env.block.time,
            locking_period: CLUB_LOCKING_DURATION,
            owner_address: buyer.clone(),
            price_paid: price,
            reward_amount: Uint128::from(CLUB_BUYING_REWARD_AMOUNT),
            owner_released: false,
        },
    )?;
    //If successfully bought save the funds in contract wallet
    CONTRACT_WALLET.update(
        deps.storage,
        &buyer_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + price) },
    )?;

    if seller != "".to_string() {
        // TODO : Update the seller wallet
        let seller_addr = deps.api.addr_validate(&seller)?;
        //If successfully bought save the funds in contract wallet
        CONTRACT_WALLET.update(
            deps.storage,
            &seller_addr,
            |balance: Option<Uint128>| -> StdResult<_> { Ok(Uint128::from(0u128)) },
        )?;

        // Now save the previous ownership details
        CLUB_PREVIOUS_OWNER_DETAILS.save(
            deps.storage,
            club_name.clone(),
            &ClubPreviousOwnerDetails {
                club_name: club_name.clone(),
                previous_owner_address: seller.clone(),
                reward_amount: previous_owners_reward_amount,
            },
        )?;
    }
    return Ok(Response::default());
}

fn release_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    seller: String,
    club_name: String,
) -> Result<Response, ContractError> {
    let seller_addr = deps.api.addr_validate(&seller)?;
    //Check if seller is same as invoker
    if seller_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    // check that the current ownership is with the seller
    if ownership_details.is_none() {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("Releaser is not the owner for the club"),
        }));
    }
    for owner in ownership_details {
        if owner.owner_address != seller_addr {
            return Err(ContractError::Std(StdError::GenericErr {
                msg: String::from("Releaser is not the owner for the club"),
            }));
        } else {
            //capture the current system time
            let now = env.block.time;
            let mut duration = owner.locking_period; 
            let now_minus_duration_timestamp = now.minus_seconds(duration);
            if now_minus_duration_timestamp < owner.start_timestamp {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Locking period for the club is not over"),
                }));
            } else {
                // Update the ownership details
                CLUB_OWNERSHIP_DETAILS.save(
                    deps.storage,
                    club_name.clone(),
                    &ClubOwnershipDetails {
                        club_name: owner.club_name,
                        start_timestamp: owner.start_timestamp,
                        locking_period: owner.locking_period,
                        owner_address: owner.owner_address,
                        price_paid: owner.price_paid,
                        reward_amount: owner.reward_amount,
                        owner_released: true,
                    },
                )?;
            }
        }
    }
    return Ok(Response::default());
}

fn stake_on_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    club_name: String,
    staked_amount: Uint128,
) -> Result<Response, ContractError> {
    let staker_addr = deps.api.addr_validate(&staker)?;
    //Check if buyer is same as invoker
    if staker_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    //check if the club_name is available for staking
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }
    if ownership_details.is_some() {
        // Now save the staking details
        save_staking_details(
            deps.storage,
            env,
            staker.clone(),
            club_name.clone(),
            staked_amount,
            INCREASE_STAKE,
        );

        //If successfully staked, save the funds in contract wallet
        CONTRACT_WALLET.update(
            deps.storage,
            &staker_addr,
            |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + staked_amount) },
        )?;

        // TODO : uncomment here 
        // // Deduct amount from the stakers wallet
        // let config = CONFIG.load(deps.storage)?;
        // let res = Response::new()
        //     .add_message(WasmMsg::Execute {
        //         contract_addr: config.cw20_token_address.to_string(),
        //         funds: vec![],
        //         msg: to_binary(&Cw20ExecuteMsg::Send {
        //             contract: config.club_staking_address.to_string(),
        //             amount: amount,
        //             msg: to_binary(data: &T)
        //         })?,
        //     })
        //     .add_attributes(vec![
        //         attr("action", "stake"),
        //         attr("club", club_name),
        //         attr("address", info.sender),
        //         attr("amount", amount),
        //     ]);
    } else {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("The club is not available for staking"),
        }));
    }
    return Ok(Response::default());
}

fn withdraw_stake_from_a_club(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    staker: String,
    club_name: String,
    withdrawal_amount: Uint128,
    immediate_withdrawal: bool,
) -> Result<Response, ContractError> {
    let staker_addr = deps.api.addr_validate(&staker)?;
    //Check if withdrawer is same as invoker
    if staker_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    //check if the club_name is available for staking
    let ownership_details;
    let ownership_details_result = CLUB_OWNERSHIP_DETAILS.may_load(deps.storage, club_name.clone());
    match ownership_details_result {
        Ok(od) => {
            ownership_details = od;
        }
        Err(e) => {
            return Err(ContractError::Std(StdError::from(e)));
        }
    }

    if ownership_details.is_some() {
        // update funds in contract wallet
        CONTRACT_WALLET.update(
            deps.storage,
            &staker_addr,
            |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() - withdrawal_amount) },
        )?;
        
        if immediate_withdrawal == IMMEDIATE_WITHDRAWAL {
            // update the staking details
            save_staking_details(
                deps.storage,
                env,
                staker.clone(),
                club_name.clone(),
                withdrawal_amount,
                DECREASE_STAKE,
            );

            // TODO : Deduct 10% and burn it

            // Remaining 90% transfer to staker wallet
            // Also assuming that negative withdrawal means add
            let refund_amount = withdrawal_amount 
                .checked_mul(Uint128::from(90u128))
                .unwrap_or_default()
                .checked_div(Uint128::from(100u128))
                .unwrap_or_default();
            // TODO : uncomment here 
            // // Add refund_amount to the stakers wallet
            // let config = CONFIG.load(deps.storage)?;
            // let res = Response::new()
            //     .add_message(WasmMsg::Execute {
            //         contract_addr: config.cw20_token_address.to_string(),
            //         funds: vec![],
            //         msg: to_binary(&Cw20ExecuteMsg::Send {
            //             contract: config.club_staking_address.to_string(),
            //             amount: 0 - refund_amount,
            //             msg: to_binary(data: &T)
            //         })?,
            //     })
            //     .add_attributes(vec![
            //         attr("action", "stake"),
            //         attr("club", club_name),
            //         attr("address", info.sender),
            //         attr("amount", refund_amount),
            //     ]);

        } else {
            // update the staking details
            save_staking_details(
                deps.storage,
                env.clone(),
                staker.clone(),
                club_name.clone(),
                withdrawal_amount,
                DECREASE_STAKE,
            );

            // Move the withdrawn stakes to bonding list. The actual refunding of bonded
            // amounts happens on a periodic basis in periodically_refund_stakeouts
            save_bonding_details(
                deps.storage,
                env.clone(),
                staker.clone(),
                club_name.clone(),
                withdrawal_amount,
                CLUB_BONDING_DURATION,
            );
        }
    } else {
        return Err(ContractError::Std(StdError::GenericErr {
            msg: String::from("The club is not available for unstaking"),
        }));
    }
    return Ok(Response::default());
}

fn save_staking_details(
    storage: &mut dyn Storage,
    env: Env,
    staker: String,
    club_name: String,
    amount: Uint128,
    increase_stake: bool,
) -> Result<Response, ContractError> {
    // Get the exising stakes for this club
    let mut stakes = Vec::new();
    let all_stakes = CLUB_STAKING_DETAILS.may_load(storage, club_name.clone())?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {}
    }

    // if already staked for this club, then increase or decrease the staked_amount in existing stake
    let mut already_staked = false;
    let existing_stakes = stakes.clone();
    let mut updated_stakes = Vec::new();
    for stake in existing_stakes {
        let mut updated_stake = stake.clone();
        if staker == stake.staker_address {
            if increase_stake == INCREASE_STAKE {
                updated_stake.staked_amount += amount;
            } else {
                if updated_stake.staked_amount >= amount {
                    updated_stake.staked_amount -= amount;
                } else {
                    updated_stake.staked_amount = Uint128::from(0u128);
                }
            }
            already_staked = true;
        }
        if updated_stake.staked_amount > Uint128::from(0u128) {
            updated_stakes.push(updated_stake);
        }
    }
    if already_staked == true {
        // save the modified stakes - with updation or removal of existing stake
        CLUB_STAKING_DETAILS.save(storage, club_name, &updated_stakes)?;
    } else if increase_stake == INCREASE_STAKE {
        stakes.push(ClubStakingDetails {
            // TODO duration and timestamp fields no longer needed - should be removed
            staker_address: staker,
            staking_start_timestamp: env.block.time,
            staked_amount: amount,
            staking_duration: CLUB_STAKING_DURATION,
            club_name: club_name.clone(),
            reward_amount: Uint128::from(CLUB_STAKING_REWARD_AMOUNT), // ensure that the first time reward amount is set to 0
        });
        CLUB_STAKING_DETAILS.save(storage, club_name, &stakes)?;
    }

    return Ok(Response::default());
}

fn save_bonding_details(
    storage: &mut dyn Storage,
    env: Env,
    bonder: String,
    club_name: String,
    bonded_amount: Uint128,
    duration: u64,
) -> Result<Response, ContractError> {
    // Get the exising bonds for this club
    let mut bonds = Vec::new();
    let all_bonds = CLUB_BONDING_DETAILS.may_load(storage, club_name.clone())?;
    match all_bonds {
        Some(some_bonds) => {
            bonds = some_bonds;
        }
        None => {}
    }
    bonds.push(ClubBondingDetails {
        bonder_address: bonder,
        bonding_start_timestamp: env.block.time,
        bonded_amount: bonded_amount,
        bonding_duration: duration,
        club_name: club_name.clone(),
    });
    CLUB_BONDING_DETAILS.save(storage, club_name, &bonds)?;
    return Ok(Response::default());
}

fn set_reward_amount(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Check if this is executed by main/transaction wallet
    let config = CONFIG.load(deps.storage)?;
    if info.sender == config.cw20_token_address {
        return Err(ContractError::Unauthorized {});
    }
    REWARD.save(deps.storage, &amount)?;
    return Ok(Response::default());
}

fn claim_rewards(
    deps: DepsMut,
    info: MessageInfo,
    staker: String,
    club_name: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let staker_addr = deps.api.addr_validate(&staker)?;
    //Check if withdrawer is same as invoker
    if staker_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Get the exising stakes for this club
    let mut stakes = Vec::new();
    let all_stakes = CLUB_STAKING_DETAILS.may_load(deps.storage, club_name.clone())?;
    match all_stakes {
        Some(some_stakes) => {
            stakes = some_stakes;
        }
        None => {}
    }

    let existing_stakes = stakes.clone();
    let mut updated_stakes = Vec::new();
    for stake in existing_stakes {
        let mut updated_stake = stake.clone();
        if staker == stake.staker_address {
            if amount <= updated_stake.reward_amount {
                updated_stake.reward_amount -= amount;
                // TODO: transfer to staker wallet
            } else {
                return Err(ContractError::Std(StdError::GenericErr {
                    msg: String::from("Insufficient rewards"),
                }));
            }
        }
        updated_stakes.push(updated_stake);
    }
    CLUB_STAKING_DETAILS.save(deps.storage, club_name, &updated_stakes)?;

    return Ok(Response::default());
}

fn calculate_and_distribute_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {

    // Check if this is executed by main/transaction wallet
    let config = CONFIG.load(deps.storage)?;
    if info.sender == config.cw20_token_address {
        return Err(ContractError::Unauthorized {});
    }
    let total_reward = REWARD.may_load(deps.storage)?.unwrap_or_default();
    // No need to calculate if there is no reward amount
    if total_reward > Uint128::zero() {
        let mut reward_given_so_far = Uint128::zero();
        // Get the club ranking as per staking
        let top_rankers = get_clubs_ranking_by_stakes(deps.storage)?;
        // No need to proceed if there are no stakers
        if top_rankers.len() > 0 {
            let winner_club = &top_rankers[0];
            let winner_club_name = winner_club.0.clone();
            let mut winner_club_details =
                query_club_ownership_details(deps.storage, winner_club_name.clone())?;
            println!("winner club owner address = {:?}", winner_club_details.owner_address);
            //Increase owner funds by 1% of total reward
            let winner_club_reward = total_reward
                .checked_div(Uint128::from(100u128))
                .unwrap_or_default();
            winner_club_details.reward_amount += winner_club_reward;
            reward_given_so_far += winner_club_reward;
            println!("winner club owner reward = {:?}", winner_club_reward);
            CLUB_OWNERSHIP_DETAILS.save(
                deps.storage,
                winner_club_details.club_name.clone(),
                &winner_club_details,
            )?;
            /* TODO: Discuss this 
                This has been removed as reward should not be added to balance

            //Update the contract wallet with reward amount
            CONTRACT_WALLET.update(
                deps.storage,
                &deps.api.addr_validate(&winner_club_details.owner_address)?,
                |balance: Option<Uint128>| -> StdResult<_> {
                    Ok(balance.unwrap_or_default() + winner_club_details.reward_amount)
                },
            )?;
            */

            //Get all stakes for this club
            let mut stakes: Vec<ClubStakingDetails> = Vec::new();
            let all_stakes_for_winner =
                CLUB_STAKING_DETAILS.may_load(deps.storage, winner_club_name.clone())?;
            match all_stakes_for_winner {
                Some(some_stakes) => {
                    stakes = some_stakes;
                }
                None => {}
            }
            let reward_for_all_winners = total_reward
                .checked_mul(Uint128::from(19u128))
                .unwrap_or_default()
                .checked_div(Uint128::from(100u128))
                .unwrap_or_default();
            let total_staking_for_this_club = winner_club.1;
            let mut updated_stakes = Vec::new();
            for stake in stakes {
                let reward_for_this_winner = reward_for_all_winners
                    .checked_mul(stake.staked_amount)
                    .unwrap_or_default()
                    .checked_div(total_staking_for_this_club)
                    .unwrap_or_default();
                let mut updated_stake = stake.clone();
                updated_stake.reward_amount += reward_for_this_winner;
                reward_given_so_far += reward_for_this_winner;
                updated_stakes.push(updated_stake);
            }
            CLUB_STAKING_DETAILS.save(deps.storage, winner_club_name.clone(), &updated_stakes)?;

            // distribute the remaining 80% to all
            let remaining_reward = total_reward
                .checked_mul(Uint128::from(80u128))
                .unwrap_or_default()
                .checked_div(Uint128::from(100u128))
                .unwrap_or_default();
            let mut total_staking = Uint128::zero();
            let all_stakes = query_all_stakes(deps.storage)?;
            for stake in all_stakes {
                total_staking += stake.staked_amount;
            }
            let all_clubs: Vec<String> = CLUB_STAKING_DETAILS
                .keys(deps.storage, None, None, Order::Ascending)
                .map(|k| String::from_utf8(k).unwrap())
                .collect();
            for club_name in all_clubs {
                let mut all_stakes = Vec::new();
                let staking_details = CLUB_STAKING_DETAILS.load(deps.storage, club_name.clone())?;
                for mut stake in staking_details {
                    let reward_for_this_stake = (remaining_reward.checked_mul(stake.staked_amount))
                        .unwrap_or_default()
                        .checked_div(total_staking)
                        .unwrap_or_default();
                    stake.reward_amount += reward_for_this_stake;
                    println!("reward for {:?} is {:?} ", stake.staker_address, stake.reward_amount);
                    reward_given_so_far += reward_for_this_stake;
                    all_stakes.push(stake);
                }
                CLUB_STAKING_DETAILS.save(deps.storage, club_name, &all_stakes)?;
            }
            println!("total reward given {:?} out of {:?}", reward_given_so_far, total_reward);
        }
    }
    return Ok(Response::default());
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_binary(&query_allowance(deps, owner.clone(), owner.clone())?),
        QueryMsg::ClubStakingDetails { club_name } => {
            to_binary(&query_club_staking_details(deps.storage, club_name)?)
        }
        QueryMsg::ClubBondingDetails { club_name } => {
            to_binary(&query_club_bonding_details(deps.storage, club_name)?)
        }
        QueryMsg::ClubOwnershipDetails { club_name } => {
            to_binary(&query_club_ownership_details(deps.storage, club_name)?)
        }
        QueryMsg::AllStakes {} => to_binary(&query_all_stakes(deps.storage)?),
        QueryMsg::GetClubRankingByStakes {} => {
            to_binary(&get_clubs_ranking_by_stakes(deps.storage)?)
        }
        QueryMsg::RewardAmount {} => to_binary(&query_reward_amount(deps)?),
    }
}

pub fn query_club_staking_details(
    storage: &dyn Storage,
    club_name: String,
) -> StdResult<Vec<ClubStakingDetails>> {
    let csd = CLUB_STAKING_DETAILS.may_load(storage, club_name)?;
    match csd {
        Some(csd) => return Ok(csd),
        None => return Err(StdError::generic_err("No staking details found")),
    };
}

pub fn query_club_bonding_details(
    storage: &dyn Storage,
    club_name: String,
) -> StdResult<Vec<ClubBondingDetails>> {
    let csd = CLUB_BONDING_DETAILS.may_load(storage, club_name)?;
    match csd {
        Some(csd) => return Ok(csd),
        None => return Err(StdError::generic_err("No bonding details found")),
    };
}

fn query_all_stakes(storage: &dyn Storage) -> StdResult<Vec<ClubStakingDetails>> {
    let mut all_stakes = Vec::new();
    let all_clubs: Vec<String> = CLUB_STAKING_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        let staking_details = CLUB_STAKING_DETAILS.load(storage, club_name)?;
        for stake in staking_details {
            all_stakes.push(stake);
        }
    }
    return Ok(all_stakes);
}

fn query_all_bonds(storage: &dyn Storage) -> StdResult<Vec<ClubBondingDetails>> {
    let mut all_bonds = Vec::new();
    let all_clubs: Vec<String> = CLUB_BONDING_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        let bonding_details = CLUB_BONDING_DETAILS.load(storage, club_name)?;
        for bond in bonding_details {
            all_bonds.push(bond);
        }
    }
    return Ok(all_bonds);
}

fn get_clubs_ranking_by_stakes(storage: &dyn Storage) -> StdResult<Vec<(String, Uint128)>> {
    let mut all_stakes = Vec::new();
    let all_clubs: Vec<String> = CLUB_STAKING_DETAILS
        .keys(storage, None, None, Order::Ascending)
        .map(|k| String::from_utf8(k).unwrap())
        .collect();
    for club_name in all_clubs {
        let _tp = query_club_staking_details(storage, club_name.clone())?;
        let mut staked_amount = Uint128::zero();
        let mut club_name: Option<String> = None;
        for stake in _tp {
            staked_amount += stake.staked_amount;
            if club_name.is_none() {
                club_name = Some(stake.club_name.clone());
            }
        }
        all_stakes.push((club_name.unwrap(), staked_amount));
    }
    all_stakes.sort_by(|a, b| b.1.cmp(&a.1));
    return Ok(all_stakes);
}

fn query_reward_amount(deps: Deps) -> StdResult<Uint128> {
    let reward: Uint128 = REWARD.may_load(deps.storage)?.unwrap_or_default();
    return Ok(reward);
}

fn query_club_ownership_details(
    storage: &dyn Storage,
    club_name: String,
) -> StdResult<ClubOwnershipDetails> {
    let cod = CLUB_OWNERSHIP_DETAILS.may_load(storage, club_name)?;
    match cod {
        Some(cod) => return Ok(cod),
        None => return Err(StdError::generic_err("No ownership details found")),
    };
}

fn query_club_previous_owner_details(
    storage: &dyn Storage,
    club_name: String,
) -> StdResult<ClubPreviousOwnerDetails> {
    let cod = CLUB_PREVIOUS_OWNER_DETAILS.may_load(storage, club_name)?;
    match cod {
        Some(cod) => return Ok(cod),
        None => return Err(StdError::generic_err("No ownership details found")),
    };
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Addr, CosmosMsg, StdError, SubMsg, WasmMsg};

    use super::*;
    use crate::msg::InstantiateMarketingInfo;

    use cosmwasm_std::coin;

    #[test]
    fn test_buying_of_club () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let queryRes = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
                assert_eq!(cod.reward_amount, Uint128::from(100u128));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_owner_claim_rewards () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let queryRes = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
                assert_eq!(cod.reward_amount, Uint128::from(100u128));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        claim_owner_rewards(deps.as_mut(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string(), Uint128::from(10u128));

        let queryResAfter = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfter {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
                assert_eq!(cod.reward_amount, Uint128::from(90u128));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_multiple_buying_of_club () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let owner2Info = mock_info("Owner002", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner2Info.clone(), "Owner002".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let queryRes = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_releasing_of_club_before_locking_period () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        release_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let queryRes = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_releasing_of_club_after_locking_period () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        release_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let now = mock_env().block.time; // today

        let queryRes = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes {
            Ok(mut cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
                cod.start_timestamp = now.minus_seconds(22 * 24 * 60 * 60);
                CLUB_OWNERSHIP_DETAILS.save(&mut deps.storage, "CLUB001".to_string(), &cod);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        release_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let queryResAfterReleasing = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterReleasing {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, true);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_buying_of_club_after_releasing_by_prev_owner () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        release_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let now = mock_env().block.time; // today

        let queryRes = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes {
            Ok(mut cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                cod.start_timestamp = now.minus_seconds(22 * 24 * 60 * 60);
                CLUB_OWNERSHIP_DETAILS.save(&mut deps.storage, "CLUB001".to_string(), &cod);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        release_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let queryResAfterReleasing = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterReleasing {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, true);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let owner2Info = mock_info("Owner002", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner2Info.clone(), "Owner002".to_string(), "Owner001".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let queryResAfterSellingByPrevOwner = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterSellingByPrevOwner {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner002".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_claim_previous_owner_rewards () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        release_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let now = mock_env().block.time; // today

        let queryRes = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryRes {
            Ok(mut cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                cod.start_timestamp = now.minus_seconds(22 * 24 * 60 * 60);
                CLUB_OWNERSHIP_DETAILS.save(&mut deps.storage, "CLUB001".to_string(), &cod);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        release_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string());

        let queryResAfterReleasing = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterReleasing {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner001".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, true);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let owner2Info = mock_info("Owner002", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner2Info.clone(), "Owner002".to_string(), "Owner001".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let queryResAfterSellingByPrevOwner = query_club_ownership_details(&mut deps.storage, "CLUB001".to_string());
        match queryResAfterSellingByPrevOwner {
            Ok(cod) => { 
                assert_eq!(cod.owner_address, "Owner002".to_string());
                assert_eq!(cod.price_paid, Uint128::from(1000u128));
                assert_eq!(cod.owner_released, false);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        claim_previous_owner_rewards(deps.as_mut(), owner1Info.clone(), "Owner001".to_string(), "CLUB001".to_string(), Uint128::from(10u128));
        let queryPrevOwnerDetailsAfterRewardClaim = query_club_previous_owner_details(&mut deps.storage, "CLUB001".to_string());
        match queryPrevOwnerDetailsAfterRewardClaim {
            Ok(pod) => { 
                assert_eq!(pod.club_name, "CLUB001".to_string());
                assert_eq!(pod.previous_owner_address, "Owner001".to_string());
                assert_eq!(pod.reward_amount, Uint128::from(90u128));
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_multiple_staking_on_club_by_same_address () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let stakerInfo = mock_info("Staker0001", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(33u128));
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(11u128));
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(42u128));

        let queryRes = query_all_stakes(&mut deps.storage);
        match queryRes {
            Ok(all_stakes) => { 
                assert_eq!(all_stakes.len(), 1);
                for stake in all_stakes {
                    assert_eq!(stake.staked_amount, Uint128::from(86u128));
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_immediate_partial_withdrawals_from_club () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let stakerInfo = mock_info("Staker0001", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(99u128));
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(11u128), IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(12u128), IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(13u128), IMMEDIATE_WITHDRAWAL);

        let queryStakes = query_all_stakes(&mut deps.storage);
        match queryStakes {
            Ok(all_stakes) => { 
                assert_eq!(all_stakes.len(), 1);
                for stake in all_stakes {
                    assert_eq!(stake.staked_amount, Uint128::from(63u128));
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage);
        match queryBonds {
            Ok(all_bonds) => { 
                assert_eq!(all_bonds.len(), 0);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_immediate_complete_withdrawals_from_club () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let stakerInfo = mock_info("Staker0001", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(99u128));
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(11u128), IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(12u128), IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(13u128), IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(63u128), IMMEDIATE_WITHDRAWAL);

        let queryStakes = query_all_stakes(&mut deps.storage);
        match queryStakes {
            Ok(all_stakes) => { 
                assert_eq!(all_stakes.len(), 0);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage);
        match queryBonds {
            Ok(all_bonds) => { 
                assert_eq!(all_bonds.len(), 0);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_non_immediate_complete_withdrawals_from_club () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let stakerInfo = mock_info("Staker0001", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(99u128));
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(11u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(12u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(13u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(63u128), NO_IMMEDIATE_WITHDRAWAL);

        let queryStakes = query_all_stakes(&mut deps.storage);
        match queryStakes {
            Ok(all_stakes) => { 
                assert_eq!(all_stakes.len(), 0);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage);
        match queryBonds {
            Ok(all_bonds) => { 
                assert_eq!(all_bonds.len(), 4);
                for bond in all_bonds {
                    if bond.bonded_amount != Uint128::from(11u128) 
                            && bond.bonded_amount != Uint128::from(12u128) 
                            && bond.bonded_amount != Uint128::from(13u128) 
                            && bond.bonded_amount != Uint128::from(63u128) {
                        println!("bond is {:?} ", bond);
                        assert_eq!(1, 2);
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_non_immediate_complete_withdrawals_from_club_with_scheduled_refunds () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let stakerInfo = mock_info("Staker0001", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(99u128));
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(11u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(12u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(13u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(63u128), NO_IMMEDIATE_WITHDRAWAL);

        let queryStakes = query_all_stakes(&mut deps.storage);
        match queryStakes {
            Ok(all_stakes) => { 
                assert_eq!(all_stakes.len(), 0);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let now = mock_env().block.time; // today

        let queryBonds = query_all_bonds(&mut deps.storage);
        match queryBonds {
            Ok(all_bonds) => { 
                let existing_bonds = all_bonds.clone();
                let mut updated_bonds = Vec::new();
                assert_eq!(existing_bonds.len(), 4);
                for bond in existing_bonds {
                    let mut updated_bond = bond.clone();
                    if updated_bond.bonded_amount != Uint128::from(11u128) 
                            && updated_bond.bonded_amount != Uint128::from(12u128) 
                            && updated_bond.bonded_amount != Uint128::from(13u128) 
                            && updated_bond.bonded_amount != Uint128::from(63u128) {
                        println!("updated_bond is {:?} ", updated_bond);
                        assert_eq!(1, 2);
                    }
                    if updated_bond.bonded_amount == Uint128::from(63u128) {
                        updated_bond.bonding_start_timestamp = now.minus_seconds(8 * 24 * 60 * 60);
                    }
                    updated_bonds.push(updated_bond);
                }
                CLUB_BONDING_DETAILS.save(&mut deps.storage, "CLUB001".to_string(), &updated_bonds);
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }


        let refunderInfo = mock_info(MAIN_WALLET, &[coin(1000, "stake")]);
        periodically_refund_stakeouts(deps.as_mut(), mock_env(), refunderInfo);

        let queryBondsAfterPeriodicRefund = query_all_bonds(&mut deps.storage);
        match queryBondsAfterPeriodicRefund {
            Ok(all_bonds) => { 
                assert_eq!(all_bonds.len(), 3);
                for bond in all_bonds {
                    if bond.bonded_amount != Uint128::from(11u128) 
                            && bond.bonded_amount != Uint128::from(12u128) 
                            && bond.bonded_amount != Uint128::from(13u128) {
                        println!("bond is {:?} ", bond);
                        assert_eq!(1, 2);
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_non_immediate_partial_withdrawals_from_club () {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));

        let stakerInfo = mock_info("Staker0001", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(99u128));
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(11u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(12u128), NO_IMMEDIATE_WITHDRAWAL);
        withdraw_stake_from_a_club(deps.as_mut(), mock_env(), stakerInfo.clone(), "Staker0001".to_string(), 
            "CLUB001".to_string(), Uint128::from(13u128), NO_IMMEDIATE_WITHDRAWAL);

        let queryStakes = query_all_stakes(&mut deps.storage);
        match queryStakes {
            Ok(all_stakes) => { 
                assert_eq!(all_stakes.len(), 1);
                for stake in all_stakes {
                    assert_eq!(stake.staked_amount, Uint128::from(63u128));
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        let queryBonds = query_all_bonds(&mut deps.storage);
        match queryBonds {
            Ok(all_bonds) => { 
                assert_eq!(all_bonds.len(), 3);
                for bond in all_bonds {
                    if bond.bonded_amount != Uint128::from(11u128) 
                            && bond.bonded_amount != Uint128::from(12u128) 
                            && bond.bonded_amount != Uint128::from(13u128) {
                        println!("bond is {:?} ", bond);
                        assert_eq!(1, 2);
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }
    }

    #[test]
    fn test_distribute_rewards() {
        let mut deps = mock_dependencies(&[]);

        let owner1Info = mock_info("Owner001", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner1Info.clone(), "Owner001".to_string(), "".to_string(), "CLUB001".to_string(),
            Uint128::from(1000u128));
        let owner2Info = mock_info("Owner002", &[coin(1000, "stake")]); 
        buy_a_club(deps.as_mut(), mock_env(), owner2Info.clone(), "Owner002".to_string(), "".to_string(), "CLUB002".to_string(),
            Uint128::from(1000u128));
        let owner3Info = mock_info("Owner003", &[coin(1000, "stake")]);
        buy_a_club(deps.as_mut(), mock_env(), owner3Info.clone(), "Owner003".to_string(), "".to_string(), "CLUB003".to_string(),
            Uint128::from(1000u128));

        let staker1Info = mock_info("Staker0001", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), staker1Info.clone(), "Staker0001".to_string(), "CLUB001".to_string(), 
            Uint128::from(330000u128));

        let staker2Info = mock_info("Staker0002", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), staker2Info.clone(), "Staker0002".to_string(), "CLUB001".to_string(), 
            Uint128::from(110000u128));

        let staker3Info = mock_info("Staker0003", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), staker3Info.clone(), "Staker0003".to_string(), "CLUB002".to_string(), 
            Uint128::from(420000u128));

        let staker4Info = mock_info("Staker0004", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), staker4Info.clone(), "Staker0004".to_string(), "CLUB002".to_string(), 
            Uint128::from(100000u128));

        let staker5Info = mock_info("Staker0005", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), staker5Info.clone(), "Staker0005".to_string(), "CLUB003".to_string(), 
            Uint128::from(820000u128));

        let staker6Info = mock_info("Staker0006", &[coin(10, "stake")]);
        stake_on_a_club(deps.as_mut(), mock_env(), staker6Info.clone(), "Staker0006".to_string(), "CLUB003".to_string(), 
            Uint128::from(50000u128));

        let instantiate_msg = InstantiateMsg {
            cw20_token_address: "cwtoken11111".to_string(),
            admin_address: "admin11111".to_string(),
        };
        let rewardInfo = mock_info("rewardInfo", &[]);
        instantiate(deps.as_mut(), mock_env(), rewardInfo.clone(), instantiate_msg).unwrap();
        set_reward_amount(deps.as_mut(), rewardInfo.clone(), Uint128::from(1000000u128));

        let res = execute(
            deps.as_mut(),
            mock_env(),
            rewardInfo,
            ExecuteMsg::CalculateAndDistributeRewards { },
        )
        .unwrap();
        assert_eq!(res, Response::default());

        let queryRes = query_all_stakes(&mut deps.storage);
        match queryRes {
            Ok(all_stakes) => { 
                for stake in all_stakes {
                    let staker_address = stake.staker_address;
                    let reward_amount = stake.reward_amount;
                    if staker_address == "Staker0001" {
                        assert_eq!(reward_amount, Uint128::from(144262u128));
                    }
                    if staker_address == "Staker0002" {
                        assert_eq!(reward_amount, Uint128::from(48087u128));
                    }
                    if staker_address == "Staker0003" {
                        assert_eq!(reward_amount, Uint128::from(183606u128));
                    }
                    if staker_address == "Staker0004" {
                        assert_eq!(reward_amount, Uint128::from(43715u128));
                    }
                    if staker_address == "Staker0005" {
                        assert_eq!(reward_amount, Uint128::from(537549u128));
                    }
                    if staker_address == "Staker0006" {
                        assert_eq!(reward_amount, Uint128::from(32776u128));
                    }
                }
            }
            Err(e) => {
                println!("error parsing header: {:?}", e);
                assert_eq!(1, 2);
            }
        }

        /*
            winner club owner address = "Owner003"
            winner club owner reward = Uint128(10000)
            reward for "Staker0001" is Uint128(144262) 
            reward for "Staker0002" is Uint128(48087) 
            reward for "Staker0003" is Uint128(183606) 
            reward for "Staker0004" is Uint128(43715) 
            reward for "Staker0005" is Uint128(537549) 
            reward for "Staker0006" is Uint128(32776) 
            total reward given Uint128(999995) out of Uint128(1000000)
        */
    }
}
