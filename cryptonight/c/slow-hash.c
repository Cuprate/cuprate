#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <unistd.h>

#include "int-util.h"
#include "hash-ops.h"
#include "oaes_lib.h"
#include "variant2_int_sqrt.h"
#include "variant4_random_math.h"
#include "keccak.h"

#define MEMORY         (1 << 21) // 2MB scratchpad
#define ITER           (1 << 20)
#define AES_BLOCK_SIZE  16
#define AES_KEY_SIZE    32
#define INIT_SIZE_BLK   8
#define INIT_SIZE_BYTE (INIT_SIZE_BLK * AES_BLOCK_SIZE)

extern void aesb_single_round(const uint8_t *in, uint8_t *out, const uint8_t *expandedKey);

extern void aesb_pseudo_round(const uint8_t *in, uint8_t *out, const uint8_t *expandedKey);

void print_hex(const char *name, const void *memory, size_t size) {
  const unsigned char *bytes = (const unsigned char *) memory;
  if (strlen(name) > 0) {
    printf("%s: ", name);
  } else {
    printf("    ");
  }
  printf("\"");
  for (size_t i = 0; i < size; ++i) {
    printf("%02x", bytes[i]);
  }
  printf("\",\n");
}


#define VARIANT1_1(p) \
  do if (variant == 1) \
  { \
    const uint8_t tmp = ((const uint8_t*)(p))[11]; \
    static const uint32_t table = 0x75310; \
    const uint8_t index = (((tmp >> 3) & 6) | (tmp & 1)) << 1; \
    ((uint8_t*)(p))[11] = tmp ^ ((table >> index) & 0x30); \
  } while(0)

#define VARIANT1_2(p) \
  do if (variant == 1) \
  { \
    xor64(p, tweak1_2); \
  } while(0)

#define VARIANT1_CHECK() \
  do if (length < 43) \
  { \
    fprintf(stderr, "Cryptonight variant 1 needs at least 43 bytes of data"); \
    _exit(1); \
  } while(0)

#define NONCE_POINTER (((const uint8_t*)data)+35)


#define U64(x) ((uint64_t *) (x))

#define VARIANT1_PORTABLE_INIT() \
  uint8_t tweak1_2[8]; \
  do if (variant == 1) \
  { \
    VARIANT1_CHECK(); \
    memcpy(&tweak1_2, &state.hs.b[192], sizeof(tweak1_2)); \
    xor64(tweak1_2, NONCE_POINTER); \
  } while(0)


#define VARIANT2_PORTABLE_INIT() \
  uint64_t division_result = 0; \
  uint64_t sqrt_result = 0; \
  do if (variant >= 2) \
  { \
    memcpy(b + AES_BLOCK_SIZE, state.hs.b + 64, AES_BLOCK_SIZE); \
    xor64(b + AES_BLOCK_SIZE, state.hs.b + 80); \
    xor64(b + AES_BLOCK_SIZE + 8, state.hs.b + 88); \
    division_result = SWAP64LE(state.hs.w[12]); \
    sqrt_result = SWAP64LE(state.hs.w[13]); \
  } while (0)

void variant2_portable_shuffle_add(
        uint8_t out[16],
        const uint8_t a[16],
        const uint8_t b[16 * 2],
        const uint8_t long_state[2097152],
        const size_t offset,
        const int variant) {
  if (variant >= 2) {
    uint64_t *chunk1 = U64((long_state) + ((offset) ^ 0x10));
    uint64_t *chunk2 = U64((long_state) + ((offset) ^ 0x20));
    uint64_t *chunk3 = U64((long_state) + ((offset) ^ 0x30));

    uint64_t chunk1_old[2] = {SWAP64LE(chunk1[0]), SWAP64LE(chunk1[1])};
    const uint64_t chunk2_old[2] = {SWAP64LE(chunk2[0]), SWAP64LE(chunk2[1])};
    const uint64_t chunk3_old[2] = {SWAP64LE(chunk3[0]), SWAP64LE(chunk3[1])};

    uint64_t b1[2];
    memcpy_swap64le(b1, b + 16, 2);
    chunk1[0] = SWAP64LE(chunk3_old[0] + b1[0]);
    chunk1[1] = SWAP64LE(chunk3_old[1] + b1[1]);

    uint64_t a0[2];
    memcpy_swap64le(a0, a, 2);
    chunk3[0] = SWAP64LE(chunk2_old[0] + a0[0]);
    chunk3[1] = SWAP64LE(chunk2_old[1] + a0[1]);

    uint64_t b0[2];
    memcpy_swap64le(b0, b, 2);
    chunk2[0] = SWAP64LE(chunk1_old[0] + b0[0]);
    chunk2[1] = SWAP64LE(chunk1_old[1] + b0[1]);

    if (variant >= 4) {
      uint64_t out_copy[2];
      memcpy_swap64le(out_copy, out, 2);
      chunk1_old[0] ^= chunk2_old[0];
      chunk1_old[1] ^= chunk2_old[1];
      out_copy[0] ^= chunk3_old[0];
      out_copy[1] ^= chunk3_old[1];
      out_copy[0] ^= chunk1_old[0];
      out_copy[1] ^= chunk1_old[1];
      memcpy_swap64le(out, out_copy, 2);
    }
  }
}

