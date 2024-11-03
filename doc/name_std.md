# 常量

使用下划线分隔的大写字母单词，并且可以在数字字面值中插入下划线来提升可读性。

例如：

```rust
const MAX_POINTS: u32 = 100_000;
```

# 局部变量

snake case 规范风格。所有字母都是小写并使用下划线分隔单词。

例如：

```rust
let max_points = 100_000;
```

# 全局变量

使用下划线分隔的大写字母单词，并且可以在数字字面值中插入下划线来提升可读性。尽量别用，除了 `lazy_static` 宏声明的。

例如：

```rust
static MAX_POINTS: u32 = 100_000;
```

# 函数名

snake case 规范风格。所有字母都是小写并使用下划线分隔单词。

例如：

```rust
fn max_point() {
    // code here
}
```

# 结构体名

camel case (大驼峰)规范风格。其字段的命名规范是 snake case 规范风格。

例如：

```rust
struct MyStruct {
    value: u32,
}
```

# 枚举名

camel case (大驼峰)规范风格。其字段的命名规范是 snake case 规范风格。

例如：

```rust
enum MyEnum {
    Value(u32),
}
```

# Trait 名

camel case (大驼峰)规范风格。

例如：

```rust
trait MyTrait {
    fn my_trait(&self);
}
```

# 文件/目录名

snake case 规范风格。

例如：

```shell
my_module.rs
```
