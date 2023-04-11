# rCore-N 开发日志

## 存在的问题

- SharedScheduler 的共享的文件格式
- SharedScheduler 优先级的更新方式
- 需要增加网络模块

### 20230409

- 把共享调度器编译成共享文件格式

### 20230408

- 把 Executor 中的 `Vec<VecDeque<CoroutineId>; PRIO_NUM>` 改成 `[VecDeque<CoroutineId>; PRIO_NUM]`

