#include "init_proc.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "unistd.h"

void run_existed_tests() {
    test_openat();
    test_yield();
}

void run_with_arg(char* str, char* argv[], char* environ[]) {
    execve(str, argv, environ);
}

void run(char* str) {
    int cpid;
    cpid = fork();
    assert(cpid != -1);
    if (cpid <= 0) {
        run_with_arg(str, NULL, NULL);
        exit(0);
    }
}

int main(void) {
    printf("[user] This is init_proc.\n");
    run("test_echo");
    run("mmap");
}