#define VARIANT2_INTEGER_MATH_DIVISION_STEP(b, ptr) \
  uint64_t tmpx = division_result ^ (sqrt_result << 32); \
  ((uint64_t*)(b))[0] ^= SWAP64LE(tmpx); \
  { \
    const uint64_t dividend = SWAP64LE(((uint64_t*)(ptr))[1]); \
    const uint32_t divisor = (SWAP64LE(((uint64_t*)(ptr))[0]) + (uint32_t)(sqrt_result << 1)) | 0x80000001UL; \
    division_result = ((uint32_t)(dividend / divisor)) + \
                     (((uint64_t)(dividend % divisor)) << 32); \
  } \
  const uint64_t sqrt_input = SWAP64LE(((uint64_t*)(ptr))[0]) + division_result


// double precision floating point type has enough bits of precision on current platform
void variant2_integer_math(
        volatile uint8_t c1[16],
        const uint8_t c2[16],
        uint64_t *division_result,
        uint64_t *sqrt_result, // in-out parameter
        const int variant
) {
  //    static size_t i = 0;
  //    const size_t INTERVAL = 250000;
  //    if (i % INTERVAL == 0) {
  //      printf("test(\n");
  //      print_hex("", c1, 16);
  //      print_hex("", c2, 16);
  //      printf("%lu,\n", *division_result);
  //      printf("%lu,\n", *sqrt_result);
  //    }
  if ((variant == 2) || (variant == 3)) {
    uint64_t tmpx = *division_result ^ (*sqrt_result << 32);
    ((uint64_t *) (c1))[0] ^= ((uint64_t) (tmpx));

    const uint64_t dividend = ((uint64_t) SWAP64LE(((uint64_t *) (c2))[1]));
    const uint32_t divisor =
            (((uint64_t) SWAP64LE(((uint64_t *) (c2))[0])) + (uint32_t) (*sqrt_result << 1)) | 0x80000001UL;
    *division_result = ((uint32_t) (dividend / divisor)) + SWAP64LE(((uint64_t) (dividend % divisor)) << 32);

    const uint64_t sqrt_input = SWAP64LE((uint64_t) (((uint64_t *) (c2))[0])) + *division_result;
    *sqrt_result = sqrt(sqrt_input + 18446744073709551616.0) * 2.0 - 8589934592.0;
    const uint64_t sr_div2 = *sqrt_result >> 1;
    const uint64_t lsb = *sqrt_result & 1;
    const uint64_t r2 = (uint64_t) (sr_div2) * (sr_div2 + lsb) + (*sqrt_result << 32);
    *sqrt_result += ((r2 + lsb > sqrt_input) ? -1 : 0) + ((r2 + (1ULL << 32) < sqrt_input - sr_div2) ? 1 : 0);

    //      if (i % INTERVAL == 0) {
    //        print_hex("", c1, 16);
    //        printf("    %lu,\n", *division_result);
    //        printf("    %lu,\n", *sqrt_result);
    //        printf(");\n");
    //        fflush(stdout);
    //      }
    //      i++;

  }
}


/*
#define VARIANT2_2_PORTABLE() \
    if (variant == 2 || variant == 3) { \
      xor_blocks(long_state + (j ^ 0x10), d); \
      xor_blocks(d, long_state + (j ^ 0x20)); \
    }
*/

