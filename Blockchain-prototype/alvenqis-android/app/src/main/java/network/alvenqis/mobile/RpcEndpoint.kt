package network.alvenqis.mobile

import java.net.URI

object RpcEndpointPolicy {
    const val PUBLIC_RPC = "https://rpcnode.dohotstudio.com"
    const val USB_LOCAL_RPC = "http://127.0.0.1:10787"
    const val EMULATOR_RPC = "http://10.0.2.2:10787"
    private const val P2P_PORT = 20787

    fun normalize(raw: String): String {
        val trimmed = raw.trim().trimEnd('/')
        require(trimmed.isNotEmpty()) { "Enter a Alvenqis RPC endpoint." }
        val withScheme = if ("://" in trimmed) trimmed else "https://$trimmed"
        val uri = runCatching { URI(withScheme) }
            .getOrElse { throw IllegalArgumentException("Invalid RPC endpoint: ${it.message}") }
        require(uri.scheme == "http" || uri.scheme == "https") {
            "RPC endpoint must use http:// or https://."
        }
        require(!uri.host.isNullOrBlank()) { "RPC endpoint must include a valid host." }
        require(uri.port != P2P_PORT) {
            "Port 20787 is Alvenqis P2P, not HTTP RPC. Use https://rpcnode.dohotstudio.com for the public RPC."
        }
        require(uri.rawQuery == null && uri.rawFragment == null) {
            "RPC endpoint cannot contain a query or fragment."
        }
        require(uri.path.isNullOrEmpty() || uri.path == "/") {
            "Enter the RPC base URL without an API path."
        }
        return URI(uri.scheme, null, uri.host, uri.port, null, null, null).toString()
    }

}
