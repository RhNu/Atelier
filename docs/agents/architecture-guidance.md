# 架构指导

## 总原则

当前阶段只固定方向，不固定结构。

新项目应避免两类极端：

- 把所有领域能力堆进一个巨大的 `core` 或 `app` crate。
- 过早创建大量 crate，导致每个边界都还没有真实职责却已经增加维护成本。

更合适的方式是先按 feature 定义边界，再决定哪些 feature 值得独立 crate，哪些只需要 crate 内模块。

## 建议的边界问题

设计每个 feature 前先回答：

- 它提供什么用户可感知能力？
- 它拥有哪些数据？
- 它依赖哪些外部能力？
- 它是否需要独立测试、独立迁移、独立缓存或独立错误模型？
- 它暴露给前端的是命令、事件、查询，还是纯本地状态？

如果这些答案说不清，先不要拆成独立 crate。

## 后端方向

可以考虑这些逻辑边界，但不要先把它们写成固定 crate 清单：

- `interface`：前端、Tauri shell、可能的 CLI/MCP 共享 DTO。
- `workspace`：项目目录、设置、路径、安全读写、锁。
- `secret`：API key registry 与系统凭据后端。
- `novelai` adapter：把应用领域请求转换为 `novelai-bridge` 请求。
- `generation`：生成提交、参数归一化、Anlas 估算、bridge 调用前后的转换。
- `jobs`：排队、取消、重试、事件、进度、并发策略。
- `artifacts`：产物落盘、metadata、replay、导出变体、路径约束。
- `prompt`：Prompt 解析、格式化、诊断、函数描述。
- `resources`：chunk、preset、thumb、lexicon、Vibe 资源。
- `gallery`：产物索引、筛选、人工标记、跨 feature 入口。

这些边界可以逐步演化。第一版可以少一些，等职责变重再拆。

## 前端方向

如果选择 Solid 或 React，优先按 feature 组织，而不是按技术类型铺开：

```text
features/<feature>/
  data/        # query, mutation, cache invalidation
  model/       # local state, derived state, adapters
  ui/          # feature-owned components
  commands.ts  # typed IPC facade if needed
```

跨 feature 共享的东西要少而稳定：基础 UI、typed IPC、路径/资产工具、错误显示、i18n、通知。

## Tauri 边界

Tauri shell 应保持薄：

- 反序列化 command 参数。
- 调用 app service。
- 包装统一响应。
- 处理系统能力，例如文件对话框、剪贴板、通知、keyring。

不要在 Tauri command 中实现任务调度、Prompt 编译、Gallery 聚合或 NovelAI 请求构建。

## 文档策略

早期文档应偏短，记录正在形成的判断。等某个判断会影响长期实现成本时，再写 ADR 或更正式的 spec。
