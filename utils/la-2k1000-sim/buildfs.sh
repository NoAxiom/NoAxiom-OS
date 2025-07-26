SUDO=$(if [ $(whoami) = "root" ];then echo -n "";else echo -n "sudo";fi)
U_FAT32_DIR="../easy-fs-fuse"
U_FAT32=$1
BLK_SZ="512"
TARGET=riscv64gc-unknown-none-elf
MODE="release"
if [ $# -ge 2 ]; then
    if [ "$2"="2k1000" -o "$2"="laqemu" ]
    then
        TARGET=loongarch64-unknown-linux-gnu
        BLK_SZ="2048"
    else
        TARGET=$2
    fi
fi

if [ $# -ge 3 ]; then
    MODE="$3"
fi


ARCH=$(echo "${TARGET}" | cut -d- -f1| grep -o '[a-zA-Z]\+[0-9]\+')
echo
echo Current arch: ${ARCH}
echo
touch ${U_FAT32}
"$SUDO" dd if=/dev/zero of=${U_FAT32} bs=1M count=56
echo Making fat32 imgage with BLK_SZ=${BLK_SZ}
"$SUDO" mkfs.vfat -F 32 ${U_FAT32} -S ${BLK_SZ}
"$SUDO" fdisk -l ${U_FAT32}

if test -e ${U_FAT32_DIR}/fs
then 
    "$SUDO" rm -r ${U_FAT32_DIR}/fs
fi

"$SUDO" mkdir ${U_FAT32_DIR}/fs

"$SUDO" mount -f ${U_FAT32} ${U_FAT32_DIR}/fs
if [ $? ]
then
    "$SUDO" umount ${U_FAT32}
fi
"$SUDO" mount ${U_FAT32} ${U_FAT32_DIR}/fs

# build root
"$SUDO" mkdir -p ${U_FAT32_DIR}/fs/lib
# 修复 libc.so 路径
if [ -f "../../tmp/testsuits-for-oskernel/runtime/${ARCH}/lib64/libc.so" ]; then
    "$SUDO" cp ../../tmp/testsuits-for-oskernel/runtime/${ARCH}/lib64/libc.so ${U_FAT32_DIR}/fs/lib
elif [ -f "../user/lib/${ARCH}/libc.so" ]; then
    "$SUDO" cp ../user/lib/${ARCH}/libc.so ${U_FAT32_DIR}/fs/lib
fi
"$SUDO" mkdir -p ${U_FAT32_DIR}/fs/etc
"$SUDO" mkdir -p ${U_FAT32_DIR}/fs/bin
"$SUDO" mkdir -p ${U_FAT32_DIR}/fs/root
"$SUDO" sh -c "echo -e \"root:x:0:0:root:/root:/bash\n\" > ${U_FAT32_DIR}/fs/etc/passwd"
"$SUDO" touch ${U_FAT32_DIR}/fs/root/.bash_history

# 只能copy一个文件夹下所有内容，无法copy单文件
try_copy(){
    if [ -d $1 ]
    then
        echo copying $1 ';'
        for programname in $(ls -A $1)
        do
            "$SUDO" cp -fr "$1"/"$programname" $2
        done
    else
        echo "$1" "doesn""'""t exist, skipped."
    fi
}

# 复制用户程序
USER_BIN_DIR="../../NoAxiom-OS-User/bin"
if [ -d "$USER_BIN_DIR" ]; then
    for programname in $(ls $USER_BIN_DIR)
    do
        "$SUDO" cp -r "$USER_BIN_DIR/$programname" ${U_FAT32_DIR}/fs/
    done
else
    echo "User bin directory not found: $USER_BIN_DIR"
fi

if [ ! -f ${U_FAT32_DIR}/fs/syscall ]
then    
    "$SUDO" mkdir -p ${U_FAT32_DIR}/fs/syscall
fi

# try_copy ../user/busybox_lua_testsuites/${ARCH} ${U_FAT32_DIR}/fs/
# try_copy ../user/fs  ${U_FAT32_DIR}/fs/
# try_copy ../live/splice-test  ${U_FAT32_DIR}/fs/

"$SUDO" umount ${U_FAT32_DIR}/fs
echo "DONE"
return 0
