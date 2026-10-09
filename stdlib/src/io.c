#include "dev_runtime.h"
#include <stdio.h>
#include <string.h>
#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdlib.h>
static wchar_t *utf16(const char *text) {
    int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text, -1, NULL, 0);
    if (!count || (size_t)count > SIZE_MAX / sizeof(wchar_t)) return NULL;
    wchar_t *wide = (wchar_t *)malloc((size_t)count * sizeof(wchar_t));
    if (wide && !MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text, -1, wide, count)) {
        free(wide); return NULL;
    }
    return wide;
}
#endif

void *dvr_file_open(const char *path, const char *mode) {
    if (!path || !mode) return NULL;
    static const char *const modes[] = {"r", "w", "a", "rb", "wb", "ab", "r+", "w+", "a+",
        "r+b", "w+b", "a+b", "rb+", "wb+", "ab+"};
    size_t index = 0;
    while (index < sizeof(modes) / sizeof(modes[0]) && strcmp(mode, modes[index])) index++;
    if (index == sizeof(modes) / sizeof(modes[0])) return NULL;
#ifdef _WIN32
    wchar_t *wpath = utf16(path), *wmode = utf16(mode);
    FILE *file = NULL;
    if (wpath && wmode) {
#ifdef _MSC_VER
        if (_wfopen_s(&file, wpath, wmode)) file = NULL;
#else
        file = _wfopen(wpath, wmode);
#endif
    }
    free(wpath); free(wmode);
    return file;
#else
    return fopen(path, mode);
#endif
}
size_t dvr_file_read(void *file, uint8_t *data, size_t bytes) {
    if (!file || (!data && bytes) || !bytes) return 0;
    return fread(data, 1, bytes, (FILE *)file);
}
size_t dvr_file_write(void *file, uint8_t *data, size_t bytes) {
    if (!file || (!data && bytes) || !bytes) return 0;
    return fwrite(data, 1, bytes, (FILE *)file);
}
size_t dvr_file_write_text(void *file, const char *text) {
    if (!text) return 0;
    return dvr_file_write(file, (uint8_t *)text, strlen(text));
}
int32_t dvr_file_flush(void *file) { return file ? fflush((FILE *)file) : -1; }
int32_t dvr_file_close(void *file) { return file ? fclose((FILE *)file) : -1; }
int32_t dvr_file_error(void *file) { return file ? ferror((FILE *)file) != 0 : 1; }
int32_t dvr_file_eof(void *file) { return file ? feof((FILE *)file) != 0 : 0; }
int32_t dvr_write(const char *text) {
    if (!text) return 0;
    size_t bytes = strlen(text);
    return fwrite(text, 1, bytes, stdout) == bytes;
}
int32_t dvr_writeln(const char *text) {
    return dvr_write(text) && fputc('\n', stdout) != EOF;
}
int32_t dvr_stdout_flush(void) { return fflush(stdout); }
intptr_t dvr_read_line(uint8_t *data, size_t capacity) {
    if (!data || capacity < 2 || capacity - 1 > INTPTR_MAX) return -2;
    size_t length = 0;
    while (length < capacity - 1) {
        int value = getchar();
        if (value == EOF) {
            data[length] = 0;
            if (ferror(stdin)) return -2;
            return length ? (intptr_t)length : -1;
        }
        if (value == '\n') {
            if (length && data[length - 1] == '\r') length--;
            break;
        }
        data[length++] = (uint8_t)value;
    }
    data[length] = 0;
    return (intptr_t)length;
}
