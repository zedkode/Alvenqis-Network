package network.alvenqis.mobile

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

data class SyncState(
    val mode: String,
    val localHeight: Long,
    val targetHeight: Long?,
    val remaining: Long?,
    val progress: Double?,
    val validatedPeers: Int,
    val detail: String
) {
    /** Allow panel when gateway has a usable tip (VPS-first clients). */
    val panelReady: Boolean
        get() = mode == "synced"
            || mode == "connected"
            || (localHeight > 0L && mode != "offline")
}

/** Extended read-only chain view for mobile dashboard parity with desktop. */
data class NetworkSnapshot(
    val networkId: String?,
    val statusLabel: String?,
    val height: Long?,
    val tipHash: String?,
    val integrityOk: Boolean?,
    val integrityDetail: String?,
    val indexInSync: Boolean?,
    val indexLag: Long?,
    val indexTip: String?,
    val balanceAtomic: Long?,
    val mempoolCount: Long?,
    val peerCount: Long?
)

data class BlockSummary(
    val height: Long,
    val hash: String,
    val timestamp: Long?,
    val txCount: Int?
)

data class PoolSnapshot(
    val poolName: String?,
    val statusLabel: String?,
    val acceptedShares: Long?,
    val blocksFound: Long?,
    val workers: Int?,
    val hashrateHs: Double?,
    val feeBps: Int?,
    val payoutScheme: String?,
    val upstreamStatus: String?
)

class SyncClient {
    suspend fun snapshot(baseUrl: String): SyncState = withContext(Dispatchers.IO) {
        val endpoint = RpcEndpointPolicy.normalize(baseUrl)
        val response = try {
            getJson("$endpoint/sync/status")
        } catch (error: RpcHttpException) {
            if (error.statusCode == HttpURLConnection.HTTP_NOT_FOUND) {
                return@withContext legacySnapshot(endpoint)
            }
            throw error
        }
        require(response.optString("network_id") == "alvenqis-mainnet-candidate") {
            "RPC belongs to a different Alvenqis network."
        }
        SyncState(
            mode = response.getString("sync_state"),
            localHeight = response.optNullableLong("local_height") ?: 0,
            targetHeight = response.optNullableLong("network_height"),
            remaining = response.optNullableLong("remaining_blocks"),
            progress = if (response.isNull("progress_percent")) null else response.getDouble("progress_percent"),
            validatedPeers = response.optInt("validated_peer_count", 0),
            detail = response.optString("detail", "RPC returned no synchronization detail")
        )
    }

    suspend fun networkSnapshot(baseUrl: String, address: String? = null): NetworkSnapshot =
        withContext(Dispatchers.IO) {
            val endpoint = RpcEndpointPolicy.normalize(baseUrl)
            val status = runCatching { getJson("$endpoint/status") }.getOrNull()
            val integrity = runCatching { getJson("$endpoint/chain/integrity") }.getOrNull()
            val indexer = runCatching { getJson("$endpoint/indexer/status") }.getOrNull()
            val balance = if (!address.isNullOrBlank()) {
                runCatching {
                    getJson("$endpoint/addresses/${java.net.URLEncoder.encode(address, "UTF-8")}/balance")
                        .optNullableLong("balance_atomic")
                }.getOrNull()
            } else {
                null
            }
            NetworkSnapshot(
                networkId = status?.optString("network_id"),
                statusLabel = status?.optString("status_label"),
                height = status?.optNullableLong("height"),
                tipHash = status?.optString("tip_hash")?.takeIf { it.isNotBlank() && it != "null" },
                integrityOk = integrity?.optBoolean("ok"),
                integrityDetail = integrity?.optString("detail"),
                indexInSync = indexer?.optBoolean("index_in_sync")
                    ?: indexer?.optBoolean("in_sync"),
                indexLag = indexer?.optNullableLong("lag_blocks"),
                indexTip = indexer?.optString("tip_hash")?.takeIf { it.isNotBlank() && it != "null" },
                balanceAtomic = balance,
                mempoolCount = status?.optNullableLong("mempool_count")
                    ?: status?.optNullableLong("mempool_size"),
                peerCount = status?.optNullableLong("connected_peer_count")
                    ?: status?.optNullableLong("peer_count")
            )
        }

