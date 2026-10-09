#!/usr/bin/env python3
"""Native C runtime sanitizer and deterministic allocation-failure checks."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

HARNESS = r'''
#include "dev_runtime.h"
#include <assert.h>
#include <stdlib.h>
#include <string.h>
static int fail_after = -1;
static int fail(void) {
    if (fail_after < 0) return 0;
    if (fail_after == 0) return 1;
    fail_after--; return 0;
}
void *test_malloc(size_t size) { return fail() ? NULL : malloc(size); }
void *test_calloc(size_t count, size_t size) { return fail() ? NULL : calloc(count, size); }
void *test_realloc(void *ptr, size_t size) { return fail() ? NULL : realloc(ptr, size); }
int main(void) {
    fail_after = 0; assert(dvr_string_new("x") == NULL);
    fail_after = 1; assert(dvr_string_new("x") == NULL);
    fail_after = 2;
    assert(dvr_string_new("0123456789012345678901234567890123456789") == NULL);
    fail_after = -1;
    void *s = dvr_string_new("seed"); assert(s);
    fail_after = 0;
    assert(!dvr_string_append(s, "0123456789012345678901234567890123456789"));
    assert(dvr_string_len(s) == 4 && !strcmp(dvr_string_view(s), "seed"));
    assert(dvr_string_clone(s) == NULL);
    assert(!dvr_string_append_bytes(s, (uint8_t *)"x", SIZE_MAX));
    fail_after = -1;
    for (int i = 0; i < 15; i++) {
        size_t previous = dvr_string_len(s);
        assert(dvr_string_append(s, dvr_string_view(s)));
        assert(dvr_string_len(s) == previous * 2);
    }
    uint8_t binary[] = {0, 1, 255, 0};
    assert(dvr_string_append_bytes(s, binary, sizeof(binary)));
    void *copy = dvr_string_clone(s); assert(copy && dvr_string_equal(copy, s));
    assert(!dvr_string_append_bytes(s, (uint8_t *)dvr_string_view(s) + dvr_string_len(s), 1));
    dvr_string_clear(s); assert(dvr_string_len(s) == 0);
    assert(!dvr_string_equal(s, copy));
    dvr_string_free(copy); dvr_string_free(s);
    assert(dvr_alloc_array(SIZE_MAX, 2) == NULL);
    uint8_t *p = dvr_alloc_array(8, 1); assert(p && p[7] == 0);
    dvr_fill(p, 9, 8); dvr_copy(p + 1, p, 7); assert(p[7] == 9);
    fail_after = 0;
    assert(dvr_resize(p, 1024) == NULL && p[7] == 9);
    fail_after = -1;
    assert(dvr_resize(p, 0) == NULL);
    dvr_copy(NULL, NULL, 0); dvr_fill(NULL, 0, 0); dvr_free(NULL);
    assert(dvr_file_open("unused", "invalid") == NULL);
    assert(dvr_read_line(NULL, 0) == -2);
    uint64_t before = dvr_now_ns(); assert(before > 0);
    assert(dvr_sleep_ms(0)); assert(dvr_now_ns() >= before);
    return 0;
}
'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--sanitize", action="store_true")
    args = parser.parse_args()
    runtime = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="dev-runtime-c-") as directory:
        root = Path(directory)
        source = root / "test.c"
        source.write_text(HARNESS)
        flags = ["-std=c11", "-O1", "-g", "-Wall", "-Wextra", "-Werror", "-I", str(runtime / "include")]
        if args.sanitize:
            flags += ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]
        objects = []
        def run(command):
            subprocess.run(command, cwd=root, check=True, timeout=120)
        for name in ("memory", "strings", "io", "time"):
            obj = root / (name + ".o")
            run([args.cc, *flags, "-Dmalloc=test_malloc", "-Dcalloc=test_calloc", "-Drealloc=test_realloc",
                 "-c", str(runtime / "src" / (name + ".c")), "-o", str(obj)])
            objects.append(str(obj))
        exe = root / ("test.exe" if os.name == "nt" else "test")
        run([args.cc, *flags, str(source), *objects, "-o", str(exe)])
        run([str(exe)])
    print("PASS: runtime ownership, growth, aliasing, binary bytes, overflow and deterministic OOM" + (" (ASan/UBSan)" if args.sanitize else ""))


if __name__ == "__main__":
    main()
