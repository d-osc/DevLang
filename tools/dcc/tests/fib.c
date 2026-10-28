// fib.c - recursion test
int fib(int n) {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

int main() {
    print_int(fib(24));
    putchar(10);
    return 0;
}
