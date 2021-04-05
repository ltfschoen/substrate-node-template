#![cfg_attr(not(feature = "std"), no_std)]

// use substrate_fixed::types::{
// 	U32F32
// };
use chrono::{
    NaiveDate,
    NaiveDateTime,
    Duration,
};
use codec::{
    Decode,
    Encode,
};
use frame_support::{debug, decl_module, decl_storage, decl_event, decl_error, dispatch, ensure,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
        Randomness,
    },
    Parameter,
};
use frame_system::ensure_signed;
use module_primitives::{
	constants::time::MILLISECS_PER_BLOCK,
};
use sp_io::hashing::blake2_128;
use sp_runtime::{
    print,
    traits::{
        AtLeast32Bit,
        Bounded,
        CheckedAdd,
        Member,
        One,
        Printable,
    },
    DispatchError,
};
// use core::convert::{
// 	TryFrom,
// 	TryInto,
// };
use sp_std::{
    convert::{
        TryFrom,
        TryInto,
    },
    prelude::*, // Imports Vec
};
#[macro_use]
extern crate alloc; // Required to use Vec

// #[cfg(test)]
// mod tests;

pub trait Config: frame_system::Config + pallet_balances::Config + pallet_timestamp::Config {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
	type Currency: Currency<Self::AccountId>;
	type Randomness: Randomness<Self::Hash>;
    type MiningEligibilityProxyIndex: Parameter + Member + AtLeast32Bit + Bounded + Default + Copy;
    type RewardsOfDay: Parameter + Member + AtLeast32Bit + Bounded + Default + Copy;
}

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type Date = i64;

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive())]
pub struct MiningEligibilityProxy(pub [u8; 16]);

#[derive(Encode, Decode, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive())]
pub struct MiningEligibilityProxyRewardRequest<U, V, W, X> {
    pub proxy_claim_requestor_account_id: U,
    pub proxy_claim_total_reward_amount: V,
    pub proxy_claim_rewardees_data: W,
    pub proxy_claim_timestamp_redeemed: X,
}

#[derive(Encode, Decode, Debug, Default, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive())]
pub struct MiningEligibilityProxyClaimRewardeeData<U, V, W, X> {
    pub proxy_claim_rewardee_account_id: U,
    pub proxy_claim_reward_amount: V,
    pub proxy_claim_start_date: W,
    pub proxy_claim_end_date: X,
}

#[derive(Encode, Decode, Debug, Default, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive())]
pub struct RewardDailyData<U, V, W, X, Y> {
	pub mining_eligibility_proxy_id: U,
	pub total_amt: V,
	pub proxy_claim_requestor_account_id: W,
	pub member_kind: X,
	pub rewarded_date: Y,
}

type RewardeeData<T> =
	MiningEligibilityProxyClaimRewardeeData<<T as frame_system::Config>::AccountId, BalanceOf<T>, Date, Date>;

type DailyData<T> = RewardDailyData<
	<T as Config>::MiningEligibilityProxyIndex,
	BalanceOf<T>,
	<T as frame_system::Config>::AccountId,
	u32,
	Date,
>;

decl_event!(
	pub enum Event<T> where
		AccountId = <T as frame_system::Config>::AccountId,
		<T as Config>::MiningEligibilityProxyIndex,
		BalanceOf = BalanceOf<T>,
		RewardeeData = RewardeeData<T>,
		DailyData = DailyData<T>,
    {
		Created(AccountId, MiningEligibilityProxyIndex),
        MiningEligibilityProxyRewardRequestSet(
            AccountId,
            MiningEligibilityProxyIndex,
            BalanceOf,
            Vec<RewardeeData>,
            Date,
        ),
        RewardsPerDaySet(
            Date,
            DailyData,
        ),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        NoneValue,
        /// Some math operation overflowed
        Overflow,
    }
}

// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	trait Store for Module<T: Config> as MiningEligibilityProxyModule {
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items

        pub MiningEligibilityProxys get(fn mining_eligibility_proxy): map hasher(opaque_blake2_256) T::MiningEligibilityProxyIndex => Option<MiningEligibilityProxy>;

		pub MiningEligibilityProxyCount get(fn mining_eligibility_proxy_count): T::MiningEligibilityProxyIndex;

		pub MiningEligibilityProxyOwners get(fn mining_eligibility_proxy_owner): map hasher(opaque_blake2_256) T::MiningEligibilityProxyIndex => Option<T::AccountId>;

		pub MiningEligibilityProxyRewardRequests get(fn mining_eligibility_proxy_eligibility_reward_requests):
			map hasher(opaque_blake2_256) T::MiningEligibilityProxyIndex =>
				Option<MiningEligibilityProxyRewardRequest<
					T::AccountId,
					BalanceOf<T>,
					Vec<RewardeeData<T>>, // TODO - change to store MiningEligibilityProxyRewardeeIndex that is created to store then instead
					<T as pallet_timestamp::Config>::Moment,
				>>;

		/// Returns reward data that has been distributed for a given day
        pub RewardsPerDay get(fn rewards_daily):
            map hasher(opaque_blake2_256) Date =>
                Option<Vec<RewardDailyData<
                    <T as Config>::MiningEligibilityProxyIndex,
                    BalanceOf<T>,
                    <T as frame_system::Config>::AccountId,
                    u32,
                    Date,
                >>>;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		fn proxy_eligibility_claim(
            origin,
            _proxy_claim_total_reward_amount: BalanceOf<T>,
            _proxy_claim_rewardees_data: Option<Vec<RewardeeData<T>>>,
        ) -> Result<(), DispatchError> {
            let sender = ensure_signed(origin)?;

			// get the current block & current date/time
			let current_block = <frame_system::Module<T>>::block_number();
			let requested_date = <pallet_timestamp::Module<T>>::get();

			// Temporary hard-coded for demo
			let member_kind = 0u32;
			let recipient_member_kind = 0u32;

            // let member_kind = T::MembershipSource::account_kind(sender.clone());
            // debug::info!("Requestor account kind: {:?}", member_kind.clone());

            // let recipient_member_kind = T::MembershipSource::account_kind(sender.clone());
            // debug::info!("Recipient account kind: {:?}", recipient_member_kind.clone());

            let mining_eligibility_proxy_id: T::MiningEligibilityProxyIndex;

            match Self::create(sender.clone()) {
                Ok(proxy_id) => {
                    mining_eligibility_proxy_id = proxy_id.into();
                },
                Err(_) => {
                    return Err(DispatchError::Other("Proxy claim rewardees data missing"));
                }
			}

            if let Some(rewardees_data) = _proxy_claim_rewardees_data {
                let reward_to_pay_as_balance_to_try = TryInto::<BalanceOf<T>>::try_into(_proxy_claim_total_reward_amount).ok();
                if let Some(reward_to_pay) = reward_to_pay_as_balance_to_try {
					let requested_date = <pallet_timestamp::Module<T>>::get();

					let requested_date_as_u64;
					if let Some(_requested_date_as_u64) = TryInto::<u64>::try_into(requested_date).ok() {
						requested_date_as_u64 = _requested_date_as_u64;
					} else {
						return Err(DispatchError::Other("Unable to convert Moment to i64 for requested_date"));
					}
					debug::info!("requested_date_as_u64: {:?}", requested_date_as_u64.clone());

					let requested_date_as_u64_secs = requested_date_as_u64.clone() / 1000u64;
					// https://docs.rs/chrono/0.4.6/chrono/naive/struct.NaiveDateTime.html#method.from_timestamp
					let sent_date = NaiveDateTime::from_timestamp(i64::try_from(requested_date_as_u64_secs).unwrap(), 0).date();
					debug::info!("requested_date_as_u64_secs: {:?}", requested_date_as_u64_secs.clone());
					debug::info!("sent_date: {:?}", sent_date.clone());

					let sent_date_millis = sent_date.and_hms(0, 0, 0).timestamp() * 1000;
					debug::info!("sent_date_millis: {:?}", sent_date_millis.clone());

					debug::info!("Timestamp sent Date: {:?}", sent_date);

					let reward_amount_item: DailyData<T> = RewardDailyData {
						mining_eligibility_proxy_id: mining_eligibility_proxy_id.clone(),
						total_amt: _proxy_claim_total_reward_amount.clone(),
						proxy_claim_requestor_account_id: sender.clone(),
						member_kind: recipient_member_kind.clone(),
						rewarded_date: sent_date_millis.clone(),
					};

					debug::info!("Appended new rewards_per_day storage item");

					<RewardsPerDay<T>>::append(
						sent_date_millis.clone(),
						reward_amount_item.clone(),
					);

					debug::info!("Appended new rewards_per_day at Date: {:?}", sent_date);
					debug::info!("Appended new rewards_per_day in storage item: {:?}", reward_amount_item.clone());

					let rewards_per_day_retrieved = <RewardsPerDay<T>>::get(
						sent_date_millis.clone(),
					);
					debug::info!("Retrieved new rewards_per_day storage item: {:?}", rewards_per_day_retrieved.clone());

					let reward_daily_data: DailyData<T> = RewardDailyData {
						mining_eligibility_proxy_id: mining_eligibility_proxy_id.clone(),
						total_amt: reward_to_pay.clone(),
						proxy_claim_requestor_account_id: sender.clone(),
						member_kind: member_kind.clone(),
						rewarded_date: sent_date_millis.clone(),
					};

					debug::info!("Setting the proxy eligibility reward daily");

					Self::insert_mining_eligibility_proxy_reward_daily(
						&sent_date_millis.clone(),
						reward_daily_data.clone(),
					);

					debug::info!("Inserted proxy_reward_daily for Moment: {:?}", requested_date.clone());
					debug::info!("Inserted proxy_reward_daily for Moment with Data: {:?}", reward_daily_data.clone());
                }

                debug::info!("Setting the proxy eligibility reward_request");

                Self::set_mining_eligibility_proxy_eligibility_reward_request(
                    sender.clone(),
                    mining_eligibility_proxy_id.clone(),
                    _proxy_claim_total_reward_amount.clone(),
                    rewardees_data.clone(),
                );

                debug::info!("Inserted proxy_eligibility_reward_request for Proxy ID: {:?}", mining_eligibility_proxy_id.clone());
                debug::info!("Inserted proxy_eligibility_reward_request for Proxy ID with reward amount: {:?}", _proxy_claim_total_reward_amount.clone());
                debug::info!("Inserted proxy_eligibility_reward_request for Proxy ID with rewardees_data: {:?}", rewardees_data.clone());

                return Ok(());
            } else {
                debug::info!("Proxy claim rewardees data missing");
                return Err(DispatchError::Other("Proxy claim rewardees data missing"));
            }
		}
	}
}

