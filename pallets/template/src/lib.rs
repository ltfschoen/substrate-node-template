#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame

use codec::{Decode, Encode, EncodeLike, Compact};
use frame_support::{debug, decl_module, decl_storage, decl_event, decl_error, dispatch, traits::{Get, Randomness}, Parameter};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{
        AtLeast32Bit,
		Bounded,
		Member,
		One,
		Zero,
    },
};
use sp_io::hashing::blake2_128;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
	/// Because this pallet emits events, it depends on the runtime's definition of an event.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
	// Note: Into<64> is required so the `for` loop compiles
	type SomethingCountIndex: Parameter + Member + AtLeast32Bit + Bounded + Default + Copy + Encode + Decode + Into<u64>;
	type SomethingConfigIndex: Parameter + Member + AtLeast32Bit + Bounded + Default + Copy + Encode + Decode;
	type SomethingTeamResultIndex: Parameter + Member + AtLeast32Bit + Bounded + Default + Copy + Encode + Decode + From<u32>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct SomethingTeamResult<U, V> {
    pub something_team_manager_account_id: U,
    pub something_team_member_data: V,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct SomethingTeamMemberData<U, V> {
    pub something_team_member_account_id: U,
    pub something_team_member_joined_block: V,
}

type MemberData<T> = SomethingTeamMemberData<
    <T as frame_system::Config>::AccountId,
    <T as frame_system::Config>::BlockNumber,
>;

// The pallet's runtime storage items.
// https://substrate.dev/docs/en/knowledgebase/runtime/storage
decl_storage! {
	// A unique name is used to ensure that the pallet's storage items are isolated.
	// This name may be updated, but each pallet in the runtime must use a unique name.
	// ---------------------------------vvvvvvvvvvvvvv
	trait Store for Module<T: Config> as TemplateModule {
		// Learn more about declaring storage items:
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
		Something get(fn something): Option<u32>;

		// Stores the amount of somethings
		pub SomethingCount get(fn something_count): T::SomethingCountIndex;

		pub SomethingConfig get(fn something_config): map hasher(opaque_blake2_256) T::SomethingConfigIndex =>
		Option<T::BlockNumber>;

		pub SomethingTeamResults get(fn something_team_result): map hasher(opaque_blake2_256) T::SomethingTeamResultIndex =>
		Option<SomethingTeamResult<
			T::AccountId,
			Vec<MemberData<T>>,
		>>;
	}
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
	pub enum Event<T> where
		AccountId = <T as frame_system::Config>::AccountId,
		MemberData = MemberData<T>,
	{
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, AccountId),

		SomethingTeamResultSet(
			AccountId,
			MemberData,
		),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Config> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

		fn on_finalize(current_block_number: T::BlockNumber) {
            debug::info!("current block number {:#?}", current_block_number);

			let something_count = Self::something_count();

			<SomethingCount<T>>::put(something_count + One::one()); // One::one() or 1u32.into()

			let index: T::SomethingConfigIndex = 0u32.into();
			<SomethingConfig<T>>::insert(index, current_block_number);

            for idx in 0..something_count.into() {
				debug::info!("idx {:#?}", idx);

				if let Some(_some_idx) = Self::something() {
					let zero: T::SomethingConfigIndex = Zero::zero();
					if let Some(_some_config_idx) = Self::something_config(zero) {
						debug::info!("_some_config_idx {:#?}", _some_config_idx);
					}
				} else {
					debug::info!("empty index");
				}
			}
		}

		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[weight = 10_000 + T::DbWeight::get().writes(1)]
		pub fn do_something(
			origin,
			something: u32,
			member_data: Vec<MemberData<T>>,
		) -> dispatch::DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://substrate.dev/docs/en/knowledgebase/runtime/origin
			let who = ensure_signed(origin)?;

			let current_block = <frame_system::Module<T>>::block_number();

			// Update storage.
			Something::put(something);

			let something_team_result_instance = SomethingTeamResult {
				something_team_manager_account_id: who,
				something_team_member_data: member_data.clone(),
			};

			// a random 128bit value
			let unique_id = Self::random_value(&who);

			<SomethingTeamResults<T>>::insert(
				unique_id,
				something_team_result_instance.clone(),
			);

			// Emit an event.
			Self::deposit_event(RawEvent::SomethingStored(something, who));

			Self::deposit_event(RawEvent::SomethingTeamResultSet(
				who,
				something_team_result_instance.clone(),
			));
			// Return a successful DispatchResult
			Ok(())

		}

		/// An example dispatchable that may throw a custom error.
		#[weight = 10_000 + T::DbWeight::get().reads_writes(1,1)]
		pub fn cause_error(origin) -> dispatch::DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match Something::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue)?,
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					Something::put(new);
					Ok(())
				},
			}
		}
	}
}

impl<T: Config> Module<T> {
    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        let payload = (
            T::Randomness::random(&[0]),
            sender,
            <frame_system::Module<T>>::extrinsic_index(),
            <frame_system::Module<T>>::block_number(),
        );
        payload.using_encoded(blake2_128)
    }
}
