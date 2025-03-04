#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "unistd.h"

void test_brk() {
    TEST_START(__func__);
    intptr_t cur_pos, alloc_pos, alloc_pos_1;

    cur_pos = brk(0);
    printf("Before alloc,heap pos: %d\n", cur_pos);
    brk(cur_pos + 64);
    alloc_pos = brk(0);
    printf("After alloc,heap pos: %d\n", alloc_pos);
    brk(alloc_pos + 64);
    alloc_pos_1 = brk(0);
    printf("Alloc again,heap pos: %d\n", alloc_pos_1);
    TEST_END(__func__);
}

static char buffer[30];
void test_chdir(void) {
    TEST_START(__func__);
    mkdir("test_chdir", 0666);
    int ret = chdir("test_chdir");
    printf("chdir ret: %d\n", ret);
    assert(ret == 0);
    getcwd(buffer, 30);
    printf("  current working dir : %s\n", buffer);
    TEST_END(__func__);
}

size_t stack[1024] = {0};
static int child_pid;
static int child_func(void) {
    printf("  Child says successfully!\n");
    return 0;
}
void test_clone(void) {
    TEST_START(__func__);
    int wstatus;
    child_pid = clone(child_func, NULL, stack, 1024, SIGCHLD);
    assert(child_pid != -1);
    if (child_pid == 0) {
        exit(0);
    } else {
        if (wait(&wstatus) == child_pid)
            printf("clone process successfully.\npid:%d\n", child_pid);
        else
            printf("clone process error.\n");
    }

    TEST_END(__func__);
}

void test_close(void) {
    TEST_START(__func__);
    int fd = open("test_close.txt", O_CREATE | O_RDWR);
    // assert(fd > 0);
    const char* str = "  close error.\n";
    int str_len = strlen(str);
    // assert(write(fd, str, str_len) == str_len);
    write(fd, str, str_len);
    int rt = close(fd);
    assert(rt == 0);
    printf("  close %d success.\n", fd);

    TEST_END(__func__);
}

void test_dup() {
    TEST_START(__func__);
    int fd = dup(STDOUT);
    assert(fd >= 0);
    printf("  new fd is %d.\n", fd);
    TEST_END(__func__);
}

void test_dup2() {
    TEST_START(__func__);
    int fd = dup2(STDOUT, 100);
    assert(fd != -1);
    const char* str = "  from fd 100\n";
    write(100, str, strlen(str));
    TEST_END(__func__);
}

void test_execve(void) {
    TEST_START(__func__);
    char* newargv[] = {"test_echo", NULL};
    char* newenviron[] = {NULL};
    execve("test_echo", newargv, newenviron);
    printf("  execve error.\n");
    // TEST_END(__func__);
}

void test_exit(void) {
    TEST_START(__func__);
    int cpid, waitret, wstatus;
    cpid = fork();
    assert(cpid != -1);
    if (cpid == 0) {
        exit(0);
    } else {
        waitret = wait(&wstatus);
        if (waitret == cpid)
            printf("exit OK.\n");
        else
            printf("exit ERR.\n");
    }
    TEST_END(__func__);
}

void test_fork(void) {
    TEST_START(__func__);
    int cpid, wstatus;
    cpid = fork();
    assert(cpid != -1);

    if (cpid > 0) {
        wait(&wstatus);
        printf("  parent process. wstatus:%d\n", wstatus);
    } else {
        printf("  child process.\n");
        exit(0);
    }
    TEST_END(__func__);
}

#define AT_FDCWD (-100)  // 相对路径
// Stat *kst;
static struct kstat kst;
void test_fstat() {
    TEST_START(__func__);
    int fd = open("./text.txt", 0);
    int ret = fstat(fd, &kst);
    printf("fstat ret: %d\n", ret);
    assert(ret >= 0);

    printf(
        "fstat: dev: %d, inode: %d, mode: %d, nlink: %d, size: %d, atime: %d, "
        "mtime: %d, ctime: %d\n",
        kst.st_dev, kst.st_ino, kst.st_mode, kst.st_nlink, kst.st_size,
        kst.st_atime_sec, kst.st_mtime_sec, kst.st_ctime_sec);

    TEST_END(__func__);
}

void test_getcwd(void) {
    TEST_START(__func__);
    char* cwd = NULL;
    char buf[128] = {0};
    cwd = getcwd(buf, 128);
    if (cwd != NULL)
        printf("getcwd: %s successfully!\n", buf);
    else
        printf("getcwd ERROR.\n");
    TEST_END(__func__);
}

char buf[512];
void test_getdents(void) {
    TEST_START(__func__);
    int fd, nread;
    struct linux_dirent64* dirp64;
    dirp64 = buf;
    // fd = open(".", O_DIRECTORY);
    fd = open(".", O_RDONLY);
    printf("open fd:%d\n", fd);

    nread = getdents(fd, dirp64, 512);
    printf("getdents fd:%d\n", nread);
    assert(nread != -1);
    printf("getdents success.\n%s\n", dirp64->d_name);

    /*
    for(int bpos = 0; bpos < nread;){
        d = (struct dirent *)(buf + bpos);
        printf(  "%s\t", d->d_name);
        bpos += d->d_reclen;
    }
    */

    printf("\n");
    close(fd);
    TEST_END(__func__);
}

