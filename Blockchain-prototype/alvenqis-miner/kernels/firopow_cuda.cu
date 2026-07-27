// Alvenqis FiroPoW 0.9.4 CUDA miner (ProgPoW period_length = 1).
// Must match alvenqis-core / vendored firoorg progpow byte-for-byte.
// Host re-validates every solution before submit.
//
// Build: nvcc via alvenqis-miner/build.rs when gpu-cuda feature is on.

#include <cuda_runtime.h>
#include <stdint.h>
#include <string.h>

// ---------------------------------------------------------------------------
// ProgPoW 0.9.4 / FiroPoW constants
// ---------------------------------------------------------------------------
static const int PP_PERIOD_LENGTH = 1;
static const int PP_NUM_REGS = 32;
static const int PP_NUM_LANES = 16;
static const int PP_NUM_ROUNDS = 64;
static const int PP_NUM_CACHE_ACCESSES = 11;
static const int PP_NUM_MATH_OPS = 18;
static const int PP_L1_CACHE_WORDS = 16 * 1024 / 4; // 4096
static const uint32_t FNV_PRIME = 0x01000193u;
static const uint32_t FNV_OFFSET = 0x811c9dc5u;
// FiroPoW / ProgPoW 0.9.4 doubles the Ethash parent rounds from 256 to 512.
static const int ETHASH_DATASET_PARENTS = 512;

// ---------------------------------------------------------------------------
// Device helpers
// ---------------------------------------------------------------------------
__device__ __forceinline__ uint32_t d_rotl32(uint32_t n, unsigned c) {
    c &= 31u;
    return (n << c) | (n >> ((32u - c) & 31u));
}
__device__ __forceinline__ uint32_t d_rotr32(uint32_t n, unsigned c) {
    c &= 31u;
    return (n >> c) | (n << ((32u - c) & 31u));
}
__device__ __forceinline__ uint32_t d_clz32(uint32_t x) {
    return x ? (uint32_t)__clz(x) : 32u;
}
__device__ __forceinline__ uint32_t d_popc32(uint32_t x) {
    return (uint32_t)__popc(x);
}
__device__ __forceinline__ uint32_t d_mul_hi32(uint32_t a, uint32_t b) {
    return __umulhi(a, b);
}
__device__ __forceinline__ uint32_t d_fnv1a(uint32_t u, uint32_t v) {
    return (u ^ v) * FNV_PRIME;
}
__device__ __forceinline__ uint32_t d_fnv1(uint32_t u, uint32_t v) {
    return (u * FNV_PRIME) ^ v;
}

// Keccak-f[800] (32-bit state) — same as ethash keccakf800.c
__device__ __forceinline__ uint32_t d_rol32(uint32_t x, unsigned s) {
    return (x << s) | (x >> (32 - s));
}

