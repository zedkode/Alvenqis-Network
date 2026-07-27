package network.alvenqis.mobile

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import org.json.JSONArray
import org.json.JSONObject
import java.security.KeyStore
import java.util.concurrent.Executor
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlin.coroutines.suspendCoroutine

data class WalletProfile(
    val id: String,
    val name: String,
    val address: String,
    val publicKeyHex: String,
    val derivationPath: String
)

data class WalletMaterial(val profile: WalletProfile, val mnemonic: String)

/**
 * Encrypted wallet secret store backed by Android Keystore.
 *
 * AES-GCM key requires user authentication (biometric or device credential).
 * Callers must unlock via [unlockForCrypto] before encrypt/decrypt operations.
 */
class SecureWalletStore(private val context: Context) {
    private val preferences = context.getSharedPreferences("alvenqis_wallets_v1", Context.MODE_PRIVATE)
    /** Bumped when key policy changes so new installs get auth-required keys. */
    private val keyAlias = "alvenqis-mobile-wallet-encryption-v2"
    private val legacyKeyAlias = "alvenqis-mobile-wallet-encryption-v1"

    @Volatile
    private var unlockedCipherEncrypt: Cipher? = null

    @Volatile
    private var unlockedUntilMs: Long = 0L

    fun wallets(): List<WalletProfile> {
        val array = JSONArray(preferences.getString("profiles", "[]"))
        return (0 until array.length()).map { index -> profileFromJson(array.getJSONObject(index)) }
    }

    fun activeWallet(): WalletProfile? {
        val active = preferences.getString("active", null)
        return wallets().firstOrNull { it.id == active } ?: wallets().firstOrNull()
    }

    fun select(id: String) {
        require(wallets().any { it.id == id }) { "Wallet does not exist" }
        preferences.edit().putString("active", id).apply()
    }

    /**
     * Whether the Android Keystore key requires biometric / device credential.
     * Always true for newly generated v2 keys.
     */
    fun requiresUserAuthentication(): Boolean = true

    fun canAuthenticate(activity: FragmentActivity): Boolean {
        val manager = BiometricManager.from(activity)
        val authenticators =
            BiometricManager.Authenticators.BIOMETRIC_STRONG or
                BiometricManager.Authenticators.DEVICE_CREDENTIAL
        return manager.canAuthenticate(authenticators) == BiometricManager.BIOMETRIC_SUCCESS ||
            manager.canAuthenticate(BiometricManager.Authenticators.DEVICE_CREDENTIAL) ==
            BiometricManager.BIOMETRIC_SUCCESS
    }

