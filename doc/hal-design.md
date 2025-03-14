# NoAxiom HAL 设计

其实主要是因为开发的差不多的时候才进行hal的接入, 感觉不如自己写

## 设计主要思路

我们打算通过rust trait实现类似头文件的功能.

然后通过对于rv64和la64分别实现trait的方式, 控制功能的完整性

这个思路来自fs的vfs层, 通过trait实现硬件层与软件层的抽象解耦.

## 主要组件

### full arch

首先定义一个`FullVirtArch`, 它被设定为实现了所有trait

然后根据需要的功能, 去实现不同的trait即可.
具体的trait不再赘述

### type def

我们尝试将不同的 interrupt / exception 统一为一致的接口, 接入到我们的系统当中.

目前对于这一方面, 尝试将la的例外中断号映射到riscv的架构当中.

wip

### mm

内存管理比较麻烦

rv的实现交给了硬件, 但la需要软件实现tlb充填, 因此需要额外的asm段进行维护

目前打算:

1. 进入trap后, 首先根据`VirtTrap::VirtException`判断例外类型

2. 当检测到pagefault的时候, 首先进行tlb充填 (这里是否需要维护是否充填成功???)

3. 再进入软件层进行缺页维护