__device__ void d_keccakf800(uint32_t st[25]) {
    const uint32_t RC[22] = {
        0x00000001u, 0x00008082u, 0x0000808Au, 0x80008000u, 0x0000808Bu, 0x80000001u,
        0x80008081u, 0x00008009u, 0x0000008Au, 0x00000088u, 0x80008009u, 0x8000000Au,
        0x8000808Bu, 0x0000008Bu, 0x00008089u, 0x00008003u, 0x00008002u, 0x00000080u,
        0x0000800Au, 0x8000000Au, 0x80008081u, 0x00008080u,
    };

    uint32_t Aba=st[0],Abe=st[1],Abi=st[2],Abo=st[3],Abu=st[4];
    uint32_t Aga=st[5],Age=st[6],Agi=st[7],Ago=st[8],Agu=st[9];
    uint32_t Aka=st[10],Ake=st[11],Aki=st[12],Ako=st[13],Aku=st[14];
    uint32_t Ama=st[15],Ame=st[16],Ami=st[17],Amo=st[18],Amu=st[19];
    uint32_t Asa=st[20],Ase=st[21],Asi=st[22],Aso=st[23],Asu=st[24];

    for (int round = 0; round < 22; round += 2) {
        uint32_t Ba,Be,Bi,Bo,Bu,Da,De,Di,Do,Du;
        uint32_t Eba,Ebe,Ebi,Ebo,Ebu,Ega,Ege,Egi,Ego,Egu;
        uint32_t Eka,Eke,Eki,Eko,Eku,Ema,Eme,Emi,Emo,Emu;
        uint32_t Esa,Ese,Esi,Eso,Esu;

        Ba = Aba^Aga^Aka^Ama^Asa; Be=Abe^Age^Ake^Ame^Ase; Bi=Abi^Agi^Aki^Ami^Asi;
        Bo = Abo^Ago^Ako^Amo^Aso; Bu=Abu^Agu^Aku^Amu^Asu;
        Da=Bu^d_rol32(Be,1); De=Ba^d_rol32(Bi,1); Di=Be^d_rol32(Bo,1); Do=Bi^d_rol32(Bu,1); Du=Bo^d_rol32(Ba,1);

        Ba=Aba^Da; Be=d_rol32(Age^De,12); Bi=d_rol32(Aki^Di,11); Bo=d_rol32(Amo^Do,21); Bu=d_rol32(Asu^Du,14);
        Eba=Ba^(~Be&Bi)^RC[round]; Ebe=Be^(~Bi&Bo); Ebi=Bi^(~Bo&Bu); Ebo=Bo^(~Bu&Ba); Ebu=Bu^(~Ba&Be);
        Ba=d_rol32(Abo^Do,28); Be=d_rol32(Agu^Du,20); Bi=d_rol32(Aka^Da,3); Bo=d_rol32(Ame^De,13); Bu=d_rol32(Asi^Di,29);
        Ega=Ba^(~Be&Bi); Ege=Be^(~Bi&Bo); Egi=Bi^(~Bo&Bu); Ego=Bo^(~Bu&Ba); Egu=Bu^(~Ba&Be);
        Ba=d_rol32(Abe^De,1); Be=d_rol32(Agi^Di,6); Bi=d_rol32(Ako^Do,25); Bo=d_rol32(Amu^Du,8); Bu=d_rol32(Asa^Da,18);
        Eka=Ba^(~Be&Bi); Eke=Be^(~Bi&Bo); Eki=Bi^(~Bo&Bu); Eko=Bo^(~Bu&Ba); Eku=Bu^(~Ba&Be);
        Ba=d_rol32(Abu^Du,27); Be=d_rol32(Aga^Da,4); Bi=d_rol32(Ake^De,10); Bo=d_rol32(Ami^Di,15); Bu=d_rol32(Aso^Do,24);
        Ema=Ba^(~Be&Bi); Eme=Be^(~Bi&Bo); Emi=Bi^(~Bo&Bu); Emo=Bo^(~Bu&Ba); Emu=Bu^(~Ba&Be);
        Ba=d_rol32(Abi^Di,30); Be=d_rol32(Ago^Do,23); Bi=d_rol32(Aku^Du,7); Bo=d_rol32(Ama^Da,9); Bu=d_rol32(Ase^De,2);
        Esa=Ba^(~Be&Bi); Ese=Be^(~Bi&Bo); Esi=Bi^(~Bo&Bu); Eso=Bo^(~Bu&Ba); Esu=Bu^(~Ba&Be);

        Ba=Eba^Ega^Eka^Ema^Esa; Be=Ebe^Ege^Eke^Eme^Ese; Bi=Ebi^Egi^Eki^Emi^Esi;
        Bo=Ebo^Ego^Eko^Emo^Eso; Bu=Ebu^Egu^Eku^Emu^Esu;
        Da=Bu^d_rol32(Be,1); De=Ba^d_rol32(Bi,1); Di=Be^d_rol32(Bo,1); Do=Bi^d_rol32(Bu,1); Du=Bo^d_rol32(Ba,1);

        Ba=Eba^Da; Be=d_rol32(Ege^De,12); Bi=d_rol32(Eki^Di,11); Bo=d_rol32(Emo^Do,21); Bu=d_rol32(Esu^Du,14);
        Aba=Ba^(~Be&Bi)^RC[round+1]; Abe=Be^(~Bi&Bo); Abi=Bi^(~Bo&Bu); Abo=Bo^(~Bu&Ba); Abu=Bu^(~Ba&Be);
        Ba=d_rol32(Ebo^Do,28); Be=d_rol32(Egu^Du,20); Bi=d_rol32(Eka^Da,3); Bo=d_rol32(Eme^De,13); Bu=d_rol32(Esi^Di,29);
        Aga=Ba^(~Be&Bi); Age=Be^(~Bi&Bo); Agi=Bi^(~Bo&Bu); Ago=Bo^(~Bu&Ba); Agu=Bu^(~Ba&Be);
        Ba=d_rol32(Ebe^De,1); Be=d_rol32(Egi^Di,6); Bi=d_rol32(Eko^Do,25); Bo=d_rol32(Emu^Du,8); Bu=d_rol32(Esa^Da,18);
        Aka=Ba^(~Be&Bi); Ake=Be^(~Bi&Bo); Aki=Bi^(~Bo&Bu); Ako=Bo^(~Bu&Ba); Aku=Bu^(~Ba&Be);
        Ba=d_rol32(Ebu^Du,27); Be=d_rol32(Ega^Da,4); Bi=d_rol32(Eke^De,10); Bo=d_rol32(Emi^Di,15); Bu=d_rol32(Eso^Do,24);
        Ama=Ba^(~Be&Bi); Ame=Be^(~Bi&Bo); Ami=Bi^(~Bo&Bu); Amo=Bo^(~Bu&Ba); Amu=Bu^(~Ba&Be);
        Ba=d_rol32(Ebi^Di,30); Be=d_rol32(Ego^Do,23); Bi=d_rol32(Eku^Du,7); Bo=d_rol32(Ema^Da,9); Bu=d_rol32(Ese^De,2);
        Asa=Ba^(~Be&Bi); Ase=Be^(~Bi&Bo); Asi=Bi^(~Bo&Bu); Aso=Bo^(~Bu&Ba); Asu=Bu^(~Ba&Be);
    }

    st[0]=Aba;st[1]=Abe;st[2]=Abi;st[3]=Abo;st[4]=Abu;
    st[5]=Aga;st[6]=Age;st[7]=Agi;st[8]=Ago;st[9]=Agu;
    st[10]=Aka;st[11]=Ake;st[12]=Aki;st[13]=Ako;st[14]=Aku;
    st[15]=Ama;st[16]=Ame;st[17]=Ami;st[18]=Amo;st[19]=Amu;
    st[20]=Asa;st[21]=Ase;st[22]=Asi;st[23]=Aso;st[24]=Asu;
}

