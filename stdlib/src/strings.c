#include "dev_runtime.h"
#include <stdlib.h>
#include <string.h>

typedef struct { char *data; size_t len, capacity; } dvr_string;
static int reserve(dvr_string *s, size_t extra) {
    if (extra > SIZE_MAX - s->len - 1) return 0;
    size_t required = s->len + extra + 1;
    if (required <= s->capacity) return 1;
    size_t capacity = s->capacity;
    while (capacity < required) {
        if (capacity > SIZE_MAX / 2) { capacity = required; break; }
        capacity *= 2;
    }
    char *data = (char *)realloc(s->data, capacity);
    if (!data) return 0;
    s->data = data;
    s->capacity = capacity;
    return 1;
}
void *dvr_string_new(const char *text) {
    dvr_string *s = (dvr_string *)malloc(sizeof(*s));
    if (!s) return NULL;
    s->len = 0; s->capacity = 32;
    s->data = (char *)malloc(s->capacity);
    if (!s->data) { free(s); return NULL; }
    s->data[0] = 0;
    if (text && !dvr_string_append(s, text)) { dvr_string_free(s); return NULL; }
    return s;
}
int32_t dvr_string_append_bytes(void *handle, uint8_t *data, size_t bytes) {
    dvr_string *s = (dvr_string *)handle;
    if (!s || (!data && bytes)) return 0;
    if (!bytes) return 1;
    /* Keep a self-append source valid when growth reallocates the buffer. */
    uintptr_t address = (uintptr_t)data, base = (uintptr_t)s->data;
    int alias = address >= base && address - base <= s->len;
    size_t offset = alias ? (size_t)(address - base) : 0;
    if (alias && bytes > s->len - offset) return 0;
    if (!reserve(s, bytes)) return 0;
    if (alias) data = (uint8_t *)s->data + offset;
    memmove(s->data + s->len, data, bytes);
    s->len += bytes; s->data[s->len] = 0;
    return 1;
}
int32_t dvr_string_append(void *handle, const char *text) {
    if (!text) return 0;
    return dvr_string_append_bytes(handle, (uint8_t *)text, strlen(text));
}
void *dvr_string_clone(void *handle) {
    const dvr_string *s = (const dvr_string *)handle;
    if (!s) return NULL;
    void *copy = dvr_string_new(NULL);
    if (copy && !dvr_string_append_bytes(copy, (uint8_t *)s->data, s->len)) {
        dvr_string_free(copy); return NULL;
    }
    return copy;
}
size_t dvr_string_len(void *handle) {
    const dvr_string *s = (const dvr_string *)handle;
    return s ? s->len : 0;
}
const char *dvr_string_view(void *handle) {
    const dvr_string *s = (const dvr_string *)handle;
    return s ? s->data : "";
}
int32_t dvr_string_equal(void *a, void *b) {
    const dvr_string *x = (const dvr_string *)a, *y = (const dvr_string *)b;
    if (!x || !y) return x == y;
    return x->len == y->len && memcmp(x->data, y->data, x->len) == 0;
}
void dvr_string_clear(void *handle) {
    dvr_string *s = (dvr_string *)handle;
    if (s) { s->len = 0; s->data[0] = 0; }
}
void dvr_string_free(void *handle) {
    dvr_string *s = (dvr_string *)handle;
    if (s) { free(s->data); free(s); }
}
