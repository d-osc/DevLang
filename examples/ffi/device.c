#include <stdint.h>

#ifdef _WIN32
#define DEVICE_API __declspec(dllexport)
#else
#define DEVICE_API
#endif

DEVICE_API int32_t device_add(int32_t a, int32_t b) {
    return a + b;
}
