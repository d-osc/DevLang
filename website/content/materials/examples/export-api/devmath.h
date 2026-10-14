#ifndef DEVMATH_H
#define DEVMATH_H
#include <stdint.h>
#include <stdbool.h>
#ifdef __cplusplus
extern "C" {
#endif
int32_t devmath_add(int32_t a, int32_t b);
double devmath_scale(double value, double factor);
bool devmath_positive(int32_t value);
const char *devmath_name(void);
#ifdef __cplusplus
}
#endif
#endif
