## 其他

### #define SYS_times 153

> done

* 功能：获取进程时间；
* 输入：tms结构体指针，用于获取保存当前进程的运行时间数据；
* 返回值：成功返回已经过去的滴答数，失败返回-1;

```c
struct tms *tms;
clock_t ret = syscall(SYS_times, tms);
```

### #define SYS_uname 160

* 功能：打印系统信息；
* 输入：utsname结构体指针用于获得系统信息数据；
* 返回值：成功返回0，失败返回-1;

```c
struct utsname *uts;
int ret = syscall(SYS_uname, uts);
```

### #define SYS_sched_yield 124

> done

* 功能：让出调度器；
* 输入：系统调用ID；
* 返回值：成功返回0，失败返回-1;

```c
int ret = syscall(SYS_sched_yield);
```

### #define SYS_gettimeofday 169

* 功能：获取时间；
* 输入： timespec结构体指针用于获得时间值；
* 返回值：成功返回0，失败返回-1;

```c
struct timespec *ts;
int ret = syscall(SYS_gettimeofday, ts, 0);
```

### #define SYS_nanosleep 101

* 功能：执行线程睡眠，sleep()库函数基于此系统调用；
* 输入：睡眠的时间间隔；

```c
struct timespec {
	time_t tv_sec;        /* 秒 */
	long   tv_nsec;       /* 纳秒, 范围在0~999999999 */
};
```

* 返回值：成功返回0，失败返回-1;

```c
const struct timespec *req, struct timespec *rem;
int ret = syscall(SYS_nanosleep, req, rem);
```
