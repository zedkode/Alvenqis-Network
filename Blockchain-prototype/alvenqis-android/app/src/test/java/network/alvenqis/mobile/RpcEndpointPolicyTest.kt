package network.alvenqis.mobile

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class RpcEndpointPolicyTest {
    @Test
    fun publicDomainDefaultsToHttps() {
        assertEquals(
            "https://rpcnode.dohotstudio.com",
            RpcEndpointPolicy.normalize("rpcnode.dohotstudio.com")
        )
    }

    @Test
    fun p2pPortIsRejectedAsRpc() {
        val error = assertThrows(IllegalArgumentException::class.java) {
            RpcEndpointPolicy.normalize("rpcnode.dohotstudio.com:20787")
        }
        assertTrue(error.message!!.contains("P2P"))
    }

    @Test
    fun localRpcEndpointsRemainExplicit() {
        assertEquals(
            "http://127.0.0.1:10787",
            RpcEndpointPolicy.normalize("http://127.0.0.1:10787/")
        )
        assertEquals(
            "http://10.0.2.2:10787",
            RpcEndpointPolicy.normalize("http://10.0.2.2:10787")
        )
    }
}
