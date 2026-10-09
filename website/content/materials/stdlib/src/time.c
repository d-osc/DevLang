#ifndef _WIN32
#define _POSIX_C_SOURCE 200809L
#endif
#include "dev_runtime.h"
#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
uint64_t dvr_now_ns(void) {
    LARGE_INTEGER counter, frequency;
    if (!QueryPerformanceFrequency(&frequency) || !QueryPerformanceCounter(&counter)) return 0;
    uint64_t ticks = (uint64_t)counter.QuadPart, hz = (uint64_t)frequency.QuadPart;
    return (ticks / hz) * UINT64_C(1000000000) + (ticks % hz) * UINT64_C(1000000000) / hz;
}
int32_t dvr_sleep_ms(uint64_t ms) {
    while (ms > UINT32_C(0xfffffffe)) { Sleep(0xfffffffe); ms -= UINT32_C(0xfffffffe); }
    Sleep((DWORD)ms); return 1;
}
#else
#include <errno.h>
#include <time.h>
uint64_t dvr_now_ns(void) {
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now)) return 0;
    return (uint64_t)now.tv_sec * UINT64_C(1000000000) + (uint64_t)now.tv_nsec;
}
int32_t dvr_sleep_ms(uint64_t ms) {
    if (ms / 1000 > INT64_MAX) return 0;
    struct timespec delay;
    delay.tv_sec = (time_t)(ms / 1000);
    if ((uint64_t)delay.tv_sec != ms / 1000) return 0;
    delay.tv_nsec = (long)((ms % 1000) * 1000000);
    while (nanosleep(&delay, &delay)) { if (errno != EINTR) return 0; }
    return 1;
}
#endif
