package network.alvenqis.mobile

import android.content.ClipData
import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.platform.toClipEntry
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.WindowCompat
import androidx.fragment.app.FragmentActivity
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private val Deep = Color(0xFF01080D)
private val Panel = Color(0xFF081B26)
private val PanelRaised = Color(0xFF0B2431)
private val Border = Color(0xFF173D4E)
private val Cyan = Color(0xFF20D5FF)
private val Gold = Color(0xFFE1B05B)
private val Muted = Color(0xFF7B95A4)
private val TextPrimary = Color(0xFFDAEBF5)
private val Positive = Color(0xFF52D37E)
private val Negative = Color(0xFFE5484D)
private val WarnBg = Color(0xFF2A1A08)

/** Tabs mirror desktop Control Center (read-only subset on mobile). */
private enum class MobileTab(val label: String) {
    Overview("Overview"),
    Wallet("Wallet"),
    Network("Network"),
    Explorer("Explorer"),
    Pool("Pool"),
    Mining("Mining")
}

class MainActivity : FragmentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        WindowCompat.getInsetsController(window, window.decorView).isAppearanceLightStatusBars = false
        val store = SecureWalletStore(this)
        setContent {
            MaterialTheme(
                colorScheme = darkColorScheme(
                    primary = Cyan,
                    secondary = Gold,
                    background = Deep,
                    surface = Panel,
                    onPrimary = Color(0xFF031018),
                    onBackground = TextPrimary,
                    onSurface = TextPrimary
                )
            ) {
                Surface(modifier = Modifier.fillMaxSize(), color = Deep) {
                    AlvenqisApp(store, this@MainActivity)
                }
            }
        }
    }
}

