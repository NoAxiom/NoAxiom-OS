import subprocess

# 提取符号表
nm_output = subprocess.check_output(
    ["nm", "-C", "./target/riscv64gc-unknown-none-elf/release/kernel"]
).decode("utf-8")

# 过滤出函数符号
symbols = []
for line in nm_output.splitlines():
    parts = line.split()
    if len(parts) == 3 and parts[1] == "T":  # 只提取代码段中的函数
        addr = int(parts[0], 16)  # 将地址转换为整数
        name = parts[2]
        symbols.append((addr, name))

# 生成 Rust 代码
rust_code = "pub const SYMBOL_TABLE: &[(usize, &str)] = &[\n"
for addr, name in symbols:
    rust_code += f"    (0x{addr:x}, \"{name}\"),\n"
rust_code += "];\n"

# 写入文件
with open("./NoAxiom/kernel/src/utils/symbol_table.rs", "w") as f:
    f.write(rust_code)

print("Symbol table generated successfully!")