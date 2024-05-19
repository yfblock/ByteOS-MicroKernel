# ByteOS MicroKernel

## 介绍

> 这是一个使用了部分 ByteOS 组件的微内核，包含内核部分和用户程序部分

下面是 cloc 显示的数据


#### MicroKernel cloc

| Language                  |   files        |  blank      |  comment        |  code |
| --- | --- | --- | --- | --- |
| Rust                      |       9        |    155      |      189        |  1029 |
| Assembly                  |       1        |     11      |        1        |    58 |
| TOML                      |       1        |      1      |        0        |    13 |
| SUM:                      |      11        |    167      |      190        |  1100 |

#### Users cloc

| Language               |      files       |  blank     |  comment   |      code |
| --- | --- | --- | --- | --- |
| Rust                   |         14       |    216     |      236   |      1235 |
| D                      |        173       |    377     |        0   |       993 |
| JSON                   |        247       |      0     |        0   |       247 |
| LLVM IR                |          2       |     14     |        7   |        73 |
| TOML                   |          4       |      6     |        2   |        43 |
| Linker Script          |          1       |      2     |        0   |        33 |
| make                   |          1       |      5     |        0   |         8 |
| SUM:                   |        442       |    620     |      245   |      2632 |

## 运行

请确保您已经安装了 `rust` 工具链、`kbuild`、 `qemu-system-riscv` 以及制作 `fat32` 镜像的 `dosfstools`.

如果您需要安装 `kbuild`，那么执行以下代码

```shell
cargo install kbuild
```

```shell
# 第一步 制作镜像
make fs-img
# 第二步 运行 riscv64 版本
make BIN=riscv64-qemu LOG=error run-user
# 运行 aarch64 版本
make BIN=aarch64-qemu LOG=error run-user
# 运行 x86_64 版本
make BIN=x86_64-qemu LOG=error run-user
# 运行 loongarch64 版本
make BIN=loongarch64-qemu LOG=error run-user
```

然后使用 `help` 就可以看到可以执行的命令。

```plain
commands available are below:
      help
      ping
     disks
        ls
```

目前有 `vm`、`blk_device`、`fs`、`pong`、`ram_disk`、`shell` 五个任务，其中 `fs`、`pong`、`ram_disk` 是服务 `vm` 是 `root_server`.
