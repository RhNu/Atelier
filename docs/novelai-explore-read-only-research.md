# NovelAI Explore Gallery 只读能力研究

- 研究日期：2026-08-31
- 研究对象：<https://novelai.net/explore/gallery>
- 研究范围：公开只读访问、搜索、排序、筛选、分页、帖子详情和图片读取
- 明确排除：上传、点赞、评论以及其他会改变服务端状态的功能

## 结论摘要

NovelAI Explore Gallery 的公开只读内容当前可以通过 HTTP 接口访问，而且不需要 API key。该接口位于独立主机 `https://explore.novelai.net`，公开网页前端正在使用它。

但这不是 NovelAI 官方 API 文档中承诺的 Explore API：

- 官方文档称凭证为 Persistent API Token，官方 API 使用 `Authorization` 请求头；没有文档说明该 token 可以访问 Explore。
- Explore 的 `/post/search`、`/post/{id}` 等接口没有出现在官方 Primary、Image 或 Text API 的 Swagger/OpenAPI 文档中。
- 当前测试中，`X-API-Key` 不参与 Explore 公开搜索认证；无认证请求反而可以成功。
- 有效 Persistent API Token 未在本研究中测试，因此“token 是否能访问 Explore 的额外内容”仍属于未确认项。

因此，当前可行的结论是：可以实现 Explore 的公共只读适配，但应把它视为网站当前使用的未公开接口，而不是稳定的官方 API-key 集成。

## 研究方法与证据边界

本研究使用未登录浏览器访问官方 Gallery 页面，并对公开页面实际调用的接口进行只读请求验证；没有提交上传、点赞或其他写入请求，也没有使用任何用户凭证。

