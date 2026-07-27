package network.alvenqis.mobile

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class NativeWalletInstrumentedTest {
    @Test
    fun nativeWalletLibraryCreatesAndImportsWalletMaterial() {
        val created = JSONObject(NativeWallet.createWallet())
        assertTrue(created.getBoolean("ok"))

        val createdWallet = created.getJSONObject("wallet")
        val mnemonic = createdWallet.getString("mnemonic")
        assertEquals(24, mnemonic.trim().split(Regex("\\s+")).size)
        assertTrue(createdWallet.getString("address").startsWith("alve1"))

        val imported = JSONObject(NativeWallet.importWallet(mnemonic))
        assertTrue(imported.getBoolean("ok"))
        assertEquals(
            createdWallet.getString("address"),
            imported.getJSONObject("wallet").getString("address")
        )
    }

    @Test
    fun twelveWordImportIsAcceptedOnDevice() {
        val phrase =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
        val imported = JSONObject(NativeWallet.importWallet(phrase))
        assertTrue(imported.getBoolean("ok"))
        assertTrue(imported.getJSONObject("wallet").getString("address").startsWith("alve1"))
    }
}
