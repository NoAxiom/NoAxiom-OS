#include "init_proc.h"
#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "test_points.h"
#include "unistd.h"

void run_existed_tests()
{
    test_openat();
    test_yield();
}

void run_with_arg(char* str, char* argv[], char* environ[])
{
    execve(str, argv, environ);
}

int cpid_list[100] = { 0 };
int run(char* str)
{
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

int failed_test[100] = { 0 };
int main(void)
{
    int test_num = sizeof(test_points) / sizeof(char*);
    int cnt = 0, i;
    int time_in = get_time(), time_out;
    // start test
    printf("========== [ init_proc ] start test! num: %d ==========\n",
        test_num);
    for (i = 0; i < test_num; i++) {
        int tmp_res = run(test_points[i]);
        failed_test[i] = tmp_res != 0;
        cnt += tmp_res;
    }
    // test done!
    time_out = get_time();
    printf("========== [ init_proc ] all tests are done!! ==========\n");
    printf("========== [ init_proc ] passed points: %d/%d ==========\n", cnt,
        test_num);
    printf("test cost time: %d\n", time_out - time_in);

    for (i = 0; i < test_num; i++) {
        if (failed_test[i] == 0) {
            printf("[init_proc] test %s FAILED!!!\n", test_points[i]);
        }
    }
}
