# 项目意图

## 目标

`NAI Atelier` 是一个专门面向 NovelAI 绘图工作流的桌面应用。它应该服务于长期使用 NovelAI 的创作者，而不是做一个可接所有模型、所有平台的泛用生成器。

核心体验可以围绕这些能力展开：

- Prompt 编写、解析、复用与资源化。
- NovelAI 参数、模型、Vibe、参考图与 Director 工具的稳定提交体验。
- 可观察、可取消、可重试、可回放的生成任务。
- 产物 Gallery、metadata、导出、回填与安全标记。
- 多 API key 管理，但默认保持用户可理解的手动切换语义。

## 设计偏好

- 产品语言要承认它是 NovelAI 专用工具。
- 后端应把 NovelAI 网络协议细节隔离到 `novelai-bridge` adapter 后面。
- 应用内部使用自己的领域语言，例如 workspace、job、artifact、prompt resource、gallery item。
- UI 不应只是旧 `nait` 页面复刻；应围绕高频创作路径重新组织。

## 暂不承诺

- 不承诺兼容旧 `nait` 的所有命令名或 DTO。
- 不承诺迁移旧数据库或工作目录。
- 不承诺第一版包含完整 orchestration、NSFW 检测、Director、Vibe 全量能力。
- 不承诺前端必须选择 Solid 或 React；需要单独比较生态、状态管理、测试和 Tauri 集成成本。
