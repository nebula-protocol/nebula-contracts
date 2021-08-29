from constants import DEPLOY_ENVIRONMENT_STATUS_W_GOV, CONTRACT_TOKEN_TO_SYM_TEQ, CT_SYM_TO_NAME, CT_SYM_TO_RECOMP_ORACLE, CT_SYM_TO_RECOMPOSER, get_terra_ecosystem_info
import os
# os.environ["USE_TEQUILA"] = "1"
# os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from contract_helpers import ClusterContract, Contract, store_contracts, deployer
from pricing_bot_all import get_query_info, set_prices
from api import Asset
import pprint
import asyncio

DEFAULT_POLL_ID = 1
DEFAULT_QUORUM = "0.1"
DEFAULT_THRESHOLD = "0.5"
DEFAULT_VOTING_PERIOD = 100
DEFAULT_EFFECTIVE_DELAY = 100
DEFAULT_PROPOSAL_DEPOSIT = "1000000"
DEFAULT_SNAPSHOT_PERIOD = 20
DEFAULT_VOTER_WEIGHT = "0.5"

DEFAULT_PROTOCOL_FEE_RATE = "0.001"
DEFAULT_DISTRIBUTION_SCHEDULE = [[0, 100000, "1000000"]]

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
    async def initialize_contracts(self):
        print("Initializing base contracts...")
        code_ids = self.code_ids = await store_contracts()

        if self.terraswap_factory is None:
            print("Initializing terraswap factory")
            self.terraswap_factory = await Contract.create(
                code_ids["terraswap_factory"],
                pair_code_id=int(code_ids["terraswap_pair"]),
                token_code_id=int(code_ids["terraswap_token"]),
            )

        print("Initializing nebula cluster factory")

        self.factory = await Contract.create(
            code_ids["nebula_cluster_factory"],
            token_code_id=int(code_ids["terraswap_token"]),
            cluster_code_id=int(code_ids["nebula_cluster"]),
            base_denom="uusd",
            protocol_fee_rate=DEFAULT_PROTOCOL_FEE_RATE,
            distribution_schedule=DEFAULT_DISTRIBUTION_SCHEDULE,
        )

        print("Initializing Nebula token")
        self.neb_token = await Contract.create(
            code_ids["terraswap_token"],
            name="Nebula Token",
            symbol="NEB",
            decimals=6,
            initial_balances=[
                {
                    "address": deployer.key.acc_address,
                    "amount": "1000000000000000",
                },
                {
                    "address": self.factory,
                    "amount": "10000000000",
                }, # TODO: give to airdrop / community
            ],
            minter={"minter": self.factory, "cap": None},
        )

        print("Initializing LP staking contract")
        self.staking = await Contract.create(
            code_ids["nebula_lp_staking"],
            owner=self.factory,
            nebula_token=self.neb_token,
            terraswap_factory=self.terraswap_factory,
            base_denom="uusd"
        )

        print("Initializing gov contract")
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

        print("Initializing collector contract")
        self.collector = await Contract.create(
            code_ids["nebula_collector"],
            distribution_contract=self.gov,
            terraswap_factory=self.terraswap_factory,
            nebula_token=self.neb_token,
            base_denom="uusd",
            owner=self.factory,
        )

        print("Post initialize factory including resetting the owner")
        await self.factory.post_initialize(
            owner=deployer.key.acc_address,
            nebula_token=self.neb_token,
            terraswap_factory=self.terraswap_factory,
            staking_contract=self.staking,
            commission_collector=self.collector,
        )

        print("Create Terraswap pair for NEB-UST")
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

        print("Set NEB-UST weight in factory")
        await self.factory.terraswap_creation_hook(asset_token=self.neb_token)

        print("Try to initialize all clusters")
        self.dummy_oracle = await Contract.create(code_ids["nebula_dummy_oracle"])

        # First initialize clusters here without governance
        # TODO: Check with Mirror team to see what they do for initial genesis mAsset creation

        pprint.pprint(self.__dict__)

        contract_addrs, symbols, query_info = await get_query_info()
        for ct_sym, recomp_oracle in CT_SYM_TO_RECOMP_ORACLE.items():
            print("Trying to deploy", CT_SYM_TO_NAME[ct_sym], ct_sym)
            await set_prices(self.dummy_oracle, contract_addrs, query_info)
            cluster = await self.deploy_cluster(ct_sym, recomp_oracle)
            print("Cluster address is ", cluster)

        print("Set owner to gov")
        await self.factory.update_config(owner=self.gov)

        cluster_list = await self.factory.query.cluster_list()
        print("Cluster list", cluster_list)

        print("Initializing community contract")
        self.community = await Contract.create(
            code_ids["nebula_community"],
            owner=self.gov,
            nebula_token=self.neb_token,
            spend_limit="1000000",
        )
        
        print("Initializing airdrop contract")
        self.airdrop = await Contract.create(
            code_ids["nebula_airdrop"],
            owner=deployer.key.acc_address,
            nebula_token=self.neb_token,
        )

        print("Initializing incentives custody contract")
        self.incentives_custody = await Contract.create(
            code_ids["nebula_incentives_custody"],
            owner=deployer.key.acc_address,
            neb_token=self.neb_token,
        )

        # Nebula incentives, terraswap arb
        # TODO: rename to nebula-incentives-terraswap-arb (?)
        print("Initializing incentives contract")
        self.incentives = await Contract.create(
            code_ids["nebula_incentives"],
            owner=deployer.key.acc_address,
            factory=self.factory,
            terraswap_factory=self.terraswap_factory,
            nebula_token=self.neb_token,
            custody=self.incentives_custody,
            base_denom="uusd",
        )

        # Sets incentives_custody owner to be incentives
        await self.incentives_custody.__getattr__("update_owner")(
            owner=self.incentives
        )

    async def deploy_cluster(self, ct_symbol, recomp_oracle):
        code_ids = self.code_ids
        
        penalty_params = {
            "penalty_amt_lo": "0.02",
            "penalty_cutoff_lo": "0.01",
            "penalty_amt_hi": "1",
            "penalty_cutoff_hi": "0.1",
            "reward_amt": "0.01",
            "reward_cutoff": "0.02",
        }

        # TODO: Call recomp deploy here somehow
        recomposer_func = CT_SYM_TO_RECOMPOSER[ct_symbol]

        if ct_symbol == 'TER':
            assets, asset_token_supply = await get_terra_ecosystem_info()
            recomposer = recomposer_func("", assets, asset_token_supply)
        else:
            recomposer = recomposer_func("")

        target = await recomposer.weighting()

        # target = [Asset.asset("uluna", "1", native=True)]

        penalty_contract = await Contract.create(
            code_ids["nebula_penalty"],
            penalty_params=penalty_params,
            owner=self.factory,
        )

        print(penalty_contract)

        oracle = self.dummy_oracle

        create_cluster = self.factory.create_cluster(
            params={
                "name": CT_SYM_TO_NAME[ct_symbol],
                "description": f"Testing {ct_symbol} cluster",
                "symbol": ct_symbol,
                "penalty": penalty_contract,
                "target": target,
                "pricing_oracle": oracle, # Generic pricing oracle
                "target_oracle": recomp_oracle,
                # "target_oracle": deployer.key.acc_address,
            },
        )

        resp = await create_cluster

        logs = resp.logs[0].events_by_type

        instantiation_logs = logs["instantiate_contract"]
        addresses = instantiation_logs["contract_address"]

        cluster_token = Contract(addresses[2])

        # Use this because cw20
        cluster = ClusterContract(
            addresses[3],
            cluster_token,
            None,
        )

        return cluster