// Keccak-f[1600] used by Ethash light-cache -> DAG item generation.
__device__ __forceinline__ uint64_t d_rotl64(uint64_t x, unsigned s) {
    return s == 0 ? x : (x << s) | (x >> (64u - s));
}

__device__ void d_keccakf1600(uint64_t st[25]) {
    const uint64_t round_constants[24] = {
        0x0000000000000001ULL, 0x0000000000008082ULL,
        0x800000000000808aULL, 0x8000000080008000ULL,
        0x000000000000808bULL, 0x0000000080000001ULL,
        0x8000000080008081ULL, 0x8000000000008009ULL,
        0x000000000000008aULL, 0x0000000000000088ULL,
        0x0000000080008009ULL, 0x000000008000000aULL,
        0x000000008000808bULL, 0x800000000000008bULL,
        0x8000000000008089ULL, 0x8000000000008003ULL,
        0x8000000000008002ULL, 0x8000000000000080ULL,
        0x000000000000800aULL, 0x800000008000000aULL,
        0x8000000080008081ULL, 0x8000000000008080ULL,
        0x0000000080000001ULL, 0x8000000080008008ULL,
    };
    const int rotation[24] = {
        1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14,
        27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
    };
    const int permutation[24] = {
        10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4,
        15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
    };

    for (int round = 0; round < 24; ++round) {
        uint64_t bc[5];
        #pragma unroll
        for (int i = 0; i < 5; ++i)
            bc[i] = st[i] ^ st[i + 5] ^ st[i + 10] ^ st[i + 15] ^ st[i + 20];
        #pragma unroll
        for (int i = 0; i < 5; ++i) {
            const uint64_t t = bc[(i + 4) % 5] ^ d_rotl64(bc[(i + 1) % 5], 1);
            for (int j = 0; j < 25; j += 5) st[j + i] ^= t;
        }

        uint64_t t = st[1];
        #pragma unroll
        for (int i = 0; i < 24; ++i) {
            const int j = permutation[i];
            const uint64_t next = st[j];
            st[j] = d_rotl64(t, (unsigned)rotation[i]);
            t = next;
        }

        for (int j = 0; j < 25; j += 5) {
            #pragma unroll
            for (int i = 0; i < 5; ++i) bc[i] = st[j + i];
            #pragma unroll
            for (int i = 0; i < 5; ++i)
                st[j + i] = bc[i] ^ ((~bc[(i + 1) % 5]) & bc[(i + 2) % 5]);
        }
        st[0] ^= round_constants[round];
    }
}

/// Legacy Keccak-512 over exactly one 64-byte Ethash item (delimiter 0x01).
__device__ void d_keccak512_64(uint32_t words[16]) {
    uint64_t state[25];
    #pragma unroll
    for (int i = 0; i < 25; ++i) state[i] = 0;
    #pragma unroll
    for (int i = 0; i < 8; ++i)
        state[i] = (uint64_t)words[i * 2] | ((uint64_t)words[i * 2 + 1] << 32);
    // Rate is 72 bytes. Input ends at byte 64, so both padding bits are in lane 8.
    state[8] = 0x8000000000000001ULL;
    d_keccakf1600(state);
    #pragma unroll
    for (int i = 0; i < 8; ++i) {
        words[i * 2] = (uint32_t)state[i];
        words[i * 2 + 1] = (uint32_t)(state[i] >> 32);
    }
}

/// Build hash512 DAG items directly in VRAM from the ~16 MiB light cache.
__global__ void alvenqis_build_dag_kernel(
    const uint32_t* __restrict__ light,
    uint32_t light_items,
    uint32_t* __restrict__ dag,
    uint32_t dataset_items_512
) {
    const uint32_t stride = blockDim.x * gridDim.x;
    for (uint32_t index = blockIdx.x * blockDim.x + threadIdx.x;
         index < dataset_items_512;
         index += stride) {
        uint32_t mix[16];
        const uint32_t* initial = light + (size_t)(index % light_items) * 16u;
        #pragma unroll
        for (int word = 0; word < 16; ++word) mix[word] = initial[word];
        mix[0] ^= index;
        d_keccak512_64(mix);

        #pragma unroll 1
        for (uint32_t parent = 0; parent < (uint32_t)ETHASH_DATASET_PARENTS; ++parent) {
            const uint32_t parent_index =
                d_fnv1(index ^ parent, mix[parent & 15u]) % light_items;
            const uint32_t* parent_words = light + (size_t)parent_index * 16u;
            #pragma unroll
            for (int word = 0; word < 16; ++word)
                mix[word] = d_fnv1(mix[word], parent_words[word]);
        }
        d_keccak512_64(mix);

        uint32_t* output = dag + (size_t)index * 16u;
        #pragma unroll
        for (int word = 0; word < 16; ++word) output[word] = mix[word];
    }
}