    suspend fun recentBlocks(baseUrl: String, limit: Int = 12): List<BlockSummary> =
        withContext(Dispatchers.IO) {
            val endpoint = RpcEndpointPolicy.normalize(baseUrl)
            val body = runCatching { getJson("$endpoint/blocks?limit=$limit") }.getOrNull()
                ?: return@withContext emptyList()
            val arr: JSONArray = when {
                body.has("blocks") -> body.getJSONArray("blocks")
                body.has("items") -> body.getJSONArray("items")
                else -> return@withContext emptyList()
            }
            buildList {
                for (i in 0 until arr.length().coerceAtMost(limit)) {
                    val o = arr.getJSONObject(i)
                    val height = o.optNullableLong("height") ?: continue
                    val hash = o.optString("hash").takeIf { it.isNotBlank() } ?: continue
                    add(
                        BlockSummary(
                            height = height,
                            hash = hash,
                            timestamp = o.optNullableLong("timestamp"),
                            txCount = if (o.has("transaction_count")) o.optInt("transaction_count")
                            else if (o.has("tx_count")) o.optInt("tx_count")
                            else null
                        )
                    )
                }
            }
        }

    /** Public pool API via reverse proxy: {rpc}/pool/api/v1/pool/status */
    suspend fun poolSnapshot(baseUrl: String): PoolSnapshot? = withContext(Dispatchers.IO) {
        val endpoint = RpcEndpointPolicy.normalize(baseUrl)
        val url = if (endpoint.contains("rpcnode.dohotstudio.com")) {
            "$endpoint/pool/api/v1/pool/status"
        } else {
            // Local stack may expose pool separately; try relative path first.
            "$endpoint/pool/api/v1/pool/status"
        }
        val json = runCatching { getJson(url) }.getOrNull() ?: return@withContext null
        PoolSnapshot(
            poolName = json.optString("pool_name").takeIf { it.isNotBlank() },
            statusLabel = json.optString("status_label").takeIf { it.isNotBlank() },
            acceptedShares = json.optNullableLong("accepted_shares"),
            blocksFound = json.optNullableLong("blocks_found"),
            workers = json.optInt("connected_workers", -1).takeIf { it >= 0 },
            hashrateHs = if (json.has("estimated_hashrate_hs") && !json.isNull("estimated_hashrate_hs"))
                json.getDouble("estimated_hashrate_hs") else null,
            feeBps = if (json.has("pool_fee_basis_points")) json.optInt("pool_fee_basis_points") else null,
            payoutScheme = json.optString("payout_scheme").takeIf { it.isNotBlank() },
            upstreamStatus = json.optString("upstream_status").takeIf { it.isNotBlank() }
        )
    }

    private fun legacySnapshot(endpoint: String): SyncState {
        val status = getJson("$endpoint/status")
        require(status.optString("network_id") == "alvenqis-mainnet-candidate") {
            "RPC belongs to a different Alvenqis network."
        }
        require(status.optBoolean("initialized", true)) { "Remote Alvenqis chain is not initialized." }
        val height = status.optNullableLong("height") ?: 0
        return SyncState(
            mode = "connected",
            localHeight = height,
            targetHeight = height,
            remaining = 0,
            progress = 100.0,
            validatedPeers = 0,
            detail = "Connected to compatible RPC; peer synchronization aggregate is unavailable until the VPS RPC is upgraded"
        )
    }

    private fun getJson(url: String): JSONObject {
        val connection = URL(url).openConnection() as HttpURLConnection
        connection.connectTimeout = 5_000
        connection.readTimeout = 12_000
        connection.requestMethod = "GET"
        connection.setRequestProperty("Accept", "application/json")
        connection.setRequestProperty("User-Agent", "AlvenqisMobile/${BuildConfig.VERSION_NAME}")
        return try {
            val code = connection.responseCode
            val stream = if (code in 200..299) connection.inputStream else connection.errorStream
            val body = stream?.bufferedReader()?.use { it.readText() }.orEmpty()
            if (code !in 200..299) {
                throw RpcHttpException(
                    code,
                    "RPC returned HTTP $code${body.takeIf { it.isNotBlank() }?.let { ": ${it.take(160)}" }.orEmpty()}"
                )
            }
            JSONObject(body)
        } finally {
            connection.disconnect()
        }
    }
}

private class RpcHttpException(val statusCode: Int, message: String) : IllegalStateException(message)

private fun JSONObject.optNullableLong(name: String): Long? =
    if (isNull(name) || !has(name)) null else getLong(name)
