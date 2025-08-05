if [ $# -ne 3 ]; then
    echo "Usage: $0 <lib_path> <platform> <arch>"
    exit 1
fi

LIB_DIR=$1
PLATFORM=$2
ARCH=$3

SOURCE_FILE=${LIB_DIR}/platform/src/ld/${ARCH}/${PLATFORM}.ld
TARGET_DIR=${LIB_DIR}/.ld
TARGET_FILE=${TARGET_DIR}/linker-${ARCH}.ld

mkdir -p ${TARGET_DIR}
cp ${SOURCE_FILE} ${TARGET_FILE}
if [ $? -ne 0 ]; then
    echo "Error: Failed to copy ${SOURCE_FILE} to ${TARGET_FILE}"
    exit 1
fi

echo "Linker script ${SOURCE_FILE} has been copied to ${TARGET_FILE}"