    /**
     * Prompt the user to unlock the encryption key (biometric or device PIN/pattern/password).
     * After success, encrypt/decrypt may proceed for a short validity window.
     */
    suspend fun unlockForCrypto(activity: FragmentActivity, title: String = "Unlock Alvenqis wallet"): Unit =
        suspendCoroutine { cont ->
            val executor: Executor = ContextCompat.getMainExecutor(activity)
            val authenticators =
                BiometricManager.Authenticators.BIOMETRIC_STRONG or
                    BiometricManager.Authenticators.DEVICE_CREDENTIAL

            val promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle(title)
                .setSubtitle("Confirm it is you to access wallet secrets")
                .setAllowedAuthenticators(authenticators)
                .build()

            val prompt = BiometricPrompt(
                activity,
                executor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        try {
                            // Validity-duration keys unlock after any successful user auth.
                            unlockedCipherEncrypt = null
                            unlockedUntilMs = System.currentTimeMillis() + AUTH_VALIDITY_MS
                            cont.resume(Unit)
                        } catch (error: Exception) {
                            cont.resumeWithException(error)
                        }
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        cont.resumeWithException(
                            SecurityException("Wallet unlock failed: $errString")
                        )
                    }

                    override fun onAuthenticationFailed() {
                        // Keep waiting for another attempt; terminal outcomes use onAuthenticationError.
                    }
                }
            )
            prompt.authenticate(promptInfo)
        }

    fun isUnlocked(): Boolean = System.currentTimeMillis() < unlockedUntilMs

    fun lock() {
        unlockedCipherEncrypt = null
        unlockedUntilMs = 0L
    }

    /**
     * Store a wallet created/imported via the native FFI response.
     * Requires a prior successful [unlockForCrypto] within the validity window.
     */
    fun store(name: String, nativeResponse: String): WalletMaterial {
        ensureUnlocked("store wallet")
        val response = JSONObject(nativeResponse)
        require(response.optBoolean("ok")) { response.optString("error", "Wallet operation failed") }
        val wallet = response.getJSONObject("wallet")
        val address = wallet.getString("address")
        val profile = WalletProfile(
            address,
            name.trim(),
            address,
            wallet.getString("public_key_hex"),
            wallet.getString("derivation_path")
        )
        val mnemonic = wallet.getString("mnemonic")
        val updated = wallets().filterNot { it.id == profile.id } + profile
        val profiles = JSONArray().also { array -> updated.forEach { array.put(profileToJson(it)) } }
        preferences.edit()
            .putString("profiles", profiles.toString())
            .putString("active", profile.id)
            .putString("secret_${profile.id}", encrypt(mnemonic))
            .apply()
        return WalletMaterial(profile, mnemonic)
    }

    /**
     * Decrypt recovery words for a wallet. Requires [unlockForCrypto] first.
     * Use for explicit "reveal / unlock to sign" flows — never call silently.
     */
    fun mnemonic(id: String): String {
        ensureUnlocked("reveal recovery phrase")
        val encrypted = requireNotNull(preferences.getString("secret_$id", null)) {
            "Encrypted wallet material is missing"
        }
        return decrypt(encrypted)
    }

    private fun ensureUnlocked(action: String) {
        if (!isUnlocked()) {
            throw SecurityException(
                "Unlock required before $action. Use biometric or device credential."
            )
        }
    }

    private fun encryptionKey(): SecretKey {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (keyStore.getKey(keyAlias, null) as? SecretKey)?.let { return it }

        // Prefer migrating from legacy unauthenticated key only for decrypt of old blobs;
        // new material always uses the auth-required v2 key.
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").run {
            val builder = KeyGenParameterSpec.Builder(
                keyAlias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(true)
                .setInvalidatedByBiometricEnrollment(false)

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                builder.setUserAuthenticationParameters(
                    AUTH_VALIDITY_SECONDS,
                    KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL
                )
            } else {
                @Suppress("DEPRECATION")
                builder.setUserAuthenticationValidityDurationSeconds(AUTH_VALIDITY_SECONDS)
            }
            init(builder.build())
            generateKey()
        }
    }

    private fun legacyEncryptionKeyOrNull(): SecretKey? {
        val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        return keyStore.getKey(legacyKeyAlias, null) as? SecretKey
    }

    private fun encrypt(value: String): String {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, encryptionKey())
        val payload = JSONObject()
            .put("v", 2)
            .put("iv", Base64.encodeToString(cipher.iv, Base64.NO_WRAP))
            .put(
                "ciphertext",
                Base64.encodeToString(
                    cipher.doFinal(value.toByteArray(Charsets.UTF_8)),
                    Base64.NO_WRAP
                )
            )
        return payload.toString()
    }

    private fun decrypt(value: String): String {
        val payload = JSONObject(value)
        val version = payload.optInt("v", 1)
        val key = if (version >= 2) {
            encryptionKey()
        } else {
            legacyEncryptionKeyOrNull()
                ?: error("Legacy wallet secret present but v1 key is missing; re-import the wallet")
        }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            key,
            GCMParameterSpec(128, Base64.decode(payload.getString("iv"), Base64.NO_WRAP))
        )
        return cipher.doFinal(Base64.decode(payload.getString("ciphertext"), Base64.NO_WRAP))
            .toString(Charsets.UTF_8)
    }

    private fun profileToJson(profile: WalletProfile) = JSONObject()
        .put("id", profile.id)
        .put("name", profile.name)
        .put("address", profile.address)
        .put("public_key_hex", profile.publicKeyHex)
        .put("derivation_path", profile.derivationPath)

    private fun profileFromJson(json: JSONObject) = WalletProfile(
        json.getString("id"),
        json.getString("name"),
        json.getString("address"),
        json.getString("public_key_hex"),
        json.getString("derivation_path")
    )

    companion object {
        /** Auth validity window after a successful biometric / device-credential unlock. */
        private const val AUTH_VALIDITY_SECONDS = 60
        private const val AUTH_VALIDITY_MS = AUTH_VALIDITY_SECONDS * 1000L
    }
}
