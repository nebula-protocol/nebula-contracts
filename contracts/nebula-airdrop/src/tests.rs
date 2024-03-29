#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, migrate, query};
    use crate::error::ContractError;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary, to_binary, CosmosMsg, SubMsg, Uint128, WasmMsg};
    use cw2::{get_contract_version, ContractVersion};
    use cw20::Cw20ExecuteMsg;
    use nebula_protocol::airdrop::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, IsClaimedResponse, LatestStageResponse,
        MerkleRootResponse, MigrateMsg, QueryMsg,
    };

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: "owner0000".to_string(),
            nebula_token: "nebula0000".to_string(),
        };

        let info = mock_info("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner0000", config.owner.as_str());
        assert_eq!("nebula0000", config.nebula_token.as_str());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::LatestStage {}).unwrap();
        let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
        assert_eq!(0u8, latest_stage.latest_stage);
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: String::from("owner0000"),
            nebula_token: String::from("nebula0000"),
        };

        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // update owner
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            owner: Some("owner0001".to_string()),
            nebula_token: Some(("nebula0000").to_string()),
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner0001", config.owner.as_str());

        // unauthorized err
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            owner: None,
            nebula_token: None,
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});
    }

    #[test]
    fn register_merkle_root() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: String::from("owner0000"),
            nebula_token: String::from("nebula0000"),
        };

        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // unauthorized msg sender
        let info = mock_info("imposter0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                .to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});

        // try invalid merkle root
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "invalidroot".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidMerkle {});

        let info = mock_info("owner0000", &[]);
        // register new merkle root
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                .to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "register_merkle_root"),
                attr("stage", "1"),
                attr(
                    "merkle_root",
                    "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                )
            ]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::LatestStage {}).unwrap();
        let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
        assert_eq!(1u8, latest_stage.latest_stage);

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::MerkleRoot {
                stage: latest_stage.latest_stage,
            },
        )
        .unwrap();
        let merkle_root: MerkleRootResponse = from_binary(&res).unwrap();
        assert_eq!(
            "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
            merkle_root.merkle_root
        );
    }

    #[test]
    fn claim() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: String::from("owner0000"),
            nebula_token: String::from("nebula0000"),
        };

        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // register merkle roots
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "85e33930e7a8f015316cb4a53a4c45d26a69f299fc4c83f17357e1fd62e8fd95"
                .to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                .to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // try invalid proof
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::Claim {
            amount: Uint128::from(1000001u128),
            stage: 1u8,
            proof: vec![
                "b8ee25ffbee5ee215c4ad992fe582f20112368bc310ad9b2b7bdf440a224b2df".to_string(),
                "98d73e0a035f23c490fef5e307f6e74652b9d3688c2aa5bff70eaa65956a24e1".to_string(),
                "f328b89c766a62b8f1c768fefa1139c9562c6e05bab57a2afa7f35e83f9e9dcf".to_string(),
                "fe19ca2434f87cadb0431311ac9a484792525eb66a952e257f68bf02b4561950".to_string(),
            ],
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::MerkleVerification {});

        // now the right one
        let msg = ExecuteMsg::Claim {
            amount: Uint128::from(1000001u128),
            stage: 1u8,
            proof: vec![
                "b8ee25ffbee5ee215c4ad992fe582f20175868bc310ad9b2b7bdf440a224b2df".to_string(),
                "98d73e0a035f23c490fef5e307f6e74652b9d3688c2aa5bff70eaa65956a24e1".to_string(),
                "f328b89c766a62b8f1c768fefa1139c9562c6e05bab57a2af87f35e83f9e9dcf".to_string(),
                "fe19ca2434f87cadb0431311ac9a484792525eb66a952e257f68bf02b4561950".to_string(),
            ],
        };

        let info = mock_info("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("nebula0000"),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                    amount: Uint128::from(1000001u128),
                })
                .unwrap(),
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", "terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                attr("amount", "1000001")
            ]
        );

        assert_eq!(
            true,
            from_binary::<IsClaimedResponse>(
                &query(
                    deps.as_ref(),
                    mock_env(),
                    QueryMsg::IsClaimed {
                        stage: 1,
                        address: String::from("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                    }
                )
                .unwrap()
            )
            .unwrap()
            .is_claimed
        );

        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(res, ContractError::AlreadyClaimed {});

        // claim next airdrop
        let msg = ExecuteMsg::Claim {
            amount: Uint128::from(2000001u128),
            stage: 2u8,
            proof: vec![
                "ca2784085f944e5594bb751c3237d6162f7c2b24480b3a37e9803815b7a5ce42".to_string(),
                "5b07b5898fc9aa101f27344dab0737aede6c3aa7c9f10b4b1fda6d26eb669b0f".to_string(),
                "4847b2b9a6432a7bdf2bdafacbbeea3aab18c524024fc6e1bc655e04cbc171f3".to_string(),
                "cad1958c1a5c815f23450f1a2761a5a75ab2b894a258601bf93cd026469d42f2".to_string(),
            ],
        };

        let info = mock_info("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from("nebula0000"),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                    amount: Uint128::from(2000001u128),
                })
                .unwrap(),
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "2"),
                attr("address", "terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                attr("amount", "2000001")
            ]
        );
    }

    #[test]
    fn migration() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: "owner0000".to_string(),
            nebula_token: "nebula0000".to_string(),
        };
        let info = mock_info("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // assert contract infos
        assert_eq!(
            get_contract_version(&deps.storage),
            Ok(ContractVersion {
                contract: "nebula-airdrop".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string()
            })
        );

        // it worked, let's migrate the contract
        let msg = MigrateMsg {};

        // we can just call .unwrap() to assert this was a success
        let _res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
    }
}