impl<T: Config> Module<T> {
	pub fn create(sender: T::AccountId) -> Result<T::MiningEligibilityProxyIndex, DispatchError> {
        let mining_eligibility_proxy_id = Self::next_mining_eligibility_proxy_id()?;

        // Generate a random 128bit value
        let unique_id = Self::random_value(&sender);

        // Create and store mining_eligibility_proxy
        let mining_eligibility_proxy = MiningEligibilityProxy(unique_id);
        Self::insert_mining_eligibility_proxy(&sender, mining_eligibility_proxy_id, mining_eligibility_proxy);

        Self::deposit_event(RawEvent::Created(sender, mining_eligibility_proxy_id));
        return Ok(mining_eligibility_proxy_id);
	}

	pub fn exists_mining_eligibility_proxy(
        mining_eligibility_proxy_id: T::MiningEligibilityProxyIndex,
    ) -> Result<(), DispatchError> {
        match Self::mining_eligibility_proxy(mining_eligibility_proxy_id) {
            Some(_value) => Ok(()),
            None => Err(DispatchError::Other("MiningEligibilityProxy does not exist")),
        }
	}

    pub fn has_value_for_mining_eligibility_proxy_reward_request_index(
        mining_eligibility_proxy_id: T::MiningEligibilityProxyIndex,
    ) -> Result<(), DispatchError> {
        debug::info!("Checking if mining_eligibility_proxy_reward_request has a value that is defined");
        let fetched_mining_eligibility_proxy_reward_request =
            <MiningEligibilityProxyRewardRequests<T>>::get(mining_eligibility_proxy_id);
        if let Some(_value) = fetched_mining_eligibility_proxy_reward_request {
            debug::info!("Found value for mining_eligibility_proxy_reward_request");
            return Ok(());
        }
        debug::info!("No value for mining_eligibility_proxy_reward_request");
        Err(DispatchError::Other("No value for mining_eligibility_proxy_reward_request"))
    }

