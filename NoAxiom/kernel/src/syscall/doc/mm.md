## 内存管理相关

### #define SYS_brk 214

* 功能：修改数据段的大小；
* 输入：指定待修改的地址；
* 返回值：成功返回0，失败返回-1;

```c
uintptr_t brk;
uintptr_t ret = syscall(SYS_brk, brk);
```

### #define SYS_munmap 215

* 功能：将文件或设备取消映射到内存中；
* 输入：映射的指定地址及区间；
* 返回值：成功返回0，失败返回-1;

```c
void *start, size_t len
int ret = syscall(SYS_munmap, start, len);
```

### #define SYS_mmap 222

* 功能：将文件或设备映射到内存中；
* 输入：
	- start: 映射起始位置，
	- len: 长度，
	- prot: 映射的内存保护方式，可取：PROT_EXEC, PROT_READ, PROT_WRITE, PROT_NONE
	- flags: 映射是否与其他进程共享的标志，
	- fd: 文件句柄，
	- off: 文件偏移量；
* 返回值：成功返回已映射区域的指针，失败返回-1;

```c
void *start, size_t len, int prot, int flags, int fd, off_t off
long ret = syscall(SYS_mmap, start, len, prot, flags, fd, off);
```
