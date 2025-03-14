#!/bin/bash
elf_path="${ELF_PATH}"
ERROR="\e[31m"
WARN="\e[33m"
NORMAL="\e[32m"
RESET="\e[0m"

img_dir="fs_img"

# 删除旧的 fs 目录和 fs.img 文件
rm -f fs.img

# 创建新的 fs.img 文件并格式化为 FAT32/ext4 文件系统
dd if=/dev/zero of=fs.img bs=1M count=2048
mkfs.ext4 fs.img

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

# 复制 mnt 文件夹到 fs 目录
if [ -d "$elf_path/mnt" ]; then
    sudo cp -r "$elf_path/mnt" "$img_dir/"
    echo -e $NORMAL "\t load: $elf_path/mnt"$RESET
else
    echo -e $WARN "\t Warning: mnt folder not found in $elf_path"$RESET
fi

# 复制 text.txt 文件到 fs 目录
if [ -f "$elf_path/text.txt" ]; then
    sudo cp "$elf_path/text.txt" "$img_dir/"
    echo -e $NORMAL "\t load: $elf_path/text.txt"$RESET
else
    echo -e $WARN "\t Warning: text.txt not found in $elf_path"$RESET
fi

echo -e $NORMAL"Making file system completed. "$RESET

sleep 0.1 # what?

# 卸载 fs 目录
sudo umount $img_dir
rmdir $img_dir