@Composable
private fun AlvenqisApp(store: SecureWalletStore, activity: FragmentActivity) {
    var wallets by remember { mutableStateOf(store.wallets()) }
    var active by remember { mutableStateOf(store.activeWallet()) }
    var stage by remember { mutableStateOf(if (active == null) "wallet" else "sync") }
    var name by remember { mutableStateOf("Primary wallet") }
    var importPhrase by remember { mutableStateOf("") }
    var recoveryPhrase by remember { mutableStateOf<String?>(null) }
    var endpoint by remember { mutableStateOf(BuildConfig.PUBLIC_RPC) }
    var sync by remember { mutableStateOf<SyncState?>(null) }
    var network by remember { mutableStateOf<NetworkSnapshot?>(null) }
    var blocks by remember { mutableStateOf<List<BlockSummary>>(emptyList()) }
    var pool by remember { mutableStateOf<PoolSnapshot?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var panelOpen by remember { mutableStateOf(false) }
    var unlocked by remember { mutableStateOf(store.isUnlocked()) }
    val scope = rememberCoroutineScope()

    fun refreshWallets() {
        wallets = store.wallets()
        active = store.activeWallet()
    }

    fun unlockThen(title: String, block: () -> Unit) {
        scope.launch {
            runCatching {
                if (!store.isUnlocked()) {
                    store.unlockForCrypto(activity, title)
                }
                unlocked = store.isUnlocked()
                block()
            }.onFailure { error = it.message }
        }
    }

    fun createWallet() {
        unlockThen("Unlock to create wallet") {
            runCatching { store.store(name, NativeWallet.createWallet()) }
                .onSuccess {
                    recoveryPhrase = it.mnemonic
                    refreshWallets()
                    error = null
                }.onFailure { error = it.message }
        }
    }

    fun importWallet() {
        unlockThen("Unlock to import wallet") {
            runCatching { store.store(name, NativeWallet.importWallet(importPhrase.trim())) }
                .onSuccess {
                    importPhrase = ""
                    recoveryPhrase = null
                    refreshWallets()
                    stage = "sync"
                    error = null
                }.onFailure { error = it.message }
        }
    }

    fun revealRecovery(walletId: String) {
        unlockThen("Unlock to reveal recovery phrase") {
            runCatching { store.mnemonic(walletId) }
                .onSuccess {
                    recoveryPhrase = it
                    error = null
                }.onFailure { error = it.message }
        }
    }

    fun checkSync() {
        scope.launch {
            runCatching {
                val client = SyncClient()
                val s = client.snapshot(endpoint)
                val n = client.networkSnapshot(endpoint, active?.address)
                val b = client.recentBlocks(endpoint, 10)
                val p = client.poolSnapshot(endpoint)
                Quad(s, n, b, p)
            }
                .onSuccess { (s, n, b, p) ->
                    sync = s
                    network = n
                    blocks = b
                    pool = p
                    error = null
                }.onFailure {
                    sync = null
                    network = null
                    blocks = emptyList()
                    pool = null
                    error = "RPC connection failed: ${it.message}"
                }
        }
    }

    LaunchedEffect(stage, endpoint, panelOpen) {
        if (stage == "sync" || panelOpen) {
            while (true) {
                checkSync()
                delay(3_000)
            }
        }
    }

    if (panelOpen) {
        MobileDashboard(
            wallet = active,
            sync = sync,
            network = network,
            blocks = blocks,
            pool = pool,
            endpoint = endpoint,
            onEndpointChange = { endpoint = it },
            onRefresh = { checkSync() },
            onLock = {
                store.lock()
                unlocked = false
                recoveryPhrase = null
                panelOpen = false
                stage = "wallet"
            }
        )
    } else {
        Box(
            Modifier
                .fillMaxSize()
                .background(Brush.verticalGradient(listOf(Deep, Color(0xFF03141C), Deep)))
                .statusBarsPadding()
                .navigationBarsPadding()
                .padding(20.dp),
            contentAlignment = Alignment.Center
        ) {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .widthIn(max = 720.dp)
                    .border(1.dp, Cyan.copy(alpha = 0.35f), RoundedCornerShape(22.dp)),
                colors = CardDefaults.cardColors(containerColor = Panel),
                shape = RoundedCornerShape(22.dp)
            ) {
                Column(
                    Modifier
                        .padding(24.dp)
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(14.dp)
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Image(
                            painter = painterResource(id = R.drawable.logo),
                            contentDescription = "Alvenqis",
                            modifier = Modifier
                                .size(52.dp)
                                .clip(RoundedCornerShape(14.dp))
                                .border(1.dp, Cyan.copy(alpha = 0.4f), RoundedCornerShape(14.dp))
                                .background(PanelRaised)
                                .padding(6.dp),
                            contentScale = ContentScale.Fit
                        )
                        Column {
                            Text(
                                "ALVENQIS MOBILE · v${BuildConfig.VERSION_NAME}",
                                color = Cyan,
                                style = MaterialTheme.typography.labelMedium,
                                fontFamily = FontFamily.Monospace,
                                letterSpacing = 1.5.sp
                            )
                            Text(
                                if (stage == "wallet") "Choose your wallet" else "Verify gateway sync",
                                style = MaterialTheme.typography.headlineSmall,
                                fontWeight = FontWeight.SemiBold
                            )
                            Text("Android 12+ · wallet + monitor (no mining)", color = Muted, fontSize = 11.sp)
                        }
                    }
                    MobileMiningBanBanner(compact = true)
                    OutlinedTextField(
                        value = endpoint,
                        onValueChange = { endpoint = it },
                        label = { Text("RPC base URL") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        supportingText = { Text("Default: official VPS · HTTPS", color = Muted, fontSize = 10.sp) }
                    )

                    if (stage == "wallet") {
                        LazyColumn(
                            modifier = Modifier.heightIn(max = 200.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            items(wallets) { wallet ->
                                OutlinedButton(
                                    onClick = {
                                        store.select(wallet.id)
                                        refreshWallets()
                                    },
                                    modifier = Modifier.fillMaxWidth(),
                                    colors = ButtonDefaults.outlinedButtonColors(contentColor = TextPrimary)
                                ) {
                                    Column(Modifier.weight(1f), horizontalAlignment = Alignment.Start) {
                                        Text(wallet.name, fontWeight = FontWeight.Medium)
                                        Text(
                                            "${wallet.address.take(16)}…${wallet.address.takeLast(8)}",
                                            color = Muted,
                                            fontFamily = FontFamily.Monospace,
                                            fontSize = 11.sp
                                        )
                                    }
                                    if (wallet.id == active?.id) {
                                        Text("ACTIVE", color = Cyan, fontSize = 11.sp)
                                    }
                                }
                            }
                        }
                        OutlinedTextField(
                            value = name,
                            onValueChange = { name = it },
                            label = { Text("Wallet name") },
                            modifier = Modifier.fillMaxWidth(),
                            singleLine = true
                        )
                        OutlinedTextField(
                            value = importPhrase,
                            onValueChange = { importPhrase = it },
                            label = { Text("Import 12 or 24-word phrase") },
                            modifier = Modifier.fillMaxWidth(),
                            minLines = 2
                        )
                        Text(
                            if (unlocked) "Wallet secrets unlocked (60s)"
                            else "Create / import / reveal requires biometric or device PIN",
                            color = if (unlocked) Positive else Muted,
                            fontSize = 11.sp
                        )
                        recoveryPhrase?.let {
                            Text("Backup these words offline:", color = Gold, fontSize = 12.sp)
                            Text(it, fontFamily = FontFamily.Monospace, fontSize = 12.sp, color = TextPrimary)
                        }
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Button(
                                onClick = { createWallet() },
                                colors = ButtonDefaults.buttonColors(containerColor = Cyan, contentColor = Color(0xFF031018))
                            ) { Text("Create 24-word") }
                            OutlinedButton(onClick = { importWallet() }) { Text("Import") }
                            Button(
                                onClick = { stage = "sync" },
                                enabled = active != null,
                                colors = ButtonDefaults.buttonColors(containerColor = PanelRaised, contentColor = Cyan)
                            ) { Text("Continue") }
                        }
                        if (active != null && recoveryPhrase == null) {
                            TextButton(onClick = { revealRecovery(active!!.id) }) {
                                Text("Unlock to reveal recovery words", color = Gold, fontSize = 12.sp)
                            }
                        }
                    } else {
                        val height = sync?.localHeight
                        val ready = sync?.panelReady == true || (sync != null && sync!!.localHeight > 0)
                        Text(
                            if (sync == null) "Waiting for VPS RPC…"
                            else "Gateway ${sync!!.mode} · height ${height ?: "—"}",
                            color = if (ready) Positive else Muted
                        )
                        LinearProgressIndicator(
                            progress = {
                                when {
                                    sync?.progress != null -> ((sync!!.progress!! / 100.0).coerceIn(0.0, 1.0)).toFloat()
                                    ready -> 1f
                                    else -> 0.35f
                                }
                            },
                            modifier = Modifier.fillMaxWidth().height(8.dp).clip(RoundedCornerShape(8.dp)),
                            color = Cyan,
                            trackColor = Border
                        )
                        Text(
                            "Validated peers ${sync?.validatedPeers ?: 0} · ${sync?.detail ?: "checking…"}",
                            color = Muted,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 11.sp
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            OutlinedButton(onClick = { stage = "wallet" }) { Text("Wallet") }
                            Button(
                                onClick = { panelOpen = true },
                                enabled = ready && active != null,
                                colors = ButtonDefaults.buttonColors(containerColor = Cyan, contentColor = Color(0xFF031018))
                            ) { Text("Open panel") }
                        }
                    }
                    error?.let { Text(it, color = Negative, fontSize = 12.sp) }
                }
            }
        }
    }
}

