#include "dev_runtime.h"
#include <stdlib.h>
#include <string.h>

void *dvr_alloc(size_t bytes) { return malloc(bytes ? bytes : 1); }
void *dvr_alloc_array(size_t count, size_t size) {
    if (size && count > SIZE_MAX / size) return NULL;
    return calloc(count && size ? count : 1, count && size ? size : 1);
}
void *dvr_resize(void *data, size_t bytes) {
    if (!bytes) { free(data); return NULL; }
    return realloc(data, bytes);
}
void dvr_free(void *data) { free(data); }
/* Copy supports overlap. Caller supplies valid regions for nonzero lengths. */
void dvr_copy(void *dest, void *src, size_t bytes) {
    if (bytes) memmove(dest, src, bytes);
}
void dvr_fill(void *dest, uint8_t value, size_t bytes) {
    if (bytes) memset(dest, value, bytes);
}
