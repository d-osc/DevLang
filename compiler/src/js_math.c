/* DEV_MATH_RUNTIME: hosted IEEE Math namespace. */
#include <math.h>
#include <stdatomic.h>
#include <string.h>
#include <time.h>
static double dev_math_f16(double x) {
  if (!isfinite(x) || x == 0)
    return x;
  double a = fabs(x);
  if (a >= 65520)
    return copysign(INFINITY, x);
  int e;
  frexp(a, &e);
  double step = a < 0x1p-14 ? 0x1p-24 : ldexp(1.0, e - 11);
  double q = a / step, n = floor(q), f = q - n;
  if (f > 0.5 || (f == 0.5 && fmod(n, 2.0) != 0))
    n += 1;
  return copysign(n * step, x);
}
static double dev_math_round(double x) {
  if (!isfinite(x) || x == 0)
    return x;
  if (x >= -0.5 && x < 0)
    return -0.0;
  double n = floor(x);
  return x - n < 0.5 ? n : n + 1;
}
static double dev_math_clz32(double x) {
  double r = isfinite(x) ? fmod(trunc(x), 4294967296.0) : 0;
  if (r < 0)
    r += 4294967296.0;
  uint32_t n = (uint32_t)r;
  unsigned count = 32;
  if (n) {
    count = 0;
    for (uint32_t mask = UINT32_C(0x80000000); !(n & mask); mask >>= 1)
      ++count;
  }
  return count;
}
static double dev_math_random(void) {
  static atomic_uint_fast64_t state = ATOMIC_VAR_INIT(0);
  uint_fast64_t old = atomic_load_explicit(&state, memory_order_relaxed);
  if (!old) {
    struct timespec t;
    timespec_get(&t, TIME_UTC);
    uint_fast64_t seed =
        (uint64_t)t.tv_sec ^ ((uint64_t)t.tv_nsec << 32) ^ (uintptr_t)&state;
    if (!seed)
      seed = 1;
    atomic_compare_exchange_strong_explicit(
        &state, &old, seed, memory_order_relaxed, memory_order_relaxed);
  }
  uint64_t z = atomic_fetch_add_explicit(&state, UINT64_C(0x9e3779b97f4a7c15),
                                         memory_order_relaxed) +
               UINT64_C(0x9e3779b97f4a7c15);
  z = (z ^ (z >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
  z = (z ^ (z >> 27)) * UINT64_C(0x94d049bb133111eb);
  z ^= z >> 31;
  return (double)(z >> 11) / 9007199254740992.0;
}
static double dev_math_call(const char *name, size_t count,
                            const double *args) {
  double x = count ? args[0] : 0, y = count > 1 ? args[1] : 0;
  if (!strcmp(name, "random"))
    return dev_math_random();
  if (!strcmp(name, "f16round"))
    return dev_math_f16(x);
  if (!strcmp(name, "fround"))
    return (double)(float)x;
  if (!strcmp(name, "round"))
    return dev_math_round(x);
  if (!strcmp(name, "clz32"))
    return dev_math_clz32(x);
  if (!strcmp(name, "sign"))
    return isnan(x) || x == 0 ? x : x > 0 ? 1 : -1;
  if (!strcmp(name, "atan2"))
    return atan2(x, y);
  if (!strcmp(name, "pow"))
    return isnan(y)                   ? NAN
           : y == 0                   ? 1
           : isnan(x)                 ? NAN
           : fabs(x) == 1 && isinf(y) ? NAN
                                      : pow(x, y);
  if (!strcmp(name, "min") || !strcmp(name, "max")) {
    bool min = !strcmp(name, "min");
    double n = min ? INFINITY : -INFINITY;
    for (size_t i = 0; i < count; i++) {
      x = args[i];
      if (isnan(x))
        return NAN;
      if ((min && x < n) || (!min && x > n) ||
          (x == 0 && n == 0 && !!signbit(x) == min))
        n = x;
    }
    return n;
  }
  if (!strcmp(name, "abs"))
    return fabs(x);
  if (!strcmp(name, "acos"))
    return acos(x);
  if (!strcmp(name, "acosh"))
    return acosh(x);
  if (!strcmp(name, "asin"))
    return asin(x);
  if (!strcmp(name, "asinh"))
    return asinh(x);
  if (!strcmp(name, "atan"))
    return atan(x);
  if (!strcmp(name, "atanh"))
    return atanh(x);
  if (!strcmp(name, "cbrt"))
    return cbrt(x);
  if (!strcmp(name, "ceil"))
    return ceil(x);
  if (!strcmp(name, "cos"))
    return cos(x);
  if (!strcmp(name, "cosh"))
    return cosh(x);
  if (!strcmp(name, "exp"))
    return exp(x);
  if (!strcmp(name, "expm1"))
    return expm1(x);
  if (!strcmp(name, "floor"))
    return floor(x);
  if (!strcmp(name, "log"))
    return log(x);
  if (!strcmp(name, "log10"))
    return log10(x);
  if (!strcmp(name, "log1p"))
    return log1p(x);
  if (!strcmp(name, "log2"))
    return log2(x);
  if (!strcmp(name, "sin"))
    return sin(x);
  if (!strcmp(name, "sinh"))
    return sinh(x);
  if (!strcmp(name, "sqrt"))
    return sqrt(x);
  if (!strcmp(name, "tan"))
    return tan(x);
  if (!strcmp(name, "tanh"))
    return tanh(x);
  if (!strcmp(name, "trunc"))
    return trunc(x);
  return NAN;
}