// KISS99
struct Kiss99 {
    uint32_t z, w, jsr, jcong;
    __device__ uint32_t next() {
        z = 36969u * (z & 0xffffu) + (z >> 16);
        w = 18000u * (w & 0xffffu) + (w >> 16);
        jcong = 69069u * jcong + 1234567u;
        jsr ^= (jsr << 17);
        jsr ^= (jsr >> 13);
        jsr ^= (jsr << 5);
        return (((z << 16) + w) ^ jcong) + jsr;
    }
};

__device__ uint32_t d_random_math(uint32_t a, uint32_t b, uint32_t selector) {
    switch (selector % 11u) {
    case 0: return a + b;
    case 1: return a * b;
    case 2: return d_mul_hi32(a, b);
    case 3: return a < b ? a : b;
    case 4: return d_rotl32(a, b);
    case 5: return d_rotr32(a, b);
    case 6: return a & b;
    case 7: return a | b;
    case 8: return a ^ b;
    case 9: return d_clz32(a) + d_clz32(b);
    default: return d_popc32(a) + d_popc32(b);
    }
}

__device__ void d_random_merge(uint32_t& a, uint32_t b, uint32_t selector) {
    const uint32_t x = (selector >> 16) % 31u + 1u;
    switch (selector % 4u) {
    case 0: a = (a * 33u) + b; break;
    case 1: a = (a ^ b) * 33u; break;
    case 2: a = d_rotl32(a, x) ^ b; break;
    default: a = d_rotr32(a, x) ^ b; break;
    }
}

// hash2048 item: 64 x uint32 (256 bytes). DAG stores hash1024 items (32 x uint32);
// index i (hash2048) maps to two consecutive hash1024 at 2*i and 2*i+1.
__device__ void d_load_dag_item2048(
    const uint32_t* __restrict__ dag1024,
    uint32_t item_index,
    uint32_t out_words[64]
) {
    const uint32_t* src = dag1024 + (size_t)item_index * 64u; // 2 * 32 words
    #pragma unroll
    for (int i = 0; i < 64; i++) out_words[i] = src[i];
}

__device__ void d_hash_seed(const uint32_t header_words[8], uint64_t nonce, uint32_t seed_out[8]) {
    uint32_t state[25];
    #pragma unroll
    for (int i = 0; i < 25; i++) state[i] = 0;
    #pragma unroll
    for (int i = 0; i < 8; i++) state[i] = header_words[i];
    state[8] = (uint32_t)nonce;
    state[9] = (uint32_t)(nonce >> 32);
    state[10] = 0x00000001u;
    state[18] = 0x80008081u;
    d_keccakf800(state);
    #pragma unroll
    for (int i = 0; i < 8; i++) seed_out[i] = state[i];
}

__device__ void d_hash_final(const uint32_t seed[8], const uint32_t mix[8], uint32_t out[8]) {
    uint32_t state[25];
    #pragma unroll
    for (int i = 0; i < 25; i++) state[i] = 0;
    #pragma unroll
    for (int i = 0; i < 8; i++) state[i] = seed[i];
    #pragma unroll
    for (int i = 0; i < 8; i++) state[8 + i] = mix[i];
    state[17] = 0x00000001u;
    state[24] = 0x80008081u;
    d_keccakf800(state);
    #pragma unroll
    for (int i = 0; i < 8; i++) out[i] = state[i];
}

