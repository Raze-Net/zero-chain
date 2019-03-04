use primitives::{Ed25519AuthorityId, ed25519};
use zero_chain_runtime::{
	AccountId, GenesisConfig, ConsensusConfig, TimestampConfig, BalancesConfig,
	SudoConfig, IndicesConfig, FeesConfig, ConfTransferConfig
};
use substrate_service;

use crate::pvk::PVK;
use zprimitives::{
	prepared_vk::PreparedVk,
	pkd_address::PkdAddress,
	ciphertext::Ciphertext,
	keys::{ExpandedSpendingKey, ViewingKey},
	};
use rand::{OsRng, Rng};
use jubjub::{curve::{JubjubBls12, FixedGenerators, fs, ToUniform}};
use zpairing::bls12_381::Bls12;
use zcrypto::elgamal::{self, elgamal_extend};

lazy_static! {
    static ref JUBJUB: JubjubBls12 = { JubjubBls12::new() };
}

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialised `ChainSpec`. This is a specialisation of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,
}

impl Alternative {
	/// Get an actual chain config from one of the alternatives.
	pub(crate) fn load(self) -> Result<ChainSpec, String> {		
		Ok(match self {
			Alternative::Development => ChainSpec::from_genesis(
				"Development",
				"dev",
				|| testnet_genesis(vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().into(),
				], vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
				],
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into()
				),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis(vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().into(),
					ed25519::Pair::from_seed(b"Bob                             ").public().into(),
				], vec![
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into(),
					ed25519::Pair::from_seed(b"Bob                             ").public().0.into(),
					ed25519::Pair::from_seed(b"Charlie                         ").public().0.into(),
					ed25519::Pair::from_seed(b"Dave                            ").public().0.into(),
					ed25519::Pair::from_seed(b"Eve                             ").public().0.into(),
					ed25519::Pair::from_seed(b"Ferdie                          ").public().0.into(),
				],
					ed25519::Pair::from_seed(b"Alice                           ").public().0.into()
				),
				vec![],
				None,
				None,
				None,
				None
			),
		})
	}

	pub(crate) fn from(s: &str) -> Option<Self> {
		match s {
			"dev" => Some(Alternative::Development),
			"" | "local" => Some(Alternative::LocalTestnet),
			_ => None,
		}
	}
}

fn testnet_genesis(initial_authorities: Vec<Ed25519AuthorityId>, endowed_accounts: Vec<AccountId>, root_key: AccountId) -> GenesisConfig {	
	GenesisConfig {
		consensus: Some(ConsensusConfig {
			// code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/node_template_runtime_wasm.compact.wasm").to_vec(),
			code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/zero_chain_runtime_wasm.wasm").to_vec(),
			authorities: initial_authorities.clone(),
		}),
		system: None,
		timestamp: Some(TimestampConfig {
			period: 5,					// 5 second block time.
		}),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.clone(),
		}),
		balances: Some(BalancesConfig {
			existential_deposit: 500,
			transfer_fee: 0,
			creation_fee: 0,
			balances: endowed_accounts.iter().map(|&k|(k, (1 << 60))).collect(),
			vesting: vec![],
		}),
		sudo: Some(SudoConfig {
			key: root_key,
		}),
		fees: Some(FeesConfig {
			transaction_base_fee: 1,
			transaction_byte_fee: 0,
		}),
		conf_transfer: Some(ConfTransferConfig {
			encrypted_balance: vec![alice_init()],
			// verifying_key: get_pvk(&PVK),
			verifying_key: PreparedVk(vec![1]),
			simple_num: 3 as u32,
			_genesis_phantom_data: Default::default(),
		})
	}
}

fn get_pvk(pvk_array: &[i32]) -> PreparedVk {
	let pvk_vec_u8: Vec<u8> = pvk_array.to_vec().into_iter().map(|e| e as u8).collect();	
	PreparedVk(pvk_vec_u8)	
}

fn alice_init() -> (PkdAddress, Ciphertext) {
	let alice_seed = b"Alice                           ";
	let alice_value = 100 as u32;

	let p_g = FixedGenerators::ElGamal;
	let mut randomness = [0u8; 32];

	if let Ok(mut e) = OsRng::new() {
		e.fill_bytes(&mut randomness[..]);
	}
	let r_fs = fs::Fs::to_uniform(elgamal_extend(&randomness).as_bytes());	

	let expsk = ExpandedSpendingKey::<Bls12>::from_spending_key(alice_seed);        
    let viewing_key = ViewingKey::<Bls12>::from_expanded_spending_key(&expsk, &JUBJUB);        
    let address = viewing_key.into_payment_address(&JUBJUB);	

	let enc_alice_val = elgamal::Ciphertext::encrypt(alice_value, r_fs, &address.0, p_g, &JUBJUB);

	(PkdAddress::from_payment_address(&address), Ciphertext::from_ciphertext(&enc_alice_val))
}
