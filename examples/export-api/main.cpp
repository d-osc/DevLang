#include <stdio.h>
#include "devmath.h"

int main() {
    const auto answer = devmath_add(20, 22);
    const auto scaled = devmath_scale(2.5, 5.0);
    const auto positive = devmath_positive(7);
    printf("%d\n%.1f\n%s\n%s\n", (int)answer, scaled,
           positive ? "true" : "false", devmath_name());
}
