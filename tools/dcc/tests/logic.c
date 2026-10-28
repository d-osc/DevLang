// logic.c - short-circuit, comparisons, unary, else-if, negative numbers
int classify(int n) {
    if (n < 0) {
        return -1;
    } else if (n == 0) {
        return 0;
    } else {
        return 1;
    }
}

int side(int v) { return 1; }

int main() {
    print_int(classify(-5));
    print_int(classify(0));
    print_int(classify(17));
    putchar(10);

    int x = 0;
    int r = 0;
    // (0 && ...) must not be evaluated; || short-circuits true
    if (x != 0 && side(9)) { r += 100; }
    if (x == 0 || side(9)) { r += 1; }
    if (!x) { r += 10; }
    if (!(x == 0)) { r += 1000; }
    print_int(r); // 11
    putchar(10);

    int big = -12345678;
    print_int(big);
    putchar(10);
    print_int(7 / 2);
    print_int(7 % 3);
    print_int(-7 / 2);
    print_int(-7 % 3);
    putchar(10);
    return 0;
}
