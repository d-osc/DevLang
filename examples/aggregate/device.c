#include <stdint.h>
typedef struct { int32_t x; int32_t y; } Pair;
Pair pair_add(Pair value, int32_t extra) {
    value.x += extra;
    value.y += extra;
    return value;
}
int32_t call_twice(int32_t (*op)(int32_t), int32_t value) {
    return op(op(value));
}