// Full ProgPoW mix (period seed from block_number / period_length)
__device__ void d_hash_mix(
    const uint32_t* __restrict__ dag1024,
    const uint32_t* __restrict__ l1,
    int full_dataset_num_items_1024,
    int block_number,
    uint64_t seed64,
    uint32_t mix_hash_out[8]
) {
    // init_mix
    uint32_t mix[PP_NUM_LANES][PP_NUM_REGS];
    const uint32_t z0 = d_fnv1a(FNV_OFFSET, (uint32_t)seed64);
    const uint32_t w0 = d_fnv1a(z0, (uint32_t)(seed64 >> 32));
    for (uint32_t l = 0; l < (uint32_t)PP_NUM_LANES; ++l) {
        const uint32_t jsr = d_fnv1a(w0, l);
        const uint32_t jcong = d_fnv1a(jsr, l);
        Kiss99 rng{z0, w0, jsr, jcong};
        for (int r = 0; r < PP_NUM_REGS; ++r) mix[l][r] = rng.next();
    }

    // mix_rng_state from period
    const uint64_t period_seed = (uint64_t)(block_number / PP_PERIOD_LENGTH);
    const uint32_t seed_lo = (uint32_t)period_seed;
    const uint32_t seed_hi = (uint32_t)(period_seed >> 32);
    uint32_t zz = d_fnv1a(FNV_OFFSET, seed_lo);
    uint32_t ww = d_fnv1a(zz, seed_hi);
    uint32_t jsr0 = d_fnv1a(ww, seed_lo);
    uint32_t jcong0 = d_fnv1a(jsr0, seed_hi);
    Kiss99 base_rng{zz, ww, jsr0, jcong0};

    uint32_t dst_seq[PP_NUM_REGS];
    uint32_t src_seq[PP_NUM_REGS];
    for (uint32_t i = 0; i < (uint32_t)PP_NUM_REGS; ++i) {
        dst_seq[i] = i;
        src_seq[i] = i;
    }
    for (uint32_t i = (uint32_t)PP_NUM_REGS; i > 1; --i) {
        uint32_t j = base_rng.next() % i;
        uint32_t tmp = dst_seq[i - 1]; dst_seq[i - 1] = dst_seq[j]; dst_seq[j] = tmp;
        j = base_rng.next() % i;
        tmp = src_seq[i - 1]; src_seq[i - 1] = src_seq[j]; src_seq[j] = tmp;
    }
    const uint32_t num_items_2048 = (uint32_t)(full_dataset_num_items_1024 / 2);
    const int max_ops =
        PP_NUM_CACHE_ACCESSES > PP_NUM_MATH_OPS ? PP_NUM_CACHE_ACCESSES : PP_NUM_MATH_OPS;
    const size_t num_words_per_lane = 64 / PP_NUM_LANES; // 4

    for (uint32_t r = 0; r < (uint32_t)PP_NUM_ROUNDS; ++r) {
        // Canonical 0.9.4 passes mix_rng_state by value to every round. The
        // shuffled sequences and post-shuffle RNG state therefore restart here.
        Kiss99 state_rng = base_rng;
        size_t dst_counter = 0;
        size_t src_counter = 0;
        const uint32_t item_index = mix[r % PP_NUM_LANES][0] % num_items_2048;
        uint32_t item[64];
        d_load_dag_item2048(dag1024, item_index, item);

        for (int i = 0; i < max_ops; ++i) {
            if (i < PP_NUM_CACHE_ACCESSES) {
                const uint32_t src = src_seq[(src_counter++) % (size_t)PP_NUM_REGS];
                const uint32_t dst = dst_seq[(dst_counter++) % (size_t)PP_NUM_REGS];
                const uint32_t sel = state_rng.next();
                for (int l = 0; l < PP_NUM_LANES; ++l) {
                    const size_t offset = mix[l][src] % (uint32_t)PP_L1_CACHE_WORDS;
                    d_random_merge(mix[l][dst], l1[offset], sel);
                }
            }
            if (i < PP_NUM_MATH_OPS) {
                const uint32_t src_rnd = state_rng.next() % (uint32_t)(PP_NUM_REGS * (PP_NUM_REGS - 1));
                const uint32_t src1 = src_rnd % (uint32_t)PP_NUM_REGS;
                uint32_t src2 = src_rnd / (uint32_t)PP_NUM_REGS;
                if (src2 >= src1) ++src2;
                const uint32_t sel1 = state_rng.next();
                const uint32_t dst = dst_seq[(dst_counter++) % (size_t)PP_NUM_REGS];
                const uint32_t sel2 = state_rng.next();
                for (int l = 0; l < PP_NUM_LANES; ++l) {
                    const uint32_t data = d_random_math(mix[l][src1], mix[l][src2], sel1);
                    d_random_merge(mix[l][dst], data, sel2);
                }
            }
        }

        uint32_t dsts[4];
        uint32_t sels[4];
        for (size_t i = 0; i < num_words_per_lane; ++i) {
            dsts[i] = i == 0 ? 0u : dst_seq[(dst_counter++) % (size_t)PP_NUM_REGS];
            sels[i] = state_rng.next();
        }
        for (int l = 0; l < PP_NUM_LANES; ++l) {
            const size_t offset = ((size_t)(l ^ (int)r) % (size_t)PP_NUM_LANES) * num_words_per_lane;
            for (size_t i = 0; i < num_words_per_lane; ++i) {
                d_random_merge(mix[l][dsts[i]], item[offset + i], sels[i]);
            }
        }
    }

    uint32_t lane_hash[PP_NUM_LANES];
    for (int l = 0; l < PP_NUM_LANES; ++l) {
        lane_hash[l] = FNV_OFFSET;
        for (int i = 0; i < PP_NUM_REGS; ++i)
            lane_hash[l] = d_fnv1a(lane_hash[l], mix[l][i]);
    }
    #pragma unroll
    for (int i = 0; i < 8; i++) mix_hash_out[i] = FNV_OFFSET;
    for (int l = 0; l < PP_NUM_LANES; ++l) {
        mix_hash_out[l % 8] = d_fnv1a(mix_hash_out[l % 8], lane_hash[l]);
    }
}