private data class Quad<A, B, C, D>(val a: A, val b: B, val c: C, val d: D)

@Composable
private fun MobileMiningBanBanner(compact: Boolean = false) {
    Column(
        Modifier
            .fillMaxWidth()
            .border(1.dp, Gold.copy(alpha = 0.55f), RoundedCornerShape(14.dp))
            .background(WarnBg, RoundedCornerShape(14.dp))
            .padding(if (compact) 12.dp else 16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Text(
            "MOBILE PLATFORMS ARE NOT FOR MINING",
            color = Gold,
            fontWeight = FontWeight.Bold,
            fontSize = if (compact) 12.sp else 13.sp,
            letterSpacing = 0.8.sp
        )
        Text(
            if (compact) {
                "Android is wallet + network monitor only. Mining stays on Windows/Linux Control Center (GPU CUDA/OpenCL)."
            } else {
                "Phones and tablets must not mine Alvenqis. This app has no miner start/stop, no share submit, and no local PoW. Use Windows or Linux Control Center for FiroPoW GPU mining."
            },
            color = TextPrimary.copy(alpha = 0.92f),
            fontSize = 12.sp,
            lineHeight = 17.sp
        )
    }
}

@Composable
private fun MobileDashboard(
    wallet: WalletProfile?,
    sync: SyncState?,
    network: NetworkSnapshot?,
    blocks: List<BlockSummary>,
    pool: PoolSnapshot?,
    endpoint: String,
    onEndpointChange: (String) -> Unit,
    onRefresh: () -> Unit,
    onLock: () -> Unit
) {
    var tab by remember { mutableStateOf(MobileTab.Overview) }
    val clipboard = LocalClipboard.current
    val clipboardScope = rememberCoroutineScope()

    Column(
        Modifier
            .fillMaxSize()
            .background(Brush.verticalGradient(listOf(Deep, Color(0xFF04151D))))
            .statusBarsPadding()
    ) {
        Column(
            Modifier
                .weight(1f)
                .verticalScroll(rememberScrollState())
                .padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Image(
                    painter = painterResource(id = R.drawable.logo),
                    contentDescription = null,
                    modifier = Modifier.size(40.dp).clip(RoundedCornerShape(12.dp)),
                    contentScale = ContentScale.Fit
                )
                Column(Modifier.weight(1f)) {
                    Text(
                        "CONTROL PANEL · v${BuildConfig.VERSION_NAME} · API 31+",
                        color = Cyan,
                        letterSpacing = 1.2.sp,
                        fontSize = 11.sp,
                        fontFamily = FontFamily.Monospace
                    )
                    Text(wallet?.name ?: "No wallet", style = MaterialTheme.typography.titleLarge)
                }
                TextButton(onClick = onRefresh) { Text("Refresh") }
                TextButton(onClick = onLock) { Text("Lock") }
            }

            MobileMiningBanBanner()

            when (tab) {
                MobileTab.Overview -> {
                    MetricCard("Gateway", sync?.mode?.uppercase() ?: "OFFLINE", endpoint)
                    MetricCard("Height", "${sync?.localHeight ?: network?.height ?: "—"}", "gateway tip")
                    MetricCard(
                        "Balance",
                        network?.balanceAtomic?.let { formatAtomic(it) } ?: "—",
                        wallet?.address?.let { shortAddr(it) } ?: "select wallet"
                    )
                    MetricCard(
                        "Integrity",
                        when {
                            network?.integrityOk == true -> "OK"
                            network?.integrityOk == false -> "CHECK"
                            else -> "—"
                        },
                        network?.integrityDetail ?: "chain integrity probe"
                    )
                    MetricCard(
                        "Indexer",
                        if (network?.indexInSync == true) "IN SYNC" else (network?.indexTip?.take(16) ?: "—"),
                        "lag ${network?.indexLag ?: "—"}"
                    )
                    MetricCard("Mempool", "${network?.mempoolCount ?: "—"}", "pending txs (if RPC exposes count)")
                    MetricCard("Peers", "${network?.peerCount ?: sync?.validatedPeers ?: "—"}", sync?.detail ?: "")
                    Text(
                        "Mobile mirrors desktop for read-only chain visibility. Signing/broadcast & mining remain on Windows/Linux Control Center.",
                        color = Muted,
                        fontSize = 12.sp
                    )
                }
                MobileTab.Wallet -> {
                    MetricCard("Active wallet", wallet?.name ?: "—", "Android Keystore AES-GCM")
                    MetricCard("Address", wallet?.address ?: "—", "same BIP39/SLIP-0010 path as desktop")
                    MetricCard(
                        "Balance",
                        network?.balanceAtomic?.let { formatAtomic(it) } ?: "—",
                        "from public RPC /addresses/.../balance"
                    )
                    if (wallet != null) {
                        Button(
                            onClick = {
                                clipboardScope.launch {
                                    clipboard.setClipEntry(
                                        ClipData.newPlainText("Alvenqis address", wallet.address).toClipEntry()
                                    )
                                }
                            },
                            colors = ButtonDefaults.buttonColors(containerColor = PanelRaised, contentColor = Cyan),
                            modifier = Modifier.fillMaxWidth()
                        ) { Text("Copy address") }
                    }
                    Text(
                        "Create/import 12 or 24 English words. Transaction signing & broadcast remain on desktop Tauri Control Center for this candidate.",
                        color = Muted,
                        fontSize = 12.sp
                    )
                }
                MobileTab.Network -> {
                    OutlinedTextField(
                        value = endpoint,
                        onValueChange = onEndpointChange,
                        label = { Text("RPC endpoint") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(onClick = { onEndpointChange(BuildConfig.PUBLIC_RPC) }) {
                            Text("Official VPS")
                        }
                        OutlinedButton(onClick = { onEndpointChange(RpcEndpointPolicy.EMULATOR_RPC) }) {
                            Text("Emulator")
                        }
                    }
                    MetricCard("Network ID", network?.networkId ?: "alvenqis-mainnet-candidate", "must match Mainnet Candidate")
                    MetricCard("Status label", network?.statusLabel ?: "—", "honest prototype label")
                    MetricCard("Tip hash", network?.tipHash ?: "—", "from /status")
                    MetricCard("Sync", sync?.mode ?: "—", sync?.detail ?: "")
                    MetricCard("Validated peers", "${sync?.validatedPeers ?: 0}", "from /sync/status")
                }
                MobileTab.Explorer -> {
                    MetricCard(
                        "Latest height",
                        "${sync?.localHeight ?: network?.height ?: "—"}",
                        "same tip as desktop explorer"
                    )
                    MetricCard("Tip", network?.tipHash ?: "—", "full tables on desktop / web explorer")
                    if (blocks.isEmpty()) {
                        Text("No recent blocks from RPC (endpoint may not list /blocks).", color = Muted, fontSize = 12.sp)
                    } else {
                        Text("Recent blocks", color = Cyan, fontWeight = FontWeight.SemiBold)
                        blocks.forEach { b ->
                            MetricCard(
                                "H ${b.height}",
                                shortHash(b.hash),
                                listOfNotNull(
                                    b.txCount?.let { "$it tx" },
                                    b.timestamp?.let { "ts $it" }
                                ).joinToString(" · ").ifBlank { "block" }
                            )
                        }
                    }
                }
                MobileTab.Pool -> {
                    if (pool == null) {
                        MetricCard("Pool", "UNAVAILABLE", "Could not load /pool/api/v1/pool/status")
                        Text(
                            "Official gateway exposes the public pool under HTTPS /pool/. Local stacks need the pool reverse-proxy path.",
                            color = Muted,
                            fontSize = 12.sp
                        )
                    } else {
                        MetricCard("Pool", pool.poolName ?: "—", pool.statusLabel ?: "PPLNS prototype")
                        MetricCard("Upstream", pool.upstreamStatus ?: "—", pool.payoutScheme ?: "scheme")
                        MetricCard("Shares", "${pool.acceptedShares ?: "—"}", "accepted (all miners)")
                        MetricCard("Blocks found", "${pool.blocksFound ?: "—"}", "pool accounting")
                        MetricCard("Workers online", "${pool.workers ?: "—"}", "connected")
                        MetricCard(
                            "Hashrate",
                            pool.hashrateHs?.let { formatHs(it) } ?: "—",
                            "pool estimate"
                        )
                        MetricCard(
                            "Fee",
                            pool.feeBps?.let { "${it / 100.0}% ($it bps)" } ?: "—",
                            "pool fee"
                        )
                    }
                    Text(
                        "Rewards are split by pool PPLNS on the server (share work). Mining workers run only on Windows/Linux.",
                        color = Muted,
                        fontSize = 12.sp
                    )
                }
                MobileTab.Mining -> {
                    MobileMiningBanBanner()
                    MetricCard("Mining rights", "DISABLED", "no start/stop · no shares · no local PoW")
                    MetricCard("Desktop mining", "Windows / Linux only", "Tauri Control Center · CUDA / OpenCL FiroPoW")
                    MetricCard("Why?", "Thermal · battery · OS limits · product policy", "Mobile is wallet + monitor")
                    Button(
                        onClick = { },
                        enabled = false,
                        modifier = Modifier.fillMaxWidth(),
                        colors = ButtonDefaults.buttonColors(
                            disabledContainerColor = Border,
                            disabledContentColor = Muted
                        )
                    ) {
                        Text("Start miner (unavailable on mobile)")
                    }
                }
            }
        }

        NavigationBar(
            containerColor = PanelRaised,
            contentColor = TextPrimary,
            modifier = Modifier.navigationBarsPadding()
        ) {
            MobileTab.entries.forEach { item ->
                NavigationBarItem(
                    selected = tab == item,
                    onClick = { tab = item },
                    icon = {
                        Text(
                            item.label.take(1),
                            color = if (tab == item) Cyan else Muted,
                            fontWeight = FontWeight.Bold
                        )
                    },
                    label = {
                        Text(
                            item.label,
                            fontSize = 10.sp,
                            color = if (tab == item) Cyan else Muted
                        )
                    },
                    colors = NavigationBarItemDefaults.colors(
                        indicatorColor = Panel,
                        selectedIconColor = Cyan,
                        selectedTextColor = Cyan,
                        unselectedIconColor = Muted,
                        unselectedTextColor = Muted
                    )
                )
            }
        }
    }
}

@Composable
private fun MetricCard(label: String, value: String, detail: String) {
    Column(
        Modifier
            .fillMaxWidth()
            .border(1.dp, Border, RoundedCornerShape(16.dp))
            .background(Panel, RoundedCornerShape(16.dp))
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Text(label.uppercase(), color = Muted, fontSize = 10.sp, letterSpacing = 1.5.sp)
        Text(value, color = Cyan, fontFamily = FontFamily.Monospace, fontSize = 16.sp, fontWeight = FontWeight.Medium)
        Text(detail, color = Muted, fontSize = 11.sp)
    }
}

private fun formatAtomic(atomic: Long): String {
    val whole = atomic / 100_000_000L
    val frac = (atomic % 100_000_000L).toString().padStart(8, '0')
    return "$whole.$frac ALVE"
}

private fun shortAddr(a: String): String =
    if (a.length <= 20) a else "${a.take(12)}…${a.takeLast(8)}"

private fun shortHash(h: String): String =
    if (h.length <= 18) h else "${h.take(10)}…${h.takeLast(8)}"

private fun formatHs(hs: Double): String = when {
    hs >= 1e9 -> String.format("%.2f GH/s", hs / 1e9)
    hs >= 1e6 -> String.format("%.2f MH/s", hs / 1e6)
    hs >= 1e3 -> String.format("%.2f kH/s", hs / 1e3)
    else -> String.format("%.0f H/s", hs)
}