#define V4_REG_LOAD(dst, src) \
  do { \
    memcpy((dst), (src), sizeof(uint32_t)); \
    *(dst) = SWAP32LE(*(dst)); \
  } while (0)

/*
#define VARIANT4_RANDOM_MATH_INIT() \
  uint32_t r[9]; \
  struct V4_Instruction code[NUM_INSTRUCTIONS_MAX + 1]; \
  do if (variant >= 4) \
  { \
    for (int i = 0; i < 4; ++i) \
      V4_REG_LOAD(r + i, (uint8_t*)(state.hs.w + 12) + sizeof(uint32_t) * i); \
    v4_random_math_init(code, height); \
  } while (0)
*/

void VARIANT4_RANDOM_MATH(
        uint8_t a1[16],
        uint8_t c2[16],
        uint32_t r[9],
        uint8_t b_1st16[16],
        uint8_t b_2nd16[16],
        int variant,
        const struct V4_Instruction *code) {
  if (variant >= 4) {
    uint64_t t[2];
    memcpy(t, c2, sizeof(uint64_t));

    t[0] ^= SWAP64LE((r[0] + r[1]) | ((uint64_t) (r[2] + r[3]) << 32));
    memcpy(c2, t, sizeof(uint64_t));

    V4_REG_LOAD(r + 4, a1);
    V4_REG_LOAD(r + 5, (uint64_t *) (a1) + 1);
    V4_REG_LOAD(r + 6, b_1st16);
    V4_REG_LOAD(r + 7, b_2nd16);
    V4_REG_LOAD(r + 8, (uint64_t *) (b_2nd16) + 1);

    v4_random_math(code, r);

    memcpy(t, a1, sizeof(uint64_t) * 2);

    t[0] ^= SWAP64LE(r[2] | ((uint64_t) (r[3]) << 32));
    t[1] ^= SWAP64LE(r[0] | ((uint64_t) (r[1]) << 32));

    memcpy(a1, t, sizeof(uint64_t) * 2);
  }
}

static void (*const extra_hashes[4])(const void *, size_t, char *) = {
        hash_extra_blake, hash_extra_groestl, hash_extra_jh, hash_extra_skein
};

static size_t e2i(const uint8_t *a, size_t count) {
  return (SWAP64LE(*((uint64_t *) a)) / AES_BLOCK_SIZE) & (count - 1);
}

/*static*/ void mul(const uint8_t *a, const uint8_t *b, uint8_t *res) {
  uint64_t a0, b0;
  uint64_t hi, lo;

  a0 = SWAP64LE(((uint64_t *) a)[0]);
  b0 = SWAP64LE(((uint64_t *) b)[0]);
  lo = mul128(a0, b0, &hi);
  ((uint64_t *) res)[0] = SWAP64LE(hi);
  ((uint64_t *) res)[1] = SWAP64LE(lo);
}

static void sum_half_blocks(uint8_t *a, const uint8_t *b) {
  uint64_t a0, a1, b0, b1;

  a0 = SWAP64LE(((uint64_t *) a)[0]);
  a1 = SWAP64LE(((uint64_t *) a)[1]);
  b0 = SWAP64LE(((uint64_t *) b)[0]);
  b1 = SWAP64LE(((uint64_t *) b)[1]);
  a0 += b0;
  a1 += b1;
  ((uint64_t *) a)[0] = SWAP64LE(a0);
  ((uint64_t *) a)[1] = SWAP64LE(a1);
}


static void copy_block(uint8_t *dst, const uint8_t *src) {
  memcpy(dst, src, AES_BLOCK_SIZE);
}

static void swap_blocks(uint8_t *a, uint8_t *b) {
  uint64_t t[2];
  U64(t)[0] = U64(a)[0];
  U64(t)[1] = U64(a)[1];
  U64(a)[0] = U64(b)[0];
  U64(a)[1] = U64(b)[1];
  U64(b)[0] = U64(t)[0];
  U64(b)[1] = U64(t)[1];
}

static void xor_blocks(uint8_t *a, const uint8_t *b) {
  size_t i;
  for (i = 0; i < AES_BLOCK_SIZE; i++) {
    a[i] ^= b[i];
  }
}

static void xor64(uint8_t *left, const uint8_t *right) {
  size_t i;
  for (i = 0; i < 8; ++i) {
    left[i] ^= right[i];
  }
}

