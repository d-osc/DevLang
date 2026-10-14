#include <stdio.h>
#include "devmath.h"

int main(void) {
    printf("%d\n", (int)devmath_add(20, 22));
    printf("%.1f\n", devmath_scale(2.5, 5.0));
    printf("%s\n", devmath_positive(7) ? "true" : "false");
    puts(devmath_name());
    return 0;
}
