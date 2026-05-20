# CREDITS

- 快照日期：2026-05-20
- 目的：记录本仓库在设计、对齐、实现与专项核对阶段参考过的外部项目、许可证与映射范围。

## 参考清单

| 项目 | 参考仓库 | 许可证 | 参考阶段 | 参考内容 | 对应位置 | 备注 |
|---|---|---|---|---|---|---|
| `NekoAI-JS` | `https://github.com/Nya-Foundation/NekoAI-JS` | AGPL-3.0 | 2026-05 generation 参数设计 | Anlas 估算公式与系数 | `crates/features/generation/src/estimate.rs` | 仅记录公式来源与行为对齐；Rust 实现按本仓类型、错误边界和测试要求重写。 |