#pragma pack(push, 1)
union cn_slow_hash_state {
    union hash_state hs;
    struct {
        uint8_t k[64];
        uint8_t init[INIT_SIZE_BYTE];
    };
};
#pragma pack(pop)



/*
void print_r_and_code(const uint32_t r[9], const struct V4_Instruction code[NUM_INSTRUCTIONS_MAX + 1]) {
  printf("        let r: [u32; 9] = [");
  for (int i = 0; i < 9; ++i) {
    printf("%u", r[i]);
    if (i < 8) {
      printf(", ");
    }
  }
  printf("];\n");

  printf("        let code:[Instruction; 71] = [\n");
  for (int i = 0; i < NUM_INSTRUCTIONS_MAX + 1; ++i) {
    printf("            Instruction {opcode: %u, dst_index: %u, src_index: %u, c: %u}",
           code[i].opcode, code[i].dst_index, code[i].src_index, code[i].C);
    if (i < NUM_INSTRUCTIONS_MAX) {
      printf(",\n");
    } else {
      printf("\n");
    }
  }
  printf("        ];\n");
}
*/
/*
void print_text_a_and_b(const uint8_t text[INIT_SIZE_BYTE], const uint8_t a[AES_BLOCK_SIZE], const uint8_t b[AES_BLOCK_SIZE * 2]) {
  printf("text: ");
    for (int i = 0; i < INIT_SIZE_BYTE; ++i) {
        printf("%02x", text[i]);
    }
    printf("\n");

    printf("a: ");
    for (int i = 0; i < AES_BLOCK_SIZE; ++i) {
        printf("%02x", a[i]);
    }
    printf("\n");

    printf("b: ");
    for (int i = 0; i < AES_BLOCK_SIZE * 2; ++i) {
        printf("%02x", b[i]);
    }
    printf("\n");
}
*/

