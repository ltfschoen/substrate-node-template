extern crate mining_eligibility_proxy as mining_eligibility_proxy;

const INITIAL_DHX_DAO_TREASURY_UNLOCKED_RESERVES_BALANCE: u64 = 30000000;

#[cfg(test)]
mod tests {
    use super::*;

    use frame_support::{
        assert_err,
        assert_ok,
        parameter_types,
        traits::{
            Contains,
            ContainsLengthBound,
            Currency,
            EnsureOrigin,
        },
        weights::{
            IdentityFee,
            Weight,
        },
    };
    use frame_system::{
        EnsureRoot,
        RawOrigin,
    };
    use sp_core::H256;
    use sp_runtime::{
        testing::Header,
        traits::{
            BlakeTwo256,
            IdentityLookup,

        },
        DispatchError,
        DispatchResult,
        ModuleId,
        Perbill,
        Percent,
        Permill,
    };
    use std::cell::RefCell;
    // Import Trait for each runtime module being tested
    use chrono::NaiveDate;
    use node_template_runtime::{
        AccountId,
        Babe,
        Balance,
        BlockNumber,
        Moment,
        DAYS,
        SLOT_DURATION,
    };
    pub use pallet_transaction_payment::{
        CurrencyAdapter,
    };
    use mining_eligibility_proxy::{
        Event as MiningEligibilityProxyEvent,
        MiningEligibilityProxyClaimRewardeeData,
        MiningEligibilityProxyRewardRequest,
        Module as MiningEligibilityProxyModule,
        RewardDailyData,
        RewardRequestorData,
        RewardTransferData,
        Config as MiningEligibilityProxyConfig,
    };

    type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test where
            Block = Block,
            NodeBlock = Block,
            UncheckedExtrinsic = UncheckedExtrinsic,
        {
            System: frame_system::{Module, Call, Config, Storage, Event<T>},
            Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
            Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
            RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
            TransactionPayment: pallet_transaction_payment::{Module, Storage},
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const SS58Prefix: u8 = 33;
    }
    impl frame_system::Config for Test {
        type AccountData = pallet_balances::AccountData<u64>;
        type AccountId = u64;
        type BaseCallFilter = ();
        type BlockHashCount = BlockHashCount;
        type BlockLength = ();
        type BlockNumber = u64;
        type BlockWeights = ();
        type Call = Call;
        type DbWeight = ();
        // type WeightMultiplierUpdate = ();
        type Event = ();
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Header = Header;
        type Index = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type OnKilledAccount = ();
        type OnNewAccount = ();
        type Origin = Origin;
        type PalletInfo = PalletInfo;
        type SS58Prefix = SS58Prefix;
        type SystemWeightInfo = ();
        type Version = ();
    }
    parameter_types! {
        pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
    }
    impl pallet_timestamp::Config for Test {
        type MinimumPeriod = MinimumPeriod;
        /// A timestamp: milliseconds since the unix epoch.
        type Moment = Moment;
        type OnTimestampSet = Babe;
        type WeightInfo = ();
    }
    parameter_types! {
        pub const ExistentialDeposit: u64 = 1;
    }
    impl pallet_balances::Config for Test {
        type AccountStore = System;
        type Balance = u64;
        type DustRemoval = ();
        type Event = ();
        type ExistentialDeposit = ExistentialDeposit;
        type MaxLocks = ();
        type WeightInfo = ();
    }
    impl pallet_transaction_payment::Config for Test {
        type FeeMultiplierUpdate = ();
        type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
        type TransactionByteFee = ();
        type WeightToFee = IdentityFee<u64>;
    }

    impl MiningEligibilityProxyConfig for Test {
        type Event = ();
        type Currency = Balances;
        type Randomness = RandomnessCollectiveFlip;
        type MembershipSource = MembershipSupernodes;
        type MiningEligibilityProxyIndex = u64;
        type RewardsOfDay = u64;
    }

    pub type MiningEligibilityProxyTestModule = MiningEligibilityProxyModule<Test>;
    type Randomness = pallet_randomness_collective_flip::Module<Test>;

