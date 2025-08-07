## ahci

### 硬件

ls2k1000la板卡上最主要的存储设备是一块sata固态硬盘，处理器通过ahci控制器进行控制，寄存器物理基地址是`0x400e0000`，该ahci控制器支持SATA 1.5Gbps和SATA 2代3Gbps的传输，向下兼容Serial ATA 2.6和AHCI 1.1规范

ahci控制器通过内存dma和处理器进行数据交换，dma配置支持64位地址；控制器支持中断控制，驱动中初始化了ahci控制器中断，但并没有使用中断

ahci控制器最多支持32个端口，板卡上固定只开启最低位的一个端口，对应着唯一的一块固态硬盘

### 驱动

驱动代码参考自linux和uboot

驱动主要依靠结构体`struct ahci_device`，它代表着ahci控制器，结构体`struct ahci_blk_dev`代表着sata硬盘

由于板卡上的ahci只开启1个端口/1个固态硬盘位，因此在`sturct ahci_device`中仅声明了1个硬盘块设备结构体`struct ahci_blk_dev`

函数`ahci_init`用于初始化ahci控制器和sata固态硬盘，按照ahci控制器->端口->sata硬盘的顺序，初始化完成后会打印出ahci控制器和sata硬盘的基本信息

函数`ahci_sata_write_common`和`ahci_sata_read_common`是sata硬盘的读写函数，传入读写开始块偏移blknr、读写块数量blkcnt，以及写入/读取数据的buffer

此外在驱动中，sata硬盘块大小固定为512，默认应当支持lba48

代码中需要实现`platform.rs`或`ahci_platform.h`中列举的一些函数，修改或替换`printf`函数的实现以及具体调用
