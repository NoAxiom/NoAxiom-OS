#include "init_proc.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "test_points.h"
#include "unistd.h"

void run_existed_tests() {
    test_openat();
    test_yield();
}

void run_with_arg(char* str, char* argv[], char* environ[]) {
    execve(str, argv, environ);
}

int cpid_list[100] = {0};
int run(char* str) {
    int waitret, wstatus, cpid;
    cpid = fork();
    assert(cpid != -1);
    if (cpid <= 0) {
        run_with_arg(str, NULL, NULL);
        exit(0);
    } else {
        waitret = wait(&wstatus);
        if (waitret == cpid && wstatus == 0) {
            printf("exit OK.\n");
            return 1;
        } else {
            printf("exit ERR.\n");
            return 0;
        }
    }
    return 0;
}

int main(void) {
    int test_num = sizeof(test_points) / sizeof(char*);
    int cnt = 0, i;
    // start test
    printf("========== [ init_proc ] start test! num: %d ==========\n",
           test_num);
    for (i = 0; i < test_num; i++) {
        cnt += run(test_points[i]);
    }
    // test done!
    printf("========== [ init_proc ] all tests are done ==========\n");
    printf("========== [ init_proc ] passed points: %d/%d ==========\n", cnt,
           test_num);
}
