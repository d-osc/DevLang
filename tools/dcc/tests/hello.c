// hello.c - basic smoke test
int main(void) {
    puts("Hello from dcc! This exe was built without clang or gcc.\n");
    putchar(72); /* H */
    putchar(101);
    putchar(10);
    print_int(42);
    putchar(10);
    return 0;
}
