use alvenqis_core::{
    generate_mnemonic, Address, MnemonicWordCount, Network, PrivateKey, WalletDerivationPath,
};
use serde::Serialize;
use zeroize::Zeroize;

#[derive(Debug, Serialize)]
pub struct MobileWalletMaterial {
    pub schema: &'static str,
    pub network_id: &'static str,
    pub address: String,
    pub public_key_hex: String,
    pub derivation_path: String,
    pub mnemonic: String,
}

pub fn create_wallet_material() -> Result<MobileWalletMaterial, String> {
    let phrase =
        generate_mnemonic(MnemonicWordCount::TwentyFour).map_err(|error| error.to_string())?;
    wallet_material_from_mnemonic(phrase)
}

pub fn import_wallet_material(phrase: String) -> Result<MobileWalletMaterial, String> {
    let words = phrase.split_whitespace().count();
    if words != 12 && words != 24 {
        return Err("recovery phrase must contain 12 or 24 English words".into());
    }
    wallet_material_from_mnemonic(phrase)
}

fn wallet_material_from_mnemonic(mut phrase: String) -> Result<MobileWalletMaterial, String> {
    let result = PrivateKey::from_mnemonic(&phrase, "", WalletDerivationPath::default())
        .map_err(|error| error.to_string())
        .map(|key| {
            let public_key = key.public_key();
            MobileWalletMaterial {
                schema: "alvenqis-mobile-wallet-v1",
                network_id: Network::MainnetCandidate.network_id(),
                address: Address::from_public_key_for_network(
                    &public_key,
                    Network::MainnetCandidate,
                )
                .to_string(),
                public_key_hex: public_key.to_hex(),
                derivation_path: WalletDerivationPath::default().to_string(),
                mnemonic: phrase.clone(),
            }
        });
    phrase.zeroize();
    result
}

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use jni::{
        errors::ThrowRuntimeExAndDefault,
        objects::{JClass, JString},
        EnvUnowned,
    };

    #[no_mangle]
    pub extern "system" fn Java_network_alvenqis_mobile_NativeWallet_createWalletNative<'local>(
        mut unowned_env: EnvUnowned<'local>,
        _class: JClass<'local>,
    ) -> JString<'local> {
        unowned_env
            .with_env(|env| env.new_string(response(create_wallet_material())))
            .resolve::<ThrowRuntimeExAndDefault>()
    }

    #[no_mangle]
    pub extern "system" fn Java_network_alvenqis_mobile_NativeWallet_importWalletNative<'local>(
        mut unowned_env: EnvUnowned<'local>,
        _class: JClass<'local>,
        phrase: JString<'local>,
    ) -> JString<'local> {
        unowned_env
            .with_env(|env| {
                let phrase = phrase.try_to_string(env).map_err(|error| error.to_string());
                env.new_string(response(phrase.and_then(import_wallet_material)))
            })
            .resolve::<ThrowRuntimeExAndDefault>()
    }

    fn response(result: Result<MobileWalletMaterial, String>) -> String {
        match result {
            Ok(material) => serde_json::json!({ "ok": true, "wallet": material }).to_string(),
            Err(error) => serde_json::json!({ "ok": false, "error": error }).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_wallet_uses_mainnet_candidate_address() {
        let material = import_wallet_material("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art".into()).expect("wallet");
        assert_eq!(material.network_id, "alvenqis-mainnet-candidate");
        assert!(material.address.starts_with("alve1"));
    }

    #[test]
    fn generated_mobile_wallet_has_twenty_four_words() {
        let material = create_wallet_material().expect("wallet");
        assert_eq!(material.mnemonic.split_whitespace().count(), 24);
    }

    #[test]
    fn twelve_word_mobile_import_is_accepted() {
        let material = import_wallet_material(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
                .into(),
        )
        .expect("12-word wallet");
        assert!(material.address.starts_with("alve1"));
    }

    #[test]
    fn eleven_word_mobile_import_is_rejected() {
        let error = import_wallet_material(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon"
                .into(),
        )
        .expect_err("must reject");
        assert!(error.contains("12 or 24"));
    }
}
