#include <stdint.h>
#if defined(_WIN32)
#define API extern "C" __declspec(dllexport)
#else
#define API extern "C" __attribute__((visibility("default")))
#endif

API int32_t foreign_add(int32_t a, int32_t b) { return a + b; }
API double foreign_scale(double value, double factor) { return value * factor; }
API bool foreign_positive(int32_t value) { return value > 0; }
