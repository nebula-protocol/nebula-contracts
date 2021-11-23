from contract_helpers import ClusterContract, Contract, store_contracts, deployer, dict_to_b64
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.util.json import dict_to_data

from api import Asset
import pprint
import asyncio

DEFAULT_POLL_ID = 1
DEFAULT_QUORUM = "0.3"
DEFAULT_THRESHOLD = "0.5"
DEFAULT_VOTING_PERIOD = 2
DEFAULT_EFFECTIVE_DELAY = 2
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"
DEFAULT_SNAPSHOT_PERIOD = 0
DEFAULT_VOTER_WEIGHT = "0.2"
    

class Ecosystem:
    def __init__(self, require_gov=False):

        # not using governance allows us to directly spin up clusters fast
        # otherwise we need to spin up clusters through governance
        self.require_gov = require_gov

        self.code_ids = None

        self.airdrop = None
        self.collector = None
        self.community = None

        # TODO support multiple cluster contracts per ecosystem
        self.cluster = None
        self.cluster_token = None
        self.asset_tokens = None
        self.asset_prices = None
        self.cluster_pair = None
        self.lp_token = None

        self.factory = None
        self.gov = None
        self.incentives = None
        self.incentives_custody = None
        self.staking = None
        self.terraswap_factory = None

        self.neb_token = None
        self.neb_pair = None
        self.dummy_oracle = None

    # background contracts needed to create cluster contracts
    async def initialize_base_contracts(self):
        print("Initializing base contracts...")
        code_ids = self.code_ids = await store_contracts()
        print(code_ids)

        if self.terraswap_factory is None:
            self.terraswap_factory = await Contract.create(
                code_ids["terraswap_factory"],
                pair_code_id=int(code_ids["terraswap_pair"]),
                token_code_id=int(code_ids["terraswap_token"]),
            )

        self.factory = await Contract.create(
            code_ids["nebula_cluster_factory"],
            token_code_id=int(code_ids["terraswap_token"]),
            cluster_code_id=int(code_ids["nebula_cluster"]),
            base_denom="uusd",
            protocol_fee_rate="0.001",
            distribution_schedule=[[0, 100000, "1000000"]],
        )

        print('factory', self.factory)

        self.neb_token = await Contract.create(
            code_ids["terraswap_token"],
            name="Nebula Token",
            symbol="NEB",
            decimals=6,
            initial_balances=[
                {
                    "address": deployer.key.acc_address,
                    "amount": "1000000000000",
                },
                {
                    "address": self.factory,
                    "amount": "10000000000",
                },
            ],
            minter={"minter": self.factory, "cap": None},
        )

        self.staking = await Contract.create(
            code_ids["nebula_lp_staking"],
            owner=self.factory,
            nebula_token=self.neb_token,
            terraswap_factory=self.terraswap_factory,
            base_denom="uusd",
            premium_min_update_interval=5,
        )

        self.gov = await Contract.create(
            code_ids["nebula_gov"],
            nebula_token=self.neb_token,
            quorum=DEFAULT_QUORUM,
            threshold=DEFAULT_THRESHOLD,
            voting_period=DEFAULT_VOTING_PERIOD,
            effective_delay=DEFAULT_EFFECTIVE_DELAY,
            proposal_deposit=DEFAULT_PROPOSAL_DEPOSIT,
            voter_weight=DEFAULT_VOTER_WEIGHT,
            snapshot_period=DEFAULT_SNAPSHOT_PERIOD,
        )

        print('gov', self.gov)

        self.collector = await Contract.create(
            code_ids["nebula_collector"],
            distribution_contract=self.gov,
            terraswap_factory=self.terraswap_factory,
            nebula_token=self.neb_token,
            base_denom="uusd",
            owner=self.factory,
        )

        self.neb_pair = Contract(
            (
                await self.terraswap_factory.create_pair(
                    asset_infos=[
                        Asset.cw20_asset_info(self.neb_token),
                        Asset.native_asset_info("uusd"),
                    ]
                )
            )
            .logs[0]
            .events_by_type["from_contract"]["pair_contract_addr"][0]
        )

        await self.factory.post_initialize(
            owner=deployer.key.acc_address,
            nebula_token=self.neb_token,
            terraswap_factory=self.terraswap_factory,
            staking_contract=self.staking,
            commission_collector=self.collector,
        )


        if self.require_gov:
            # Update factory owner as gov
            await self.factory.update_config(owner=self.gov)

        # provide some liquidity so conversion works
        await self.neb_token.increase_allowance(amount="1000", spender=self.neb_pair)
        await self.neb_pair.provide_liquidity(
            assets=[
                Asset.asset(self.neb_token, amount="1000"),
                Asset.asset("uusd", amount="1000", native=True),
            ],
            _send={"uusd": "1000"},
        )

    # standalone-ish contracts, could still create cluster without these
    async def initialize_extraneous_contracts(self):
        print("Initializing extraneous contracts...")
        code_ids = self.code_ids
        self.community = await Contract.create(
            code_ids["nebula_community"],
            owner=self.gov,
            nebula_token=self.neb_token,
            spend_limit="1000000",
        )

        self.airdrop = await Contract.create(
            code_ids["nebula_airdrop"],
            owner=deployer.key.acc_address,
            nebula_token=self.neb_token,
        )

        self.incentives_custody = await Contract.create(
            code_ids["nebula_incentives_custody"],
            owner=deployer.key.acc_address,
            neb_token=self.neb_token,
        )

        self.incentives = await Contract.create(
            code_ids["nebula_incentives"],
            owner=deployer.key.acc_address,
            factory=self.factory,
            terraswap_factory=self.terraswap_factory,
            nebula_token=self.neb_token,
            custody=self.incentives_custody,
            base_denom="uusd",
        )

        # stupid name mangling
        await self.incentives_custody.__getattr__("update_owner")(
            owner=self.incentives
        )

    async def create_cluster(
        self,
        cluster_tokens,
        asset_tokens,
        asset_prices,
        target_weights,
        penalty_params,
        asset_names=None,
    ):
        print("Creating cluster...")

        cluster_tokens = str(cluster_tokens)
        asset_tokens = tuple(str(i) for i in asset_tokens)
        asset_prices = tuple(str(i) for i in asset_prices)
        target_weights = tuple(str(i) for i in target_weights)
        penalty_params = {k: str(v) for k, v in penalty_params.items()}

        code_ids = self.code_ids

        if asset_names is None:
            assets = [
                (
                    await Contract.create(
                        code_ids["terraswap_token"],
                        name=f"Asset {i}",
                        symbol=f"AA{chr(i + 97)}",
                        decimals=6,
                        initial_balances=[
                            {
                                "address": deployer.key.acc_address,
                                "amount": "1" + "0" * 15,
                            }
                        ],
                        mint=None,
                    )
                )
                for i in range(len(asset_tokens))
            ]
        else:
            assets = [
                (
                    await Contract.create(
                        code_ids["terraswap_token"],
                        name=name,
                        symbol=name,
                        decimals=6,
                        initial_balances=[
                            {
                                "address": deployer.key.acc_address,
                                "amount": "1" + "0" * 15,
                            }
                        ],
                        mint=None,
                    )
                )
                for name in asset_names
            ]

        assets = tuple(assets)
        oracle = await Contract.create(code_ids["nebula_dummy_oracle"], owner=deployer.key.acc_address)
        await oracle.set_prices(prices=list(zip(assets, asset_prices)))
        
        self.dummy_oracle = oracle

        penalty_contract = await Contract.create(
            code_ids["nebula_penalty"],
            penalty_params=penalty_params,
            owner=self.factory,
        )

        target = [Asset.asset(info, amount) for info, amount in zip(assets, target_weights)]

        create_cluster = self.factory.create_cluster(
            name="CLUSTER",
            symbol="BSK",
            params={
                "name": "CLUSTER",
                "symbol": "BSK",
                "description": "Test cluster",
                "penalty": penalty_contract,
                "target": target,
                "pricing_oracle": oracle,
                "target_oracle": deployer.key.acc_address,
            },
        )

        if self.require_gov:
            
            await self.neb_token.send(
                contract=self.gov,
                amount="600000000000",
                msg=dict_to_b64({'stake_voting_tokens': {'lock_for_weeks': 104}}),
            )
            
            string_target = [Asset.asset(info.address, amount) for info, amount in zip(assets, target_weights)]
            print(string_target)
            create_dict = {
                "create_cluster": {
                    # "name": "CLUSTER",
                    # "symbol": "BSK",
                    "params": {
                        "name": "CLUSTER",
                        "symbol": "BSK",
                        "description": "Test cluster",
                        "penalty": penalty_contract.address,
                        "target": string_target,
                        "pricing_oracle": oracle.address,
                        "target_oracle": deployer.key.acc_address,
                    },
                }
            }

            resp = await self.create_and_execute_poll(
                {"contract": self.factory.address, "msg": dict_to_b64(create_dict)}
            )
        else:
            resp = await create_cluster

        logs = resp.logs[0].events_by_type

        instantiation_logs = logs["instantiate_contract"]
        addresses = instantiation_logs["contract_address"]

        self.asset_tokens = assets
        self.asset_prices = asset_prices

        # self.cluster_token = Contract(addresses[2])
        # self.cluster_pair = Contract(addresses[1])
        # self.lp_token = Contract(addresses[0])
        # self.cluster = ClusterContract(
        #     addresses[3],
        #     self.cluster_token,
        #     self.asset_tokens,
        # )

        self.cluster_token = Contract(addresses[1])
        self.cluster_pair = Contract(addresses[2])
        self.lp_token = Contract(addresses[3])
        self.cluster = ClusterContract(
            addresses[0],
            self.cluster_token,
            self.asset_tokens,
        )

        # initialize the cluster with its initial state
        await self.cluster.mint(asset_amounts=asset_tokens, min_tokens=cluster_tokens)

        print(f"Successfully created cluster:")
        cluster_state = await self.cluster.query.cluster_state(
            cluster_contract_address=self.cluster.address
        )

        pprint.pprint(cluster_state)

        cluster_list = await self.factory.query.cluster_list()

        pprint.pprint(cluster_list)

        assert (
            cluster_state["cluster_contract_address"] 
            == cluster_list["contract_infos"][0][0]
        )

    async def create_and_execute_poll(
        self, execute_msg, distribute_collector=False, sleep_time=2
    ):
        create_msg = {
            'create_poll': {
                "title": "A new poll!",
                "description": "Wow, I love polls!",
                # "link":"See more at https://nebulaprotocol.org",
                "execute_msg": execute_msg,
            }
        }
        resp = await self.neb_token.send(
            contract=self.gov,
            amount=DEFAULT_PROPOSAL_DEPOSIT,
            msg=dict_to_b64(create_msg),
        )

        poll_id = int(resp.logs[0].events_by_type["from_contract"]["poll_id"][0])
        await self.gov.cast_vote(poll_id=poll_id, vote="yes", amount="600000000000")
        await asyncio.sleep(sleep_time)

        if distribute_collector:
            await self.collector.distribute()

        await self.gov.end_poll(poll_id=poll_id)
        return await self.gov.execute_poll(poll_id=poll_id)
