from contract_helpers import BasketContract, Contract, store_contracts, deployer
from api import Asset
import pprint
import asyncio

DEFAULT_POLL_ID = 1
DEFAULT_QUORUM = "0.3"
DEFAULT_THRESHOLD = "0.5"
DEFAULT_VOTING_PERIOD = 2
DEFAULT_EFFECTIVE_DELAY = 2
DEFAULT_EXPIRATION_PERIOD = 20000
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"
DEFAULT_SNAPSHOT_PERIOD = 0
DEFAULT_VOTER_WEIGHT = "0.1"


class Ecosystem:
    def __init__(self, require_gov=False):

        # not using governance allows us to directly spin up baskets fast
        # otherwise we need to spin up baskets through governance
        self.require_gov = require_gov

        self.code_ids = None

        self.airdrop = None
        self.collector = None
        self.community = None

        # TODO support multiple basket contracts per ecosystem
        self.basket = None
        self.basket_token = None
        self.asset_tokens = None
        self.basket_pair = None
        self.lp_token = None

        self.factory = None
        self.gov = None
        self.incentives = None
        self.incentives_custody = None
        self.staking = None
        self.terraswap_factory = None

        self.neb_token = None
        self.neb_pair = None

    # background contracts needed to create basket contracts
    async def initialize_base_contracts(self):
        print("Initializing base contracts...")
        code_ids = self.code_ids = await store_contracts()

        self.terraswap_factory = await Contract.create(
            code_ids["terraswap_factory"],
            pair_code_id=int(code_ids["terraswap_pair"]),
            token_code_id=int(code_ids["terraswap_token"]),
        )

        self.factory = await Contract.create(
            code_ids["basket_factory"],
            token_code_id=int(code_ids["terraswap_token"]),
            cluster_code_id=int(code_ids["basket_contract"]),
            base_denom="uusd",
            protocol_fee_rate="0.001",
            distribution_schedule=[[0, 100000, "1000000"]],
        )

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
            code_ids["basket_staking"],
            owner=self.factory,
            nebula_token=self.neb_token,
            terraswap_factory=self.terraswap_factory,
            base_denom="uusd",
            premium_min_update_interval=5,
        )

        self.gov = await Contract.create(
            code_ids["basket_gov"],
            nebula_token=self.neb_token,
            quorum=DEFAULT_QUORUM,
            threshold=DEFAULT_THRESHOLD,
            voting_period=DEFAULT_VOTING_PERIOD,
            effective_delay=DEFAULT_EFFECTIVE_DELAY,
            expiration_period=DEFAULT_EXPIRATION_PERIOD,
            proposal_deposit=DEFAULT_PROPOSAL_DEPOSIT,
            voter_weight=DEFAULT_VOTER_WEIGHT,
            snapshot_period=DEFAULT_SNAPSHOT_PERIOD,
        )

        self.collector = await Contract.create(
            code_ids["basket_collector"],
            distribution_contract=self.gov,
            terraswap_factory=self.terraswap_factory,
            nebula_token=self.neb_token,
            base_denom="uusd",
            owner=self.factory,
        )

        await self.factory.post_initialize(
            owner=self.gov if self.require_gov else deployer.key.acc_address,
            nebula_token=self.neb_token,
            oracle_contract=self.neb_token,  # ??? provide arbitrary contract for now
            terraswap_factory=self.terraswap_factory,
            staking_contract=self.staking,
            commission_collector=self.collector,
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

        # provide some liquidity so conversion works
        await self.neb_token.increase_allowance(amount="1000", spender=self.neb_pair)
        await self.neb_pair.provide_liquidity(
            assets=[
                Asset.asset(self.neb_token, amount="1000"),
                Asset.asset("uusd", amount="1000", native=True),
            ],
            _send={"uusd": "1000"},
        )

    # standalone-ish contracts, could still create basket without these
    async def initialize_extraneous_contracts(self):
        print("Initializing extraneous contracts...")
        code_ids = self.code_ids
        self.community = await Contract.create(
            code_ids["basket_community"],
            owner=self.gov,
            nebula_token=self.neb_token,
            spend_limit="1000000",
        )

        self.airdrop = await Contract.create(
            code_ids["airdrop"],
            owner=deployer.key.acc_address,
            nebula_token=self.neb_token,
        )

        self.incentives_custody = await Contract.create(
            code_ids["basket_incentives_custody"],
            owner=deployer.key.acc_address,
            neb_token=self.neb_token,
        )

        self.incentives = await Contract.create(
            code_ids["basket_incentives"],
            owner=deployer.key.acc_address,
            factory=self.factory,
            terraswap_factory=self.terraswap_factory,
            nebula_token=self.neb_token,
            custody=self.incentives_custody,
            base_denom="uusd",
        )

        # stupid name mangling
        await self.incentives_custody.__getattr__("__reset_owner")(
            owner=self.incentives
        )

    async def create_basket(
        self,
        basket_tokens,
        asset_tokens,
        asset_prices,
        target_weights,
        penalty_params,
    ):
        print("Creating basket...")

        basket_tokens = str(basket_tokens)
        asset_tokens = tuple(str(i) for i in asset_tokens)
        asset_prices = tuple(str(i) for i in asset_prices)
        target_weights = tuple(int(i) for i in target_weights)
        penalty_params = {k: str(v) for k, v in penalty_params.items()}

        code_ids = self.code_ids

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

        assets = tuple(assets)
        oracle = await Contract.create(code_ids["basket_dummy_oracle"])
        await oracle.set_prices(prices=list(zip(assets, asset_prices)))

        penalty_contract = await Contract.create(
            code_ids["basket_penalty"],
            penalty_params=penalty_params,
            owner=self.factory,
        )

        create_basket = self.factory.create_cluster(
            name="BASKET",
            symbol="BSK",
            params={
                "name": "BASKET",
                "symbol": "BSK",
                "penalty": penalty_contract,
                "target": target_weights,
                "assets": [Asset.cw20_asset_info(i) for i in assets],
                "pricing_oracle": oracle,
                "composition_oracle": oracle,
            },
        )

        if self.require_gov:

            await self.neb_token.send(
                contract=self.gov,
                amount="600000000000",
                msg=self.gov.stake_voting_tokens(),
            )

            resp = await self.create_and_execute_poll(
                {"contract": self.factory, "msg": create_basket}
            )
        else:

            resp = await create_basket

        logs = resp.logs[0].events_by_type

        instantiation_logs = logs["instantiate_contract"]
        addresses = instantiation_logs["contract_address"]

        self.basket_token = Contract(addresses[2])
        self.basket_pair = Contract(addresses[1])
        self.lp_token = Contract(addresses[0])
        self.asset_tokens = assets
        self.basket = BasketContract(
            addresses[3],
            self.basket_token,
            self.asset_tokens,
        )

        # initialize the basket with its initial state
        await self.basket.mint(asset_amounts=asset_tokens, min_tokens=basket_tokens)

        print(f"Successfully created basket:")
        pprint.pprint(
            await self.basket.query.basket_state(
                basket_contract_address=self.basket.address
            )
        )

    async def create_and_execute_poll(self, execute_msg, distribute_collector=False):
        resp = await self.neb_token.send(
            contract=self.gov,
            amount=DEFAULT_PROPOSAL_DEPOSIT,
            msg=self.gov.create_poll(
                title="A new poll!",
                description="Wow, I love polls!",
                link="See more at https://nebulaprotocol.org",
                execute_msg=execute_msg,
            ),
        )

        poll_id = int(resp.logs[0].events_by_type["from_contract"]["poll_id"][0])

        await self.gov.cast_vote(poll_id=poll_id, vote="yes", amount="600000000000")
        await asyncio.sleep(1)

        if distribute_collector:
            await self.collector.distribute()

        await self.gov.end_poll(poll_id=poll_id)
        return await self.gov.execute_poll(poll_id=poll_id)
