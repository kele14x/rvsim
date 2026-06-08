#include <stdio.h>
#include <unistd.h>

int main(void) {
    printf("Hello from Linux on rvsim!\n");
    printf("Type something (echo test):\n");
    char buf[1];
    while (read(STDIN_FILENO, buf, 1) > 0) {
        write(STDOUT_FILENO, buf, 1);
    }
    return 0;
}