int test_getpid() {
    TEST_START(__func__);
    int pid = getpid();
    assert(pid >= 0);
    printf("getpid success.\npid = %d\n", pid);
    TEST_END(__func__);
}

int test_getppid() {
    TEST_START(__func__);
    pid_t ppid = getppid();
    if (ppid > 0)
        printf("  getppid success. ppid : %d\n", ppid);
    else
        printf("  getppid error.\n");
    TEST_END(__func__);
}

void test_gettimeofday() {
    TEST_START(__func__);
    int test_ret1 = get_time();
    volatile int i = 12500000;  // qemu时钟频率12500000
    while (i > 0)
        i--;
    int test_ret2 = get_time();
    if (test_ret1 > 0 && test_ret2 > 0) {
        printf("gettimeofday success.\n");
        printf("start:%d, end:%d\n", test_ret1, test_ret2);
        printf("interval: %d\n", test_ret2 - test_ret1);
    } else {
        printf("gettimeofday error.\n");
    }
    TEST_END(__func__);
}

void test_mkdir(void) {
    TEST_START(__func__);
    int rt, fd;

    rt = mkdir("test_mkdir", 0666);
    printf("mkdir ret: %d\n", rt);
    assert(rt != -1);
    fd = open("test_mkdir", O_RDONLY | O_DIRECTORY);
    if (fd > 0) {
        printf("  mkdir success.\n");
        close(fd);
    } else
        printf("  mkdir error.\n");
    TEST_END(__func__);
}

void test_mmap(void) {
    TEST_START(__func__);
    char* array;
    const char* str = "  Hello, mmap successfully!";
    int fd;

    fd = open("test_mmap.txt", O_RDWR | O_CREATE);
    write(fd, str, strlen(str));
    fstat(fd, &kst);
    printf("file len: %d\n", kst.st_size);
    array = mmap(NULL, kst.st_size, PROT_WRITE | PROT_READ,
                 MAP_FILE | MAP_SHARED, fd, 0);
    // printf("return array: %x\n", array);

    if (array == MAP_FAILED) {
        printf("mmap error.\n");
    } else {
        printf("mmap content: %s\n", array);
        // printf("%s\n", str);

        munmap(array, kst.st_size);
    }

    close(fd);

    TEST_END(__func__);
}

static char mntpoint[64] = "./mnt";
static char device[64] = "/dev/vda2";
static const char* fs_type = "vfat";

void test_mount() {
    TEST_START(__func__);

    printf("Mounting dev:%s to %s\n", device, mntpoint);
    int ret = mount(device, mntpoint, fs_type, 0, NULL);
    printf("mount return: %d\n", ret);
    assert(ret == 0);

    if (ret == 0) {
        printf("mount successfully\n");
        ret = umount(mntpoint);
        printf("umount return: %d\n", ret);
    }

    TEST_END(__func__);
}

void test_munmap(void) {
    TEST_START(__func__);
    char* array;
    const char* str = "  Hello, mmap successfully!";
    int fd;

    fd = open("test_mmap.txt", O_RDWR | O_CREATE);
    write(fd, str, strlen(str));
    fstat(fd, &kst);
    printf("file len: %d\n", kst.st_size);
    array = mmap(NULL, kst.st_size, PROT_WRITE | PROT_READ,
                 MAP_FILE | MAP_SHARED, fd, 0);
    // printf("return array: %x\n", array);

    if (array == MAP_FAILED) {
        printf("mmap error.\n");
    } else {
        // printf("mmap content: %s\n", array);

        int ret = munmap(array, kst.st_size);
        printf("munmap return: %d\n", ret);
        assert(ret == 0);

        if (ret == 0)
            printf("munmap successfully!\n");
    }
    close(fd);

    TEST_END(__func__);
}

void test_open() {
    TEST_START(__func__);
    // O_RDONLY = 0, O_WRONLY = 1
    int fd = open("./text.txt", 0);
    assert(fd >= 0);
    char buf[256];
    int size = read(fd, buf, 256);
    if (size < 0) {
        size = 0;
    }
    write(STDOUT, buf, size);
    close(fd);
    TEST_END(__func__);
}

void test_openat(void) {
    TEST_START(__func__);
    // int fd_dir = open(".", O_RDONLY | O_CREATE);
    int fd_dir = open("./mnt", O_DIRECTORY);
    printf("open dir fd: %d\n", fd_dir);
    int fd = openat(fd_dir, "test_openat.txt", O_CREATE | O_RDWR);
    printf("openat fd: %d\n", fd);
    assert(fd > 0);
    printf("openat success.\n");

    /*(
    char buf[256] = "openat text file";
    write(fd, buf, strlen(buf));
    int size = read(fd, buf, 256);
    if (size > 0) printf("  openat success.\n");
    else printf("  openat error.\n");
    */
    close(fd);

    TEST_END(__func__);
}

static int fd[2];

