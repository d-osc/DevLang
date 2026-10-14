#include "../devmath.h"
#if defined(_WIN32)
#define API __declspec(dllexport)
#else
#define API __attribute__((visibility("default")))
#endif
API int32_t bridge_add(int32_t a, int32_t b) { return devmath_add(a, b); }
API double bridge_scale(double value, double factor) { return devmath_scale(value, factor); }
API bool bridge_positive(int32_t value) { return devmath_positive(value); }
API const char *bridge_name(void) { return devmath_name(); }
