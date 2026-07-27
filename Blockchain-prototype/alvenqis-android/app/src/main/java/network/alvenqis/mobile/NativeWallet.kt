package network.alvenqis.mobile

import android.os.Build

object NativeWallet {
    @Volatile private var loaded = false

    fun createWallet(): String {
        ensureLoaded()
        return createWalletNative()
    }

    fun importWallet(phrase: String): String {
        ensureLoaded()
        return importWalletNative(phrase)
    }

    @Synchronized
    private fun ensureLoaded() {
        if (loaded) return
        try {
            System.loadLibrary("alvenqis_mobile_core")
            loaded = true
        } catch (error: UnsatisfiedLinkError) {
            val abis = Build.SUPPORTED_ABIS.joinToString().ifBlank { "unknown" }
            throw IllegalStateException(
                "Alvenqis native wallet engine is unavailable for device ABI(s) [$abis]. " +
                    "Install a verified APK containing lib/<abi>/libalvenqis_mobile_core.so. " +
                    "Native loader: ${error.message}",
                error
            )
        }
    }

    @JvmStatic private external fun createWalletNative(): String
    @JvmStatic private external fun importWalletNative(phrase: String): String
}
