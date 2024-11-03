## NoAxiom项目结构

### NoAxiom

放置项目核心开发代码

内部子目录:

 - kernel: 内核代码的实现并整合lib中的功能, 含有Makefile
 - lib: 功能模块的实现
 - user: 用户态的各类功能维护

### vendor

本地保存的依赖库环境

### test

用于进行官方提供的测例运行环境搭建

### part

暂定未来用于进行磁盘相关(文件, 磁盘镜像等)的管理

### 各类config文件的位置

#### 根目录

`.cargo/`内部放置了vendor相关的配置文件

`rust-toolchain.toml`放置工具链版本

`rustfmt.toml`放置格式化选项

`Cargo.toml`保存了工作区的路径指引

#### NoAxiom核心子目录

`*/Cargo.toml`当前crate/lib下的依赖