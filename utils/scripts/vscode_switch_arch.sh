#!/bin/bash

# 定义文件路径
SETTINGS_FILE="../../.vscode/settings.json"

# 检查文件是否存在
if [ ! -f "$SETTINGS_FILE" ]; then
    echo "Error: $SETTINGS_FILE not found!"
    exit 1
fi

# 检查文件是否包含目标行
if ! grep -q '"rust-analyzer.cargo.target":' "$SETTINGS_FILE"; then
    echo "Error: rust-analyzer.cargo.target setting not found in $SETTINGS_FILE"
    exit 1
fi

# 获取当前target值
current_target=$(grep '"rust-analyzer.cargo.target":' "$SETTINGS_FILE" | awk -F'"' '{print $4}')

# 根据当前值决定新值
if [ "$current_target" = "riscv64gc-unknown-none-elf" ]; then
    new_target="loongarch64-unknown-none"
elif [ "$current_target" = "loongarch64-unknown-none" ]; then
    new_target="riscv64gc-unknown-none-elf"
else
    echo "Error: Unknown target '$current_target'"
    exit 1
fi

# 替换文件内容
sed -i "s/\"rust-analyzer.cargo.target\": \".*\"/\"rust-analyzer.cargo.target\": \"$new_target\"/" "$SETTINGS_FILE"

echo "Target changed from $current_target to $new_target in $SETTINGS_FILE"