    pub fn is_mining_eligibility_proxy_owner(
        mining_eligibility_proxy_id: T::MiningEligibilityProxyIndex,
        sender: T::AccountId,
    ) -> Result<(), DispatchError> {
        ensure!(
            Self::mining_eligibility_proxy_owner(&mining_eligibility_proxy_id)
                .map(|owner| owner == sender)
                .unwrap_or(false),
            "Sender is not owner of MiningEligibilityProxy"
        );
        Ok(())
    }

    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        let payload = (
            T::Randomness::random(&[0]),
            sender,
            <frame_system::Module<T>>::extrinsic_index(),
            <frame_system::Module<T>>::block_number(),
        );
        payload.using_encoded(blake2_128)
	}

    fn next_mining_eligibility_proxy_id() -> Result<T::MiningEligibilityProxyIndex, DispatchError> {
        let mining_eligibility_proxy_id = Self::mining_eligibility_proxy_count();
        if mining_eligibility_proxy_id == <T::MiningEligibilityProxyIndex as Bounded>::max_value() {
            return Err(DispatchError::Other("MiningEligibilityProxy count overflow"));
        }
        Ok(mining_eligibility_proxy_id)
	}

	fn insert_mining_eligibility_proxy(
        owner: &T::AccountId,
        mining_eligibility_proxy_id: T::MiningEligibilityProxyIndex,
        mining_eligibility_proxy: MiningEligibilityProxy,
    ) {
        // Create and store mining mining_eligibility_proxy
		<MiningEligibilityProxys<T>>::insert(mining_eligibility_proxy_id, mining_eligibility_proxy);
		<MiningEligibilityProxyCount<T>>::put(mining_eligibility_proxy_id + One::one());
		<MiningEligibilityProxyOwners<T>>::insert(mining_eligibility_proxy_id, owner.clone());
	}

    fn insert_mining_eligibility_proxy_reward_daily(sent_date: &Date, reward_daily_data: DailyData<T>) {
        debug::info!("Appending reward daily data");

        <RewardsPerDay<T>>::append(sent_date.clone(), &reward_daily_data.clone());

        Self::deposit_event(RawEvent::RewardsPerDaySet(sent_date.clone(), reward_daily_data.clone()));
	}

    fn set_mining_eligibility_proxy_eligibility_reward_request(
        _proxy_claim_requestor_account_id: T::AccountId,
        mining_eligibility_proxy_id: T::MiningEligibilityProxyIndex,
        _proxy_claim_total_reward_amount: BalanceOf<T>,
        _proxy_claim_rewardees_data: Vec<RewardeeData<T>>,
    ) {
        // Ensure that the mining_eligibility_proxy_id whose config we want to change actually exists
        let is_mining_eligibility_proxy = Self::exists_mining_eligibility_proxy(mining_eligibility_proxy_id);

        if !is_mining_eligibility_proxy.is_ok() {
            debug::info!("Error no supernode exists with given id");
        }

        // Ensure that the caller is owner of the mining_eligibility_proxy_reward_request they are trying to change
        Self::is_mining_eligibility_proxy_owner(mining_eligibility_proxy_id, _proxy_claim_requestor_account_id.clone());

        let proxy_claim_requestor_account_id = _proxy_claim_requestor_account_id.clone();
        let proxy_claim_total_reward_amount = _proxy_claim_total_reward_amount.clone();
        let proxy_claim_rewardees_data = _proxy_claim_rewardees_data.clone();
        let current_block = <frame_system::Module<T>>::block_number();
        let proxy_claim_block_redeemed = current_block;
        let proxy_claim_timestamp_redeemed = <pallet_timestamp::Module<T>>::get();

        // Check if a mining_eligibility_proxy_reward_request already exists with the given mining_eligibility_proxy_id
        // to determine whether to insert new or mutate existing.
        if Self::has_value_for_mining_eligibility_proxy_reward_request_index(mining_eligibility_proxy_id).is_ok() {
            debug::info!("Mutating values");
            <MiningEligibilityProxyRewardRequests<T>>::mutate(
                mining_eligibility_proxy_id,
                |mining_eligibility_proxy_reward_request| {
                    if let Some(_mining_eligibility_proxy_reward_request) = mining_eligibility_proxy_reward_request {
                        // Only update the value of a key in a KV pair if the corresponding parameter value has been
                        // provided
                        _mining_eligibility_proxy_reward_request.proxy_claim_requestor_account_id =
                            proxy_claim_requestor_account_id.clone();
                        _mining_eligibility_proxy_reward_request.proxy_claim_total_reward_amount =
                            proxy_claim_total_reward_amount.clone();
                        _mining_eligibility_proxy_reward_request.proxy_claim_rewardees_data =
                            proxy_claim_rewardees_data.clone();
                        _mining_eligibility_proxy_reward_request.proxy_claim_timestamp_redeemed =
                            proxy_claim_timestamp_redeemed.clone();
                    }
                },
            );

            debug::info!("Checking mutated values");
            let fetched_mining_eligibility_proxy_reward_request =
                <MiningEligibilityProxyRewardRequests<T>>::get(mining_eligibility_proxy_id);
            if let Some(_mining_eligibility_proxy_reward_request) = fetched_mining_eligibility_proxy_reward_request {
                debug::info!(
                    "Latest field proxy_claim_requestor_account_id {:#?}",
                    _mining_eligibility_proxy_reward_request.proxy_claim_requestor_account_id
                );
                debug::info!(
                    "Latest field proxy_claim_total_reward_amount {:#?}",
                    _mining_eligibility_proxy_reward_request.proxy_claim_total_reward_amount
                );
                // debug::info!(
                //     "Latest field proxy_claim_rewardees_data {:#?}",
                //     serde_json::to_string_pretty(&_mining_eligibility_proxy_reward_request.
                // proxy_claim_rewardees_data) );
                debug::info!(
                    "Latest field proxy_claim_timestamp_redeemed {:#?}",
                    _mining_eligibility_proxy_reward_request.proxy_claim_timestamp_redeemed
                );
            }
        } else {
            debug::info!("Inserting values");

            // Create a new mining mining_eligibility_proxy_reward_request instance with the input params
            let mining_eligibility_proxy_reward_request_instance = MiningEligibilityProxyRewardRequest {
                // Since each parameter passed into the function is optional (i.e. `Option`)
                // we will assign a default value if a parameter value is not provided.
                proxy_claim_requestor_account_id: proxy_claim_requestor_account_id.clone(),
                proxy_claim_total_reward_amount: proxy_claim_total_reward_amount.clone(),
                proxy_claim_rewardees_data: proxy_claim_rewardees_data.clone(),
                proxy_claim_timestamp_redeemed: proxy_claim_timestamp_redeemed.clone(),
            };

            <MiningEligibilityProxyRewardRequests<T>>::insert(
                mining_eligibility_proxy_id,
                &mining_eligibility_proxy_reward_request_instance,
            );

            debug::info!("Checking inserted values");
            let fetched_mining_eligibility_proxy_reward_request =
                <MiningEligibilityProxyRewardRequests<T>>::get(mining_eligibility_proxy_id);
            if let Some(_mining_eligibility_proxy_reward_request) = fetched_mining_eligibility_proxy_reward_request {
                debug::info!(
                    "Inserted field proxy_claim_requestor_account_id {:#?}",
                    _mining_eligibility_proxy_reward_request.proxy_claim_requestor_account_id
                );
                debug::info!(
                    "Inserted field proxy_claim_total_reward_amount {:#?}",
                    _mining_eligibility_proxy_reward_request.proxy_claim_total_reward_amount
                );
                // TODO
                // debug::info!(
                //     "Inserted field proxy_claim_rewardees_data {:#?}",
                //     serde_json::to_string_pretty(&_mining_eligibility_proxy_reward_request.
                // proxy_claim_rewardees_data) );
                debug::info!(
                    "Inserted field proxy_claim_timestamp_redeemed {:#?}",
                    _mining_eligibility_proxy_reward_request.proxy_claim_timestamp_redeemed
                );
            }
        }

        let proxy_claim_timestamp_redeemed_as_u64 =
            TryInto::<u64>::try_into(proxy_claim_timestamp_redeemed).ok().unwrap();
        let proxy_claim_date_redeemed = NaiveDateTime::from_timestamp(
            i64::try_from(proxy_claim_timestamp_redeemed_as_u64.clone() / 1000u64).unwrap(),
            0,
        )
        .date();

        let date_redeemed_millis = proxy_claim_date_redeemed.and_hms(0, 0, 0).timestamp() * 1000;
        debug::info!("proxy_claim_date_redeemed.timestamp {:#?}", date_redeemed_millis.clone());

        Self::deposit_event(RawEvent::MiningEligibilityProxyRewardRequestSet(
            proxy_claim_requestor_account_id,
            mining_eligibility_proxy_id,
            proxy_claim_total_reward_amount,
            proxy_claim_rewardees_data,
            date_redeemed_millis.clone(),
        ));
    }
}
