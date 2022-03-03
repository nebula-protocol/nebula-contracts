#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::ext_query::query_asset_balance;
use crate::{
    error::ContractError,
    state::{store_config, store_target_asset_data},
    util::vec_to_string,
};
use astroport::asset::AssetInfo;
use cosmwasm_std::{attr, DepsMut, Env, MessageInfo, QuerierWrapper, Response, Uint128};
use nebula_protocol::cluster::{ClusterConfig, InstantiateMsg};

/// ## Description
/// Check for duplicate and unsupported assets
///
/// ## Params
/// - **querier** is an object of type [`QuerierWrapper`].
///
/// - **env** is a reference to an object of type [`Env`].
///
/// - **target_assets** is an object of type [`Vec<AssetInfo>`] which is
///     a vector of assets to be validated.
pub fn validate_targets(
    querier: QuerierWrapper,
    env: &Env,
    target_assets: Vec<AssetInfo>,
) -> Result<(), ContractError> {
    for i in 0..target_assets.len() {
        // Check if each asset is either a valid CW20 or native token.
        query_asset_balance(&querier, &env.contract.address, &target_assets[i])?;
        for j in i + 1..target_assets.len() {
            // Check for no duplication
            if target_assets[i].equal(&target_assets[j]) {
                return Err(ContractError::InvalidAssets {});
            }
        }
    }
    return Ok(());
}

/// ## Description
/// Creates a new contract with the specified parameters packed in the `msg` variable.
/// Returns a [`Response`] with the specified attributes if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **msg**  is a message of type [`InstantiateMsg`] which contains the parameters
///     used for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Extract params from `msg` as the initial cluster config
    let cfg = ClusterConfig {
        name: msg.name.clone(),
        description: msg.description.clone(),
        owner: deps.api.addr_validate(msg.owner.as_str())?,
        cluster_token: msg
            .cluster_token
            .map(|x| deps.api.addr_validate(x.as_str()))
            .transpose()?,
        factory: deps.api.addr_validate(msg.factory.as_str())?,
        pricing_oracle: deps.api.addr_validate(msg.pricing_oracle.as_str())?,
        target_oracle: deps.api.addr_validate(msg.target_oracle.as_str())?,
        penalty: deps.api.addr_validate(msg.penalty.as_str())?,
        active: true,
    };

    // Get asset infos from the provided `target`
    let asset_infos = msg
        .target
        .iter()
        .map(|x| x.info.clone())
        .collect::<Vec<_>>();

    // Get asset target weights from the provided `target`
    let weights = msg
        .target
        .iter()
        .map(|x| x.amount.clone())
        .collect::<Vec<_>>();

    // Target weights must not be zero
    for w in weights.iter() {
        if *w == Uint128::zero() {
            return Err(ContractError::Generic(
                "Initial weights cannot contain zero".to_string(),
            ));
        }
    }

    // Check assets in `target` for duplicate and unsupported assets
    if validate_targets(deps.querier, &env, asset_infos.clone()).is_err() {
        return Err(ContractError::InvalidAssets {});
    }

    let asset_data = msg.target.clone();

    // Save the cluster config and asset target weights
    store_config(deps.storage, &cfg)?;
    store_target_asset_data(deps.storage, &asset_data)?;

    let log = vec![
        attr("name", msg.name),
        attr("owner", msg.owner),
        attr("assets", vec_to_string(&asset_infos)),
    ];

    Ok(Response::new().add_attributes(log))
}