void test_pipe(void) {
    TEST_START(__func__);
    int cpid;
    char buf[128] = {0};
    int ret = pipe(fd);
    assert(ret != -1);
    const char* data = "  Write to pipe successfully.\n";
    cpid = fork();
    printf("cpid: %d\n", cpid);
    if (cpid > 0) {
        close(fd[1]);
        while (read(fd[0], buf, 1) > 0)
            write(STDOUT, buf, 1);
        write(STDOUT, "\n", 1);
        close(fd[0]);
        wait(NULL);
    } else {
        close(fd[0]);
        write(fd[1], data, strlen(data));
        close(fd[1]);
        exit(0);
    }
    TEST_END(__func__);
}

void test_read() {
    TEST_START(__func__);
    int fd = open("./text.txt", 0);
    char buf[256];
    int size = read(fd, buf, 256);
    assert(size >= 0);

    write(STDOUT, buf, size);
    close(fd);
    TEST_END(__func__);
}

void test_sleep() {
    TEST_START(__func__);

    int time1 = get_time();
    assert(time1 >= 0);
    int ret = sleep(1);
    assert(ret == 0);
    int time2 = get_time();
    assert(time2 >= 0);

    if (time2 - time1 >= 1) {
        printf("sleep success.\n");
    } else {
        printf("sleep error.\n");
    }
    TEST_END(__func__);
}

// void test_echo(void);
struct tms {
    long tms_utime;
    long tms_stime;
    long tms_cutime;
    long tms_cstime;
};

struct tms mytimes;

void test_times() {
    TEST_START(__func__);

    int test_ret = times(&mytimes);
    assert(test_ret >= 0);

    printf(
        "mytimes success\n{tms_utime:%d, tms_stime:%d, tms_cutime:%d, "
        "tms_cstime:%d}\n",
        mytimes.tms_utime, mytimes.tms_stime, mytimes.tms_cutime,
        mytimes.tms_cstime);
    TEST_END(__func__);
}

void test_umount() {
    TEST_START(__func__);

    printf("Mounting dev:%s to %s\n", device, mntpoint);
    int ret = mount(device, mntpoint, fs_type, 0, NULL);
    printf("mount return: %d\n", ret);

    if (ret == 0) {
        ret = umount(mntpoint);
        assert(ret == 0);
        printf("umount success.\nreturn: %d\n", ret);
    }

    TEST_END(__func__);
}

struct utsname {
    char sysname[65];
    char nodename[65];
    char release[65];
    char version[65];
    char machine[65];
    char domainname[65];
};

struct utsname un;

void test_uname() {
    TEST_START(__func__);
    int test_ret = uname(&un);
    assert(test_ret >= 0);

    printf("Uname: %s %s %s %s %s %s\n", un.sysname, un.nodename, un.release,
           un.version, un.machine, un.domainname);

    TEST_END(__func__);
}

int test_unlink() {
    TEST_START(__func__);

    char* fname = "./test_unlink";
    int fd, ret;

    fd = open(fname, O_CREATE | O_WRONLY);
    assert(fd > 0);
    close(fd);

    // unlink test
    ret = unlink(fname);
    assert(ret == 0);
    fd = open(fname, O_RDONLY);
    if (fd < 0) {
        printf("  unlink success!\n");
    } else {
        printf("  unlink error!\n");
        close(fd);
    }
    // It's Ok if you don't delete the inode and data blocks.

    TEST_END(__func__);
}

void test_wait(void) {
    TEST_START(__func__);
    int cpid, wstatus;
    cpid = fork();
    if (cpid == 0) {
        printf("This is child process\n");
        exit(0);
    } else {
        pid_t ret = wait(&wstatus);
        assert(ret != -1);
        if (ret == cpid)
            printf("wait child success.\nwstatus: %d\n", wstatus);
        else
            printf("wait child error.\n");
    }
    TEST_END(__func__);
}

void test_waitpid(void) {
    TEST_START(__func__);
    int i = 1000;
    int cpid, wstatus;
    cpid = fork();
    assert(cpid != -1);
    if (cpid == 0) {
        while (i--)
            ;
        sched_yield();
        printf("This is child process\n");
        exit(3);
    } else {
        pid_t ret = waitpid(cpid, &wstatus, 0);
        assert(ret != -1);
        if (ret == cpid && WEXITSTATUS(wstatus) == 3)
            printf("waitpid successfully.\nwstatus: %x\n",
                   WEXITSTATUS(wstatus));
        else
            printf("waitpid error.\n");
    }
    TEST_END(__func__);
}

void test_write() {
    TEST_START(__func__);
    const char* str = "Hello operating system contest.\n";
    int str_len = strlen(str);
    assert(write(STDOUT, str, str_len) == str_len);
    TEST_END(__func__);
}

int test_yield() {
    TEST_START(__func__);

    for (int i = 0; i < 3; ++i) {
        if (fork() == 0) {
            for (int j = 0; j < 5; ++j) {
                sched_yield();
                printf("  I am child process: %d. iteration %d.\n", getpid(),
                       i);
            }
            exit(0);
        }
    }
    wait(NULL);
    wait(NULL);
    wait(NULL);
    TEST_END(__func__);
}