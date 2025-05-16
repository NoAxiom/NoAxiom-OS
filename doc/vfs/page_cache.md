# 文件系统缓存设计

## 整体层次结构

![层次结构](./pic/structure.svg)

## 读写流程

1. 首先通过 `PageCache` 进行文件内容缓存查找，若有，直接返回，反之进行下一步
2. 调用 `Ext4_rs` 库提供的读操作，将内容读出(这里具体步骤如下)，填充至 `PageCache` 中，返回
    1. 对于 `Ext4_ts` 所调用的 `BlockDevice` 的读写操作，会在 `BlockCache` 中进行缓存查找，若有，则直接返回，反之进行下一步
    2. 调用 `BlockDevice` 的读操作，读出数据块，填充至 `BlockCache` 中，返回

## 同步问题

目前简单的实现是，在kernel退出时，手动显式的执行 `PageCache` 的 `sync_all` 方法，实现所有脏数据的写回。当然可以为 `PageCache` 中的 `Page` 单元实现 `Drop trait`从而实现自动的写回操作。