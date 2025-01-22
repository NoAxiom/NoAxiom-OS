#!/bin/bash
elf_path="${ELF_PATH}"
ERROR="\e[31m"
WARN="\e[33m"
NORMAL="\e[32m"
RESET="\e[0m"

img_dir="fs_img"

# 删除旧的 fs 目录和 fs.img 文件
rm -f fs.img

# 创建新的 fs.img 文件并格式化为 FAT32
dd if=/dev/zero of=fs.img bs=3M count=16
mkfs.fat -F 32 fs.img

# 创建并挂载 fs 目录
mkdir $img_dir
sudo mount -o loop fs.img $img_dir

echo -e $NORMAL"Making file system image: "$RESET

# 复制 ELF 文件到 fs 目录，排除 kernel
find $elf_path -maxdepth 1 -type f -exec file {} \; | grep "\<ELF\>" | awk -F ':' '{print $1}' | while read line
do
    if [[ $line != *"kernel"* ]]; then
        sudo cp $line $img_dir/
        echo -e $NORMAL "\t load: $line"$RESET
    fi
done

echo -e $NORMAL"Making file system completed. "$RESET

sleep 0.1 # what?

# 卸载 fs 目录
sudo umount $img_dir
rmdir $img_dir