    pub fn new_test_ext() -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
        pallet_balances::GenesisConfig::<Test> {
            balances: vec![(0, INITIAL_DHX_DAO_TREASURY_UNLOCKED_RESERVES_BALANCE), (1, 10), (2, 20), (3, 30)],
        }
        .assimilate_storage(&mut t)
        .unwrap();
        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }

    #[test]
    fn setup_users() {
        new_test_ext().execute_with(|| {
            assert_eq!(Balances::free_balance(0), INITIAL_DHX_DAO_TREASURY_UNLOCKED_RESERVES_BALANCE);
            assert_eq!(Balances::free_balance(1), 10);
            assert_eq!(Balances::free_balance(2), 20);
            assert_eq!(Balances::free_balance(3), 30);
            assert_eq!(Balances::reserved_balance(&1), 0);
        });
    }

    #[test]
    fn integration_test() {
        new_test_ext().execute_with(|| {
            let rewardee_data = MiningEligibilityProxyClaimRewardeeData {
                proxy_claim_rewardee_account_id: 3,
                proxy_claim_reward_amount: 1000,
                proxy_claim_start_date: NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0).timestamp(),
                proxy_claim_end_date: NaiveDate::from_ymd(2000, 1, 6).and_hms(0, 0, 0).timestamp(),
            };
            let mut proxy_claim_rewardees_data: Vec<MiningEligibilityProxyClaimRewardeeData<u64, u64, i64, i64>> =
                Vec::new();
            proxy_claim_rewardees_data.push(rewardee_data);

            System::set_block_number(1);

            // 26th March 2021 @ ~2am is 1616724600000
            // where milliseconds/day         86400000
            Timestamp::set_timestamp(1616724600000u64);

            // This will generate mining_eligibility_proxy_id 0
            assert_ok!(MiningEligibilityProxyTestModule::proxy_eligibility_claim(
                Origin::signed(1),
                1000, // _proxy_claim_total_reward_amount
                Some(proxy_claim_rewardees_data.clone()),
            ));

            System::set_block_number(2);

            // 27th March 2021 @ ~2am is 1616811000000u64
            // https://currentmillis.com/
            Timestamp::set_timestamp(1616811000000u64);

            // Verify Storage
            assert_eq!(MiningEligibilityProxyTestModule::mining_eligibility_proxy_count(), 1);
            assert!(MiningEligibilityProxyTestModule::mining_eligibility_proxy(0).is_some());
            assert_eq!(MiningEligibilityProxyTestModule::mining_eligibility_proxy_owner(0), Some(1));

            // Check that data about the proxy claim and rewardee data has been stored.
            assert_eq!(
                MiningEligibilityProxyTestModule::mining_eligibility_proxy_eligibility_reward_requests(0),
                Some(MiningEligibilityProxyRewardRequest {
                    proxy_claim_requestor_account_id: 1u64,
                    proxy_claim_total_reward_amount: 1000u64,
                    proxy_claim_rewardees_data: proxy_claim_rewardees_data.clone(),
                    proxy_claim_timestamp_redeemed: 1616724600000u64, // current timestamp
                })
            );

            // Repeat with an additional claim
            assert_ok!(MiningEligibilityProxyTestModule::proxy_eligibility_claim(
                Origin::signed(2),
                3000, // _proxy_claim_total_reward_amount
                Some(proxy_claim_rewardees_data.clone()),
            ));

            let invalid_date_redeemed_millis_2021_01_15 = NaiveDate::from_ymd(2021, 01, 15).and_hms(0, 0, 0).timestamp() * 1000;
            let date_redeemed_millis_2021_03_26 = NaiveDate::from_ymd(2021, 03, 26).and_hms(0, 0, 0).timestamp() * 1000;
            let date_redeemed_millis_2021_03_27 = NaiveDate::from_ymd(2021, 03, 27).and_hms(0, 0, 0).timestamp() * 1000;

            if let Some(rewards_daily_data) = MiningEligibilityProxyTestModule::rewards_daily(
                date_redeemed_millis_2021_03_27.clone(),
            ) {
                // Check that data about the proxy claim reward daily data has been stored.
                // Check latest transfer added to vector for requestor AccountId 0
                assert_eq!(
                    rewards_daily_data.clone().pop(),
                    // TODO - instead of using `RewardDailyData` from the implementation, consider
                    // creating a mock of it instead and decorate it with `Debug` and so forth
                    // like in the implementation. It doesn't cause any errors at the moment
                    // because `RewardDailyData` only uses generics in the implementation,
                    // but if it was defined with specific types then it would generate errors
                    Some(RewardDailyData {
                        mining_eligibility_proxy_id: 1u64,
                        total_amt: 3000u64,
                        proxy_claim_requestor_account_id: 2u64,
                        member_kind: 1u32,
                        rewarded_date: date_redeemed_millis_2021_03_27.clone(),
                    })
                );
            } else {
                assert_eq!(false, true);
            }
        });
    }
}