当前网站前端代码确认 Explore 使用独立后端和以下请求路径：[Explore 当前前端 bundle](https://novelai.net/_next/static/chunks/9935-4ed79846118f1cd0.js)、[搜索实现 bundle](https://novelai.net/_next/static/chunks/4800-badddb361d8b9612.js)、[Gallery UI bundle](https://novelai.net/_next/static/chunks/3306-5ef25cfccb266997.js)。这些 bundle 是实现证据，不是版本化的公开协议；URL 和字段都可能随网站更新而变化。

本文中的数量、返回字段和错误行为是研究日期的现场快照，不代表永久不变的服务承诺。

## 官方 API 与 Explore 的边界

NovelAI 官方文档目前将 API 分为以下范围：

| API 范围 | 官方入口 | 主要内容 |
| --- | --- | --- |
| Primary API | <https://api.novelai.net/docs> | 用户接口和 `/ai/` 相关能力 |
| Image API | <https://image.novelai.net/docs/index.html> | 图像生成、放大、Vibe、标签建议等 |
| Text API | <https://text.novelai.net/docs/index.html> | 文本生成及 OpenAI-compatible 接口 |
| Explore | <https://explore.novelai.net> | 当前网页使用的社区帖子读取接口；未发现官方 API 文档 |

官方 Primary API 文档明确提示，第三方一般不应使用除 `/ai/` 之外的 Primary API 路径；集成应向用户请求 Persistent API Token。账号文档还说明，生成新的 Persistent API Token 会使旧 token 失效。[Primary API Swagger](https://api.novelai.net/docs) [Persistent API Token 文档](https://docs.novelai.net/en/text/usersettings/account/)

在官方 Image API 的当前 OpenAPI 文档中可以看到 `/ai/generate-image`、`/ai/upscale`、`/ai/encode-vibe`、`/oa/v1/models` 等路径，但没有 Explore 的 `/post/*` 路径。[Image API OpenAPI JSON](https://image.novelai.net/docs/doc.json)

## 当前已验证的只读接口

所有以下路径均位于 `https://explore.novelai.net`。

| 能力 | 方法与路径 | 未登录测试结果 |
| --- | --- | --- |
| 帖子搜索/列表 | `POST /post/search` | HTTP 200，返回分页结果 |
| 帖子详情 | `GET /post/{id}` | HTTP 200，返回 JSON |
| 缩略图 | `GET /post/thumbnail/{id}` | HTTP 200，返回 `image/webp` |
| 原图 | `GET /post/blob/{id}` | HTTP 200，返回 `image/webp` |

搜索接口返回的常见字段包括：

- 帖子 `id`、`type`、`created_at`、`title`、`description`；
- 创作者对象、`creator_id`；
- `moderation_status`、删除标记和点赞数量；
- 图片宽高、blurhash 和 `nai_metadata`。

帖子元数据在当前样本中包含 prompt、负面 prompt、模型名称、seed、steps、sampler、宽高等生成参数，但具体内容取决于作者是否公开以及帖子本身的元数据。

公开图片接口当前允许浏览器跨域读取，响应包含 `Access-Control-Allow-Origin: *`；这说明 WebView/浏览器直连在当前实现下可行。它仍然不是官方稳定性承诺，桌面后端直连也不受浏览器 CORS 限制。

## 搜索、排序和筛选能力

### 标签搜索

网页搜索框的 placeholder 是 `Search for a tag`，当前行为更接近精确标签搜索，而不是任意全文搜索：

- 一个标签可表示为 `{"field":"tag","value":"1girl"}`；
- 网页中用逗号分隔的多个标签会转换为多个 tag selector；
- 多个 tag selector 当前表现为 AND 关系；
- 现场测试中 `1girl` 返回 914 条，`1girl + solo` 返回 381 条；数量会随内容变化；
- 输入部分字符串 `girl` 没有返回结果，因此不能把它当作模糊文本搜索。

当前前端还存在 title selector 的构造代码，但现场请求中提供 title selector 后结果总数没有变化，说明服务端当前没有可靠执行该筛选。本文不把标题搜索计为可用能力。

### 排序

网页当前提供四种排序：

| UI 选项 | 请求语义 | 时间周期 |
| --- | --- | --- |
| New | `created_at` 降序 | 不适用 |
| Top | `top` 降序 | Day、Week、Month |
| Hot | `hot` 降序 | Day、Week、Month |
| Random | `random` 降序 | Day、Week、Month |

Top/Hot 的周期通过 selector 表示，例如 `{"field":"top","value":"week"}`。

Random 是一个特殊查询：当前服务端要求使用唯一的 random selector，例如 `day:abc123`，并且不能同时附加普通 tag 或 moderation selector。网页也会在 Random 模式下禁用标签搜索。

### 创作者筛选

当前接口支持 `creator_id` selector。网页会把作者名称和 ID 放入 URL，官方 Explore 说明也提到可以点击显示名称来筛选创作者。[Explore 官方说明](https://docs.novelai.net/en/misc/explore/)

### Moderation 筛选

未登录的公开 Gallery 查询由当前前端自动附加 Approved 状态筛选，现场观察到的内部值为 `moderation_status: "1"`。这是当前内部枚举，不是公开稳定协议。

登录后前端还存在“只显示我的帖子”“只显示我点赞的帖子”等控制，但它们不属于本研究的公共只读核心范围。未认证调用 `POST /post/search/liked_by_self` 返回 HTTP 401。

## 分页与请求示例

### 标签查询

```http
POST https://explore.novelai.net/post/search
Content-Type: application/json
```

```json
{
  "orderers": [
    { "field": "created_at", "sort_direction": "desc" }
  ],
  "selectors": [
    { "field": "tag", "value": "1girl" },
    { "field": "tag", "value": "solo" },
    { "field": "moderation_status", "value": "1" }
  ],
  "pagination": {
    "limit": 50,
    "offset": 0
  }
}
```

响应包含 `pagination.total`、`limit`、`offset` 和 `results`。当前服务端接受的最大 `limit` 为 200，网页前端通常每批请求 50 条。

### Top/Hot 查询

```json
{
  "orderers": [
    { "field": "top", "sort_direction": "desc" }
  ],
  "selectors": [
    { "field": "top", "value": "week" }
  ],
  "pagination": {
    "limit": 50,
    "offset": 0
  }
}
```

Hot 查询将 `top` 替换为 `hot`。在公开网页查询中，服务端当前会根据查询模式处理公开/审核状态；`moderation_status` 的组合规则应视为内部行为，不能作为稳定协议假设。

### Random 查询

```json
{
  "orderers": [
    { "field": "random", "sort_direction": "desc" }
  ],
  "selectors": [
    { "field": "random", "value": "day:abc123" }
  ],
  "pagination": {
    "limit": 50,
    "offset": 0
  }
}
```

`abc123` 仅表示六位随机盐的示例。Random 查询不能再附加其他 selector；这是当前服务端的校验行为。

## API key 认证结论

| 问题 | 结论 |
| --- | --- |
| 公开 Gallery 是否需要 key？ | 不需要。公开搜索、详情和图片读取均可无认证完成。 |
| 是否使用 `X-API-Key`？ | 当前 Explore 公开搜索忽略该请求头；它不是已观察到的认证方案。 |
| Persistent API Token 是否官方支持 Explore？ | 未发现官方文档或官方协议声明；本研究未使用真实 token 验证。 |
| `Authorization: Bearer ...` 是否存在？ | 当前前端在登录状态下会附加该请求头；无效值会收到 401，但这不能证明有效 Persistent API Token 一定适用于 Explore。 |
| 是否能访问私有/用户态只读内容？ | 未确认；公开内容和需要登录的用户态查询应分开建模。 |

## 限制与风险

- Explore 后端没有发现公开 Swagger/OpenAPI 文档；其接口来自当前网站实现，字段、路径和认证行为可能变化。
- 当前没有可依赖的 Explore API 版本、限流或 SLA 文档；应采用低频、按需、可失败的读取策略。
- 结果总数、排序结果和内容会持续变化，本文的数量只用于证明现场能力。
- Explore 是社区分享和发现页面，官方说明其内容需要审核，并对可分享内容有规则限制；FAQ 说明 Explore 是用户主动选择加入的社区功能。[Explore 官方说明](https://docs.novelai.net/en/misc/explore/) [NovelAI FAQ](https://docs.novelai.net/en/faq/)
- 官方文档对第三方 API 集成的正式支持边界仍是 Persistent API Token 以及对应的 Primary/Image/Text API，不包括本文观察到的 Explore 内部接口。

## 研究来源

- [NovelAI Explore Gallery](https://novelai.net/explore/gallery)
- [Explore 官方说明](https://docs.novelai.net/en/misc/explore/)
- [NovelAI FAQ](https://docs.novelai.net/en/faq/)
- [Primary API Swagger](https://api.novelai.net/docs)
- [Image API 文档](https://image.novelai.net/docs/index.html)
- [Text API 文档](https://text.novelai.net/docs/index.html)
- [Image API 当前 OpenAPI JSON](https://image.novelai.net/docs/doc.json)
- [账号与 Persistent API Token](https://docs.novelai.net/en/text/usersettings/account/)


## 实施时复核（2026-08-31）

Atelier 独立匿名客户端已低频复核 New、Top、Hot、Random、Random offset 翻页、
单帖详情和缩略图。普通测试使用本地 HTTP fixture；现场烟测显式标记为 ignored，需手动运行。
官方画廊可见帖子链接为 `https://novelai.net/explore/image/{post_id}`，不是 gallery 查询参数。
这些结果只说明复核时公共接口可用，不构成稳定性承诺。