__device__ bool d_hash_meets_boundary(const uint32_t final_words[8], const uint8_t boundary[32]) {
    // final_hash words are little-endian uint32; compare as big-endian bytes like host.
    uint8_t fh[32];
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        uint32_t w = final_words[i];
        fh[i * 4 + 0] = (uint8_t)(w & 0xff);
        fh[i * 4 + 1] = (uint8_t)((w >> 8) & 0xff);
        fh[i * 4 + 2] = (uint8_t)((w >> 16) & 0xff);
        fh[i * 4 + 3] = (uint8_t)((w >> 24) & 0xff);
    }
    for (int b = 0; b < 32; b++) {
        if (fh[b] < boundary[b]) return true;
        if (fh[b] > boundary[b]) return false;
    }
    return true;
}

__global__ void alvenqis_firopow_search_kernel(
    const uint32_t* __restrict__ dag1024,
    const uint32_t* __restrict__ l1,
    int full_dataset_num_items_1024,
    int block_number,
    const uint32_t* __restrict__ header_words, // 8
    const uint8_t* __restrict__ boundary,      // 32
    uint64_t start_nonce,
    uint32_t max_jobs,
    uint64_t* __restrict__ nonce_out,
    uint8_t* __restrict__ final_hash_out, // 32
    uint8_t* __restrict__ mix_hash_out,   // 32
    int* __restrict__ found_out,
    unsigned long long* __restrict__ hashes_done_out
) {
    const uint32_t gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= max_jobs) return;
    if (atomicAdd(found_out, 0) != 0) return;

    const uint64_t nonce = start_nonce + (uint64_t)gid;
    uint32_t seed[8];
    d_hash_seed(header_words, nonce, seed);
    const uint64_t seed64 = (uint64_t)seed[0] | ((uint64_t)seed[1] << 32);

    uint32_t mix[8];
    d_hash_mix(dag1024, l1, full_dataset_num_items_1024, block_number, seed64, mix);

    uint32_t final_h[8];
    d_hash_final(seed, mix, final_h);
    // Avoid per-hash global atomics (destroys hashrate). Host sets hashes_done from
    // the launched job count after a successful synchronize; only track solutions here.

    if (!d_hash_meets_boundary(final_h, boundary)) return;

    // First writer wins
    if (atomicCAS(found_out, 0, 1) != 0) return;
    *nonce_out = nonce;
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        uint32_t w = final_h[i];
        final_hash_out[i * 4 + 0] = (uint8_t)(w & 0xff);
        final_hash_out[i * 4 + 1] = (uint8_t)((w >> 8) & 0xff);
        final_hash_out[i * 4 + 2] = (uint8_t)((w >> 16) & 0xff);
        final_hash_out[i * 4 + 3] = (uint8_t)((w >> 24) & 0xff);
        w = mix[i];
        mix_hash_out[i * 4 + 0] = (uint8_t)(w & 0xff);
        mix_hash_out[i * 4 + 1] = (uint8_t)((w >> 8) & 0xff);
        mix_hash_out[i * 4 + 2] = (uint8_t)((w >> 16) & 0xff);
        mix_hash_out[i * 4 + 3] = (uint8_t)((w >> 24) & 0xff);
    }
}

