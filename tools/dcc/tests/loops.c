// loops.c - for/while/break/continue/compound-assign
int main() {
    int total = 0;
    for (int i = 1; i <= 100; i++) {
        total += i;
    }
    print_int(total); // 5050
    putchar(10);

    int w = 0;
    int hits = 0;
    while (1) {
        w++;
        if (w % 2 == 0) { continue; }
        if (w > 20) { break; }
        hits++;
    }
    print_int(hits); // 10 odd numbers 1..19
    putchar(10);

    int a = 10;
    int b = 3;
    a -= b;      // 7
    a *= 4;      // 28
    a /= 3;      // 9
    a %= 5;      // 4
    print_int(a);
    putchar(10);
    return 0;
}
