#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static uint64_t read_number(void) {
    int c = getchar();
    while (c >= 0 && c <= 32) c = getchar();
    uint64_t value = 0;
    while (c >= 48 && c <= 57) {
        value = value * 10 + (uint64_t)(c - 48);
        c = getchar();
    }
    return value;
}

uint64_t sum_loop(uint64_t n) {
    uint64_t total = 0, i = 0;
    while (i < n) {
        total += i & 255;
        i += 1;
    }
    return total;
}

uint64_t xorshift(uint64_t n, uint64_t seed) {
    uint64_t state = seed, i = 0;
    while (i < n) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        i += 1;
    }
    return state;
}

uint64_t array_update(size_t n, uint64_t seed, uint64_t rounds) {
    uint64_t *memory = malloc(n * sizeof(uint64_t));
    if (memory == NULL) return 0;
    size_t i = 0;
    while (i < n) {
        memory[i] = (seed + i) & UINT64_C(0xffffffff);
        i += 1;
    }
    uint64_t round = 0;
    while (round < rounds) {
        i = 0;
        while (i < n) {
            memory[i] = (memory[i] * UINT64_C(1664525) + UINT64_C(1013904223)) & UINT64_C(0xffffffff);
            i += 1;
        }
        round += 1;
    }
    uint64_t total = 0;
    i = 0;
    while (i < n) {
        total += memory[i];
        i += 1;
    }
    free(memory);
    return total;
}

int main(void) {
    uint64_t mode = read_number(), n = read_number();
    uint64_t seed = read_number(), rounds = read_number(), result;
    if (mode == 1) result = sum_loop(n);
    else if (mode == 2) result = xorshift(n, seed);
    else if (mode == 3) result = array_update((size_t)n, seed, rounds);
    else return 1;
    printf("%llu\n", (unsigned long long)result);
    return 0;
}