// ---------------------------------------------------------------------------
// Host C API
// ---------------------------------------------------------------------------
extern "C" {

struct AlvenqisCudaDeviceInfo {
    int index;
    char name[256];
    size_t total_mem;
    int multi_processor_count;
    int major;
    int minor;
};

int alvenqis_cuda_available(void) {
    int n = 0;
    if (cudaGetDeviceCount(&n) != cudaSuccess) return 0;
    return n > 0 ? n : 0;
}

int alvenqis_cuda_device_info(int index, AlvenqisCudaDeviceInfo* out) {
    if (!out) return -1;
    int n = 0;
    if (cudaGetDeviceCount(&n) != cudaSuccess || index < 0 || index >= n) return -1;
    cudaDeviceProp prop;
    if (cudaGetDeviceProperties(&prop, index) != cudaSuccess) return -1;
    out->index = index;
    int i = 0;
    for (; i < 255 && prop.name[i]; i++) out->name[i] = prop.name[i];
    out->name[i] = 0;
    out->total_mem = prop.totalGlobalMem;
    out->multi_processor_count = prop.multiProcessorCount;
    out->major = prop.major;
    out->minor = prop.minor;
    return 0;
}

struct AlvenqisCudaMiner {
    int device_index;
    uint32_t* d_dag;
    uint32_t* d_l1;
    uint32_t* d_header;
    uint8_t* d_boundary;
    uint64_t* d_nonce;
    uint8_t* d_final;
    uint8_t* d_mix;
    int* d_found;
    unsigned long long* d_hashes_done;
    int full_dataset_num_items_1024;
    int epoch_block_number; // height used to build DAG (epoch marker)
    size_t dag_bytes;
};

AlvenqisCudaMiner* alvenqis_cuda_miner_create(int device_index) {
    if (cudaSetDevice(device_index) != cudaSuccess) return nullptr;
    auto* m = new AlvenqisCudaMiner();
    m->device_index = device_index;
    m->d_dag = nullptr;
    m->d_l1 = nullptr;
    m->d_header = nullptr;
    m->d_boundary = nullptr;
    m->d_nonce = nullptr;
    m->d_final = nullptr;
    m->d_mix = nullptr;
    m->d_found = nullptr;
    m->d_hashes_done = nullptr;
    m->full_dataset_num_items_1024 = 0;
    m->epoch_block_number = -1;
    m->dag_bytes = 0;
    if (cudaMalloc(&m->d_header, 8 * sizeof(uint32_t)) != cudaSuccess ||
        cudaMalloc(&m->d_boundary, 32) != cudaSuccess ||
        cudaMalloc(&m->d_nonce, sizeof(uint64_t)) != cudaSuccess ||
        cudaMalloc(&m->d_final, 32) != cudaSuccess ||
        cudaMalloc(&m->d_mix, 32) != cudaSuccess ||
        cudaMalloc(&m->d_found, sizeof(int)) != cudaSuccess ||
        cudaMalloc(&m->d_hashes_done, sizeof(unsigned long long)) != cudaSuccess) {
        if (m->d_header) cudaFree(m->d_header);
        if (m->d_boundary) cudaFree(m->d_boundary);
        if (m->d_nonce) cudaFree(m->d_nonce);
        if (m->d_final) cudaFree(m->d_final);
        if (m->d_mix) cudaFree(m->d_mix);
        if (m->d_found) cudaFree(m->d_found);
        if (m->d_hashes_done) cudaFree(m->d_hashes_done);
        delete m;
        return nullptr;
    }
    return m;
}

void alvenqis_cuda_miner_destroy(AlvenqisCudaMiner* m) {
    if (!m) return;
    cudaSetDevice(m->device_index);
    if (m->d_dag) cudaFree(m->d_dag);
    if (m->d_l1) cudaFree(m->d_l1);
    if (m->d_header) cudaFree(m->d_header);
    if (m->d_boundary) cudaFree(m->d_boundary);
    if (m->d_nonce) cudaFree(m->d_nonce);
    if (m->d_final) cudaFree(m->d_final);
    if (m->d_mix) cudaFree(m->d_mix);
    if (m->d_found) cudaFree(m->d_found);
    if (m->d_hashes_done) cudaFree(m->d_hashes_done);
    delete m;
}

int alvenqis_cuda_miner_build_dag(
    AlvenqisCudaMiner* m,
    int block_number,
    const uint32_t* light_host,
    uint32_t light_items,
    const uint32_t* l1_host,
    uint32_t l1_words,
    int full_dataset_num_items_1024
) {
    if (!m || !light_host || light_items == 0 || !l1_host || l1_words == 0 ||
        full_dataset_num_items_1024 <= 0)
        return -1;
    if (cudaSetDevice(m->device_index) != cudaSuccess) return -2;

    const size_t dag_bytes =
        (size_t)full_dataset_num_items_1024 * 32u * sizeof(uint32_t);

    // Reuse if same epoch dataset size already loaded for this height's epoch.
    if (m->d_dag && m->dag_bytes == dag_bytes &&
        m->full_dataset_num_items_1024 == full_dataset_num_items_1024 &&
        m->epoch_block_number >= 0) {
        // Epoch length 1300 — skip re-upload when same epoch.
        int old_epoch = m->epoch_block_number / 1300;
        int new_epoch = block_number / 1300;
        if (old_epoch == new_epoch) {
            m->epoch_block_number = block_number;
            return 0;
        }
    }

    if (m->d_dag) { cudaFree(m->d_dag); m->d_dag = nullptr; }
    if (m->d_l1) { cudaFree(m->d_l1); m->d_l1 = nullptr; }

    if (cudaMalloc((void**)&m->d_dag, dag_bytes) != cudaSuccess) return -10;
    if (cudaMalloc((void**)&m->d_l1, (size_t)l1_words * sizeof(uint32_t)) != cudaSuccess) {
        cudaFree(m->d_dag); m->d_dag = nullptr;
        return -11;
    }
    if (cudaMemcpy(m->d_l1, l1_host, (size_t)l1_words * sizeof(uint32_t), cudaMemcpyHostToDevice) != cudaSuccess)
        return -20;

    uint32_t* d_light = nullptr;
    const size_t light_bytes = (size_t)light_items * 16u * sizeof(uint32_t);
    if (cudaMalloc((void**)&d_light, light_bytes) != cudaSuccess) return -21;
    if (cudaMemcpy(d_light, light_host, light_bytes, cudaMemcpyHostToDevice) != cudaSuccess) {
        cudaFree(d_light);
        return -22;
    }

    const uint32_t dataset_items_512 = (uint32_t)full_dataset_num_items_1024 * 2u;
    const int threads = 128;
    int blocks = (int)((dataset_items_512 + (uint32_t)threads - 1u) / (uint32_t)threads);
    if (blocks > 65535) blocks = 65535;
    alvenqis_build_dag_kernel<<<blocks, threads>>>(
        d_light, light_items, m->d_dag, dataset_items_512);
    cudaError_t build_error = cudaGetLastError();
    if (build_error == cudaSuccess) build_error = cudaDeviceSynchronize();
    cudaFree(d_light);
    if (build_error != cudaSuccess) return -23;

    m->dag_bytes = dag_bytes;
    m->full_dataset_num_items_1024 = full_dataset_num_items_1024;
    m->epoch_block_number = block_number;
    return 0;
}

int alvenqis_cuda_miner_copy_dag_item(
    AlvenqisCudaMiner* m,
    uint32_t index,
    uint8_t out[128]
) {
    if (!m || !m->d_dag || !out || index >= (uint32_t)m->full_dataset_num_items_1024)
        return -1;
    if (cudaSetDevice(m->device_index) != cudaSuccess) return -2;
    const uint32_t* source = m->d_dag + (size_t)index * 32u;
    return cudaMemcpy(out, source, 128, cudaMemcpyDeviceToHost) == cudaSuccess ? 0 : -3;
}

// Mine a batch on device. Returns 0 on success (found or not).
// *found_out = 1 if solution, 0 otherwise.
int alvenqis_cuda_mine_firopow(
    AlvenqisCudaMiner* m,
    int block_number,
    const uint8_t header_hash[32],
    const uint8_t boundary[32],
    uint64_t start_nonce,
    uint32_t max_jobs,
    uint64_t* nonce_out,
    uint8_t final_hash_out[32],
    uint8_t mix_hash_out[32],
    int* found_out,
    uint64_t* hashes_done_out
) {
    if (!m || !header_hash || !boundary || !nonce_out || !final_hash_out || !mix_hash_out ||
        !found_out || !hashes_done_out)
        return -1;
    *found_out = 0;
    *hashes_done_out = 0;
    if (!m->d_dag || !m->d_l1 || max_jobs == 0) return -2;
    if (cudaSetDevice(m->device_index) != cudaSuccess) return -3;

    uint32_t header_words[8];
    for (int i = 0; i < 8; i++) {
        header_words[i] =
            (uint32_t)header_hash[i * 4 + 0] |
            ((uint32_t)header_hash[i * 4 + 1] << 8) |
            ((uint32_t)header_hash[i * 4 + 2] << 16) |
            ((uint32_t)header_hash[i * 4 + 3] << 24);
    }

    int rc = 0;

    if (cudaMemcpy(m->d_header, header_words, 8 * sizeof(uint32_t), cudaMemcpyHostToDevice) != cudaSuccess) {
        rc = -20; goto cleanup;
    }
    if (cudaMemcpy(m->d_boundary, boundary, 32, cudaMemcpyHostToDevice) != cudaSuccess) {
        rc = -21; goto cleanup;
    }
    {
        int zero = 0;
        unsigned long long zero_hashes = 0;
        if (cudaMemcpy(m->d_found, &zero, sizeof(int), cudaMemcpyHostToDevice) != cudaSuccess ||
            cudaMemcpy(m->d_hashes_done, &zero_hashes, sizeof(zero_hashes), cudaMemcpyHostToDevice) != cudaSuccess) {
            rc = -22; goto cleanup;
        }
    }

    {
        const int threads = 128; // ProgPoW is register-heavy
        int blocks = (int)((max_jobs + (uint32_t)threads - 1) / (uint32_t)threads);
        if (blocks < 1) blocks = 1;
        if (blocks > 65535) blocks = 65535;
        uint32_t jobs = max_jobs;
        if (jobs > (uint32_t)blocks * (uint32_t)threads)
            jobs = (uint32_t)blocks * (uint32_t)threads;

        alvenqis_firopow_search_kernel<<<blocks, threads>>>(
            m->d_dag,
            m->d_l1,
            m->full_dataset_num_items_1024,
            block_number,
            m->d_header,
            m->d_boundary,
            start_nonce,
            jobs,
            m->d_nonce,
            m->d_final,
            m->d_mix,
            m->d_found,
            m->d_hashes_done
        );
        if (cudaGetLastError() != cudaSuccess) { rc = -30; goto cleanup; }
        if (cudaDeviceSynchronize() != cudaSuccess) { rc = -31; goto cleanup; }
        // Host-side completed work: launched job count (no per-hash atomics).
        *hashes_done_out = (uint64_t)jobs;
    }

    if (cudaMemcpy(found_out, m->d_found, sizeof(int), cudaMemcpyDeviceToHost) != cudaSuccess) {
        rc = -40; goto cleanup;
    }
    if (*found_out) {
        if (cudaMemcpy(nonce_out, m->d_nonce, sizeof(uint64_t), cudaMemcpyDeviceToHost) != cudaSuccess) {
            rc = -41; goto cleanup;
        }
        if (cudaMemcpy(final_hash_out, m->d_final, 32, cudaMemcpyDeviceToHost) != cudaSuccess) {
            rc = -42; goto cleanup;
        }
        if (cudaMemcpy(mix_hash_out, m->d_mix, 32, cudaMemcpyDeviceToHost) != cudaSuccess) {
            rc = -43; goto cleanup;
        }
    }

cleanup:
    return rc;
}

int alvenqis_cuda_device_kernels_linked(void) {
    return 1;
}

} // extern "C"
