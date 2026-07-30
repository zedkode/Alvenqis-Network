// Alvenqis FFI for FiroPoW 0.9.4 (ProgPoW period_length=1).
// Apache-2.0 ethash/progpow vendored from firoorg/firo.
#include <crypto/progpow/include/ethash/ethash.hpp>
#include <crypto/progpow/include/ethash/progpow.hpp>
#include <crypto/progpow/include/ethash/keccak.hpp>
#include <crypto/progpow/lib/ethash/ethash-internal.hpp>
#include <atomic>
#include <cstring>
#include <memory>
#include <mutex>
#include <thread>
#include <unordered_map>
#include <vector>

extern "C" {

struct alvenqis_firopow_result {
    uint8_t final_hash[32];
    uint8_t mix_hash[32];
};

static std::mutex g_ctx_mu;
static std::mutex g_full_materialize_mu;
static std::unordered_map<int, ethash::epoch_context_ptr> g_light_ctx;
static std::unordered_map<int, ethash::epoch_context_full_ptr> g_full_ctx;

static const ethash::epoch_context* get_light_ctx(int epoch) {
    std::lock_guard<std::mutex> lock(g_ctx_mu);
    auto it = g_light_ctx.find(epoch);
    if (it == g_light_ctx.end()) {
        auto ctx = ethash::create_epoch_context(epoch);
        it = g_light_ctx.emplace(epoch, std::move(ctx)).first;
    }
    return it->second.get();
}

static ethash::epoch_context_full* get_full_ctx(int epoch) {
    std::lock_guard<std::mutex> lock(g_ctx_mu);
    auto it = g_full_ctx.find(epoch);
    if (it == g_full_ctx.end()) {
        auto ctx = ethash::create_epoch_context_full(epoch);
        if (!ctx)
            return nullptr;
        it = g_full_ctx.emplace(epoch, std::move(ctx)).first;
    }
    return it->second.get();
}

static ethash::hash256 load_h256(const uint8_t* b) {
    ethash::hash256 h{};
    std::memcpy(h.bytes, b, 32);
    return h;
}

int alvenqis_firopow_revision(char* out, int out_len) {
    const char* rev = progpow::revision;
    if (!out || out_len <= 0)
        return (int)std::strlen(rev);
    int n = (int)std::strlen(rev);
    if (n >= out_len)
        n = out_len - 1;
    std::memcpy(out, rev, (size_t)n);
    out[n] = 0;
    return n;
}

int alvenqis_firopow_period_length(void) {
    return progpow::period_length;
}

int alvenqis_firopow_epoch_length(void) {
    return ETHASH_EPOCH_LENGTH;
}

int alvenqis_firopow_epoch_number(int block_number) {
    return ethash::get_epoch_number(block_number);
}

void alvenqis_keccak256(const uint8_t* data, size_t len, uint8_t out[32]) {
    auto h = ethash::keccak256(data, len);
    std::memcpy(out, h.bytes, 32);
}

int alvenqis_firopow_hash(
    int block_number,
    const uint8_t header_hash[32],
    uint64_t nonce,
    alvenqis_firopow_result* out
) {
    if (!header_hash || !out)
        return -1;
    try {
        int epoch = ethash::get_epoch_number(block_number);
        // Consensus hashing must use the immutable light context. The vendored
        // full-context lookup lazily writes DAG items and is not safe when node,
        // RPC and P2P threads evaluate hashes concurrently.
        const auto* ctx = get_light_ctx(epoch);
        auto hh = load_h256(header_hash);
        auto r = progpow::hash(*ctx, block_number, hh, nonce);
        std::memcpy(out->final_hash, r.final_hash.bytes, 32);
        std::memcpy(out->mix_hash, r.mix_hash.bytes, 32);
        return 0;
    } catch (...) {
        return -2;
    }
}

int alvenqis_firopow_verify(
    int block_number,
    const uint8_t header_hash[32],
    const uint8_t mix_hash[32],
    uint64_t nonce,
    const uint8_t boundary[32]
) {
    if (!header_hash || !mix_hash || !boundary)
        return 0;
    try {
        int epoch = ethash::get_epoch_number(block_number);
        const auto* ctx = get_light_ctx(epoch);
        auto hh = load_h256(header_hash);
        auto mh = load_h256(mix_hash);
        auto bd = load_h256(boundary);
        return progpow::verify(*ctx, block_number, hh, mh, nonce, bd) ? 1 : 0;
    } catch (...) {
        return 0;
    }
}

int alvenqis_firopow_search_light(
    int block_number,
    const uint8_t header_hash[32],
    const uint8_t boundary[32],
    uint64_t start_nonce,
    size_t iterations,
    uint64_t* found_nonce,
    alvenqis_firopow_result* out
) {
    if (!header_hash || !boundary || !found_nonce || !out)
        return -1;
    try {
        int epoch = ethash::get_epoch_number(block_number);
        const auto* ctx = get_light_ctx(epoch);
        auto hh = load_h256(header_hash);
        auto bd = load_h256(boundary);
        auto sr = progpow::search_light(*ctx, block_number, hh, bd, start_nonce, iterations);
        if (!sr.solution_found)
            return 0;
        *found_nonce = sr.nonce;
        std::memcpy(out->final_hash, sr.final_hash.bytes, 32);
        std::memcpy(out->mix_hash, sr.mix_hash.bytes, 32);
        return 1;
    } catch (...) {
        return -2;
    }
}

// Multi-threaded immutable light-context search for tests/genesis one-shots.
// threads<=0 => hardware concurrency. cancel_flag optional: non-null and *cancel!=0 aborts.
int alvenqis_firopow_search_mt(
    int block_number,
    const uint8_t header_hash[32],
    const uint8_t boundary[32],
    uint64_t start_nonce,
    size_t iterations,
    int threads,
    const int* cancel_flag,
    uint64_t* found_nonce,
    alvenqis_firopow_result* out,
    uint64_t* hashes_done
) {
    if (!header_hash || !boundary || !found_nonce || !out)
        return -1;
    if (iterations == 0)
        return 0;
    try {
        int epoch = ethash::get_epoch_number(block_number);
        const auto* light = get_light_ctx(epoch);
        if (!light)
            return -2;
        auto hh = load_h256(header_hash);
        auto bd = load_h256(boundary);

        unsigned hc = std::thread::hardware_concurrency();
        if (hc == 0)
            hc = 4;
        int t = threads > 0 ? threads : (int)hc;
        if (t < 1)
            t = 1;
        if ((size_t)t > iterations)
            t = (int)iterations;

        std::atomic<uint64_t> best_index{static_cast<uint64_t>(iterations)};
        std::atomic<uint64_t> done{0};

        auto worker = [&](int tid) {
            size_t chunk = (iterations + (size_t)t - 1) / (size_t)t;
            size_t begin = (size_t)tid * chunk;
            if (begin >= iterations)
                return;
            size_t end = begin + chunk;
            if (end > iterations)
                end = iterations;
            uint64_t local_done = 0;
            for (size_t i = begin; i < end; ++i) {
                if (static_cast<uint64_t>(i) >= best_index.load(std::memory_order_relaxed))
                    break;
                if (cancel_flag && *cancel_flag)
                    break;
                uint64_t nonce = start_nonce + (uint64_t)i;
                const auto r = progpow::hash(*light, block_number, hh, nonce);
                ++local_done;
                // boundary compare big-endian: final <= boundary
                bool ok = true;
                for (int b = 0; b < 32; ++b) {
                    if (r.final_hash.bytes[b] < bd.bytes[b]) {
                        ok = true;
                        break;
                    }
                    if (r.final_hash.bytes[b] > bd.bytes[b]) {
                        ok = false;
                        break;
                    }
                }
                if (ok) {
                    uint64_t candidate = static_cast<uint64_t>(i);
                    uint64_t current = best_index.load(std::memory_order_relaxed);
                    while (candidate < current &&
                           !best_index.compare_exchange_weak(
                               current,
                               candidate,
                               std::memory_order_relaxed,
                               std::memory_order_relaxed)) {}
                    break;
                }
            }
            done.fetch_add(local_done, std::memory_order_relaxed);
        };

        std::vector<std::thread> pool;
        pool.reserve((size_t)t);
        for (int i = 0; i < t; ++i)
            pool.emplace_back(worker, i);
        for (auto& th : pool)
            th.join();

        if (hashes_done)
            *hashes_done = done.load();

        const uint64_t winner = best_index.load(std::memory_order_relaxed);
        if (winner < static_cast<uint64_t>(iterations)) {
            const uint64_t nonce = start_nonce + winner;
            const auto result = progpow::hash(*light, block_number, hh, nonce);
            *found_nonce = nonce;
            std::memcpy(out->final_hash, result.final_hash.bytes, 32);
            std::memcpy(out->mix_hash, result.mix_hash.bytes, 32);
            return 1;
        }
        return 0;
    } catch (...) {
        return -2;
    }
}

// Ensure full DAG context exists for epoch (pre-warm for mining).
int alvenqis_firopow_prewarm_full(int block_number) {
    try {
        int epoch = ethash::get_epoch_number(block_number);
        return get_full_ctx(epoch) ? 0 : -1;
    } catch (...) {
        return -2;
    }
}

// Dataset size helpers for GPU planners.
uint64_t alvenqis_firopow_full_dataset_bytes(int block_number) {
    int epoch = ethash::get_epoch_number(block_number);
    int items = ethash::calculate_full_dataset_num_items(epoch);
    return ethash::get_full_dataset_size(items);
}

int alvenqis_firopow_light_cache_items(int block_number) {
    int epoch = ethash::get_epoch_number(block_number);
    return ethash::calculate_light_cache_num_items(epoch);
}

/// Export the small, immutable epoch light cache and ProgPoW L1 cache.
/// CUDA miners use this view to build the full DAG directly in VRAM instead of
/// materialising ~1 GiB on the host and copying it over PCIe.
int alvenqis_firopow_export_light_cache(
    int block_number,
    const uint32_t** light_out,
    uint32_t* light_items_out,
    const uint32_t** l1_out,
    uint32_t* l1_words_out,
    int* full_dataset_num_items_out
) {
    if (!light_out || !light_items_out || !l1_out || !l1_words_out ||
        !full_dataset_num_items_out)
        return -1;
    try {
        const int epoch = ethash::get_epoch_number(block_number);
        const auto* light = get_light_ctx(epoch);
        if (!light || !light->light_cache || !light->l1_cache)
            return -2;

        *light_out = reinterpret_cast<const uint32_t*>(light->light_cache);
        *light_items_out = static_cast<uint32_t>(light->light_cache_num_items);
        *l1_out = light->l1_cache;
        *l1_words_out = static_cast<uint32_t>(progpow::l1_cache_num_items);
        *full_dataset_num_items_out = light->full_dataset_num_items;
        return 0;
    } catch (...) {
        return -3;
    }
}

int alvenqis_firopow_dataset_item_1024(
    int block_number,
    uint32_t index,
    uint8_t out[128]
) {
    if (!out)
        return -1;
    try {
        const int epoch = ethash::get_epoch_number(block_number);
        const auto* light = get_light_ctx(epoch);
        if (!light || index >= static_cast<uint32_t>(light->full_dataset_num_items))
            return -2;
        const auto item = ethash::calculate_dataset_item_1024(*light, index);
        std::memcpy(out, item.bytes, sizeof(item.bytes));
        return 0;
    } catch (...) {
        return -3;
    }
}

/// Fully materialize the epoch DAG and export host pointers for CUDA upload.
/// Pointers remain valid until process exit (cached epoch contexts).
///
/// - dag_out: hash1024 items (full_dataset_num_items entries, 128 bytes each)
/// - l1_out: ProgPoW L1 cache words (l1_cache_num_items uint32_t)
/// - full_dataset_num_items_out: number of hash1024 items
int alvenqis_firopow_export_full_dag(
    int block_number,
    const uint8_t** dag_out,
    uint64_t* dag_bytes_out,
    const uint32_t** l1_out,
    uint32_t* l1_words_out,
    int* full_dataset_num_items_out
) {
    if (!dag_out || !dag_bytes_out || !l1_out || !l1_words_out || !full_dataset_num_items_out)
        return -1;
    try {
        // Full DAG materialization mutates lazy dataset slots. Serialize the
        // one-time operation; all returned pointers are immutable afterwards.
        std::lock_guard<std::mutex> materialize_lock(g_full_materialize_mu);
        int epoch = ethash::get_epoch_number(block_number);
        auto* full = get_full_ctx(epoch);
        if (!full || !full->full_dataset || !full->l1_cache)
            return -2;

        // Materialize every hash2048 DAG item (pairs of hash1024) so GPU reads are dense.
        const int num_1024 = full->full_dataset_num_items;
        const int num_2048 = num_1024 / 2;
        auto* dataset_1024 = full->full_dataset;
        auto* dataset_2048 = reinterpret_cast<ethash::hash2048*>(dataset_1024);
        for (int i = 0; i < num_2048; ++i) {
            if (dataset_2048[i].word64s[0] == 0) {
                dataset_2048[i] = ethash::calculate_dataset_item_2048(*full, static_cast<uint32_t>(i));
            }
        }

        *dag_out = reinterpret_cast<const uint8_t*>(dataset_1024);
        *dag_bytes_out = ethash::get_full_dataset_size(num_1024);
        *l1_out = full->l1_cache;
        *l1_words_out = static_cast<uint32_t>(progpow::l1_cache_num_items);
        *full_dataset_num_items_out = num_1024;
        return 0;
    } catch (...) {
        return -3;
    }
}

} // extern "C"