void cn_slow_hash(const void *data, size_t length, char *hash, int variant, uint64_t height) {
  uint8_t long_state[MEMORY] = {0};

  union cn_slow_hash_state state = {0};
  uint8_t text[INIT_SIZE_BYTE] = {0};
  uint8_t a[AES_BLOCK_SIZE] = {0};
  uint8_t a1[AES_BLOCK_SIZE] = {0};
  uint8_t b[AES_BLOCK_SIZE * 2] = {0};
  uint8_t c1[AES_BLOCK_SIZE] = {0};
  uint8_t c2[AES_BLOCK_SIZE] = {0};
  uint8_t d[AES_BLOCK_SIZE] = {0};
  size_t i, j;
  uint8_t aes_key[AES_KEY_SIZE] = {0};
  oaes_ctx *aes_ctx = NULL;

  keccak1600(data, length, state.hs.b);
  memcpy(text, state.init, INIT_SIZE_BYTE);
  //print_hex("text(1): ", text, sizeof(text));
  memcpy(aes_key, state.hs.b, AES_KEY_SIZE);
  aes_ctx = (oaes_ctx *) oaes_alloc();

  // VARIANT1_PORTABLE_INIT();
  uint8_t tweak1_2[8];
  if (variant == 1) {
    // VARIANT1_CHECK();
    if (length < 43) {
      fprintf(stderr, "Cryptonight variant 1 needs at least 43 bytes of data");
      _exit(1);
    }

    memcpy(&tweak1_2, &state.hs.b[192], sizeof(tweak1_2));
    xor64(tweak1_2, NONCE_POINTER);
  }

  // VARIANT2_PORTABLE_INIT();
  uint64_t division_result = 0;
  uint64_t sqrt_result = 0;
  if (variant >= 2) {
    memcpy(b + AES_BLOCK_SIZE, state.hs.b + 64, AES_BLOCK_SIZE);
    xor64(b + AES_BLOCK_SIZE, state.hs.b + 80);
    xor64(b + AES_BLOCK_SIZE + 8, state.hs.b + 88);
    division_result = SWAP64LE(state.hs.w[12]);
    sqrt_result = SWAP64LE(state.hs.w[13]);
  }

  //    print_hex("b", b, 32);
  //    print_hex("state.hs.b", state.hs.b, 200);
  //    printf("division_result: %lu\n", division_result);
  //    printf("sqrt_result: %lu\n", sqrt_result);

  // VARIANT4_RANDOM_MATH_INIT();
  uint32_t r[9];
  struct V4_Instruction code[NUM_INSTRUCTIONS_MAX + 1];
  if (variant >= 4) {
    for (i = 0; i < 4; ++i) {
      V4_REG_LOAD(r + i, (uint8_t *) (state.hs.w + 12) + sizeof(uint32_t) * i);
    }
    v4_random_math_init(code, height);
  }

  //print_r_and_code(r, code);

  oaes_key_import_data(aes_ctx, aes_key, AES_KEY_SIZE);
  for (i = 0; i < MEMORY / INIT_SIZE_BYTE; i++) {
    for (j = 0; j < INIT_SIZE_BLK; j++) {
      aesb_pseudo_round(&text[AES_BLOCK_SIZE * j], &text[AES_BLOCK_SIZE * j], aes_ctx->key->exp_data);
    }
    memcpy(&long_state[i * INIT_SIZE_BYTE], text, INIT_SIZE_BYTE);
  }

  for (i = 0; i < AES_BLOCK_SIZE; i++) {
    a[i] = state.k[i] ^ state.k[AES_BLOCK_SIZE * 2 + i];
    b[i] = state.k[AES_BLOCK_SIZE + i] ^ state.k[AES_BLOCK_SIZE * 3 + i];
  }

  //print_text_a_and_b(text, a, b);

  for (i = 0; i < ITER / 2; i++) {
    /* Dependency chain: address -> read value ------+
     * written value <-+ hard function (AES or MUL) <+
     * next address  <-+
     */
    /* Iteration 1 */
    j = e2i(a, MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
    copy_block(c1, &long_state[j]);
    aesb_single_round(c1, c1, a);
    variant2_portable_shuffle_add(c1, a, b, long_state, j, variant);
//    if (i == 0) {
//      printf("i = %ld, j = %ld\n", i, j);
//      print_hex("c1", c1, sizeof(c1));
//      uint8_t output[32];
//      hash_extra_blake(long_state, 32, (char *)output);
//      print_hex("long_state", output, 32);
//    }
    copy_block(&long_state[j], c1);
    xor_blocks(&long_state[j], b);
    assert(j == e2i(a, MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE);
    VARIANT1_1(&long_state[j]);

    /* Iteration 2 */
    j = e2i(c1, MEMORY / AES_BLOCK_SIZE) * AES_BLOCK_SIZE;
    copy_block(c2, &long_state[j]);
    copy_block(a1, a);
    variant2_integer_math(c2, c1, &division_result, &sqrt_result, variant);
    VARIANT4_RANDOM_MATH(a1, c2, r, b, b + AES_BLOCK_SIZE, variant, code);
    mul(c1, c2, d);

    // VARIANT2_2_PORTABLE();
    if (variant == 2 || variant == 3) {
      xor_blocks(long_state + (j ^ 0x10), d);
      xor_blocks(d, long_state + (j ^ 0x20));
    }

    variant2_portable_shuffle_add(c1, a, b, long_state, j, variant);
    sum_half_blocks(a1, d);
    swap_blocks(a1, c2);
    xor_blocks(a1, c2);
    VARIANT1_2(c2 + 8);
    copy_block(&long_state[j], c2);
    if (variant >= 2) {
      copy_block(b + AES_BLOCK_SIZE, b);
    }
    copy_block(b, c1);
    copy_block(a, a1);
  }

  //print_hex("a", text, sizeof(a));
  //print_hex("b", text, sizeof(b));

  memcpy(text, state.init, INIT_SIZE_BYTE);
  oaes_key_import_data(aes_ctx, &state.hs.b[32], AES_KEY_SIZE);
  for (i = 0; i < MEMORY / INIT_SIZE_BYTE; i++) {
    for (j = 0; j < INIT_SIZE_BLK; j++) {
      xor_blocks(&text[j * AES_BLOCK_SIZE], &long_state[i * INIT_SIZE_BYTE + j * AES_BLOCK_SIZE]);
      aesb_pseudo_round(&text[AES_BLOCK_SIZE * j], &text[AES_BLOCK_SIZE * j], aes_ctx->key->exp_data);
    }
  }
  memcpy(state.init, text, INIT_SIZE_BYTE);
  hash_permutation(&state.hs);
  /*memcpy(hash, &state, 32);*/
  extra_hashes[state.hs.b[0] & 3](&state, 200, hash);
  oaes_free((OAES_CTX **) &aes_ctx);
}
