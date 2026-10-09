#ifndef DVR_RUNTIME_H
#define DVR_RUNTIME_H
#include <stdint.h>
#include <stddef.h>
/* ABI v1: opaque handles, explicit ownership, UTF-8 byte lengths. */
void *dvr_alloc(size_t bytes);
void *dvr_alloc_array(size_t count, size_t size);
void *dvr_resize(void *data, size_t bytes);
void dvr_free(void *data);
void dvr_copy(void *dest, void *src, size_t bytes);
void dvr_fill(void *dest, uint8_t value, size_t bytes);
void *dvr_string_new(const char *text);
void *dvr_string_clone(void *handle);
size_t dvr_string_len(void *handle);
const char *dvr_string_view(void *handle);
int32_t dvr_string_append(void *handle, const char *text);
int32_t dvr_string_append_bytes(void *handle, uint8_t *data, size_t bytes);
int32_t dvr_string_equal(void *a, void *b);
void dvr_string_clear(void *handle);
void dvr_string_free(void *handle);
void *dvr_file_open(const char *path, const char *mode);
size_t dvr_file_read(void *file, uint8_t *data, size_t bytes);
size_t dvr_file_write(void *file, uint8_t *data, size_t bytes);
size_t dvr_file_write_text(void *file, const char *text);
int32_t dvr_file_flush(void *file);
int32_t dvr_file_close(void *file);
int32_t dvr_file_error(void *file);
int32_t dvr_file_eof(void *file);
int32_t dvr_write(const char *text);
int32_t dvr_writeln(const char *text);
int32_t dvr_stdout_flush(void);
intptr_t dvr_read_line(uint8_t *data, size_t capacity);
uint64_t dvr_now_ns(void);
int32_t dvr_sleep_ms(uint64_t milliseconds);
#endif
