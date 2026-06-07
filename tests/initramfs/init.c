#include <stdio.h>
#include <unistd.h>

int main(void) {
    printf("Hello from Linux on rvsim!\n");
    // Keep init alive — the kernel panics if init exits.
    for (;;)
        sleep(1);
    return 0;
}
