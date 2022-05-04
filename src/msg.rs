use cosmwasm_std::{Binary, Uint128};
use cw0::Expiration;
use cw20::Logo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<Logo>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub cw20_token_address: String,
    pub admin_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    BuyAClub {
        buyer: String,
        seller: String,
        club_name: String,
    },
    ReleaseClub {
        owner: String,
        club_name: String,
    },
    ClaimOwnerRewards {
        owner: String,
        club_name: String,
        amount: Uint128,
    },
    ClaimPreviousOwnerRewards {
        previous_owner: String,
        club_name: String,
        amount: Uint128,
    },
    StakeOnAClub {
        staker: String,
        club_name: String,
        amount: Uint128,
    },
    StakeWithdrawFromAClub {
        staker: String,
        club_name: String,
        amount: Uint128,
        immediate_withdrawal: bool,
    },
    PeriodicallyRefundStakeouts {},
    SetRewardAmount {
        amount: Uint128,
    },
    CalculateAndDistributeRewards{},
    ClaimRewards {
        staker: String,
        club_name: String,
        amount: Uint128,
    },
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    BurnFrom {
        owner: String,
        amount: Uint128,
    },
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Only with "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    /// Return type: AllowanceResponse.
    Allowance {
        owner: String,
        spender: String,
    },
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this owner has approved. Supports pagination.
    /// Return type: AllAllowancesResponse.
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the current state of vesting information for the given address.
    /// Return type: StakingDetails.
    ClubStakingDetails {
        club_name: String,
    },
    /// Returns the current state of withdrawn tokens that are locked for 
    /// BONDING_DURATION = 7 days (before being credited back) for the given address.
    /// Return type: BondingDetails.
    ClubBondingDetails {
        club_name: String,
    },
    ClubOwnershipDetails {
        club_name: String,
    },
    AllStakes {},
    GetClubRankingByStakes {},
    RewardAmount {},
}
