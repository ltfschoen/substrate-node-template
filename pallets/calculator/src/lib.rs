#![cfg_attr(not(feature = "std"), no_std)]

use substrate_fixed::types::{
	U32F32
};
use frame_support::{debug, decl_module, decl_storage, decl_event, decl_error, dispatch,
	dispatch::{
		DispatchError,
		DispatchResult,
	},
	traits::{
		Currency,
		Get
	},
};
use frame_system::ensure_signed;
use core::convert::TryInto;
use module_primitives::{
	constants::time::MILLISECS_PER_BLOCK,
};
use sp_std::prelude::*; // Imports Vec
#[macro_use]
extern crate alloc; // Required to use Vec

// #[cfg(test)]
// mod tests;

pub trait Config: frame_system::Config + pallet_balances::Config + pallet_timestamp::Config {
	type Currency: Currency<Self::AccountId>;
}

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Debug, Default, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive())]
pub struct RewardDailyData<U, V, W> {
	pub requestor_account_id: U,
	pub total_amt: V,
    pub rewarded_block: W,
}

type DailyData<T> = RewardDailyData<
	<T as frame_system::Config>::AccountId,
	BalanceOf<T>,
    <T as frame_system::Config>::BlockNumber,
>;

// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	trait Store for Module<T: Config> as CalculatorModule {
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items

		/// Returns reward data that has been distributed for a given day
		pub RewardsPerDay get(fn rewards_daily):
			map hasher(opaque_blake2_256) T::Moment =>
				Option<Vec<DailyData<T>>>;
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		NoneValue,
		/// Some math operation overflowed
		Overflow,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		fn update_fixed_daily_rewards(origin, reward_amount: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Ok(())
		}
	}
}
