## 进程管理相关

### #define SYS_clone 220

* 功能：创建一个子进程；
* 输入：
	- flags: 创建的标志，如SIGCHLD；
	- stack: 指定新进程的栈，可为0；
	- ptid: 父线程ID；
	- tls: TLS线程本地存储描述符；
	- ctid: 子线程ID；
* 返回值：成功则返回子进程的线程ID，失败返回-1；

```c
pid_t ret = syscall(SYS_clone, flags, stack, ptid, tls, ctid)
```

### #define SYS_execve 221

* 功能：执行一个指定的程序；
* 输入：
	- path: 待执行程序路径名称，
	- argv: 程序的参数， 
	- envp: 环境变量的数组指针
* 返回值：成功不返回，失败返回-1；

```c
const char *path, char *const argv[], char *const envp[];
int ret = syscall(SYS_execve, path, argv, envp);
```

### #define SYS_wait4 260

* 功能：等待进程改变状态;
* 输入：
	- pid: 指定进程ID，可为-1等待任何子进程；
	- status: 接收状态的指针；
	- options: 选项：WNOHANG，WUNTRACED，WCONTINUED；
* 返回值：成功则返回进程ID；如果指定了WNOHANG，且进程还未改变状态，直接返回0；失败则返回-1；

```c
pid_t pid, int *status, int options;
pid_t ret = syscall(SYS_wait4, pid, status, options);
```

### #define SYS_exit 93

* 功能：触发进程终止，无返回值；
* 输入：终止状态值；
* 返回值：无返回值；

```c
int ec;
syscall(SYS_exit, ec);
```

### #define SYS_getppid 173

* 功能：获取父进程ID；
* 输入：系统调用ID；
* 返回值：成功返回父进程ID；

```c
pid_t ret = syscall(SYS_getppid);
```

### #define SYS_getpid 172

* 功能：获取进程ID；
* 输入：系统调用ID；
* 返回值：成功返回进程ID；

```c
pid_t ret = syscall(SYS_getpid);
```
