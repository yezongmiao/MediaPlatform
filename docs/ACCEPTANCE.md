# 验收记录 · 媒材工坊 MediaDesk v0.6.0

> 验收时间：2026-11-23 · 验收环境：macOS 26.6.2（Apple Silicon arm64，8GB）
> Node 22.18.0 · Rust 1.97.1 · Xcode 26 · VS Code 1.134.0

本记录只描述**实际执行过**的验证。第三方 AI CLI 与真实媒体服务没有可用凭据，
使用可控的 mock 验证完整链路；不把它们写成「已对接真实服务」。服务商模型列表对
OpenRouter 与 OpenCode Go 做了真实网络请求；其余厂商需要用户自己的 Key，未冒充已验证。

---

## 1. 交付物

| 产物 | 路径 | 体积 | 校验 |
|---|---|---|---|
| macOS 安装包（DMG） | `release/媒材工坊_0.6.0_aarch64.dmg` | 3,806,701 bytes / 3.63 MiB | `hdiutil verify` VALID |
| macOS 应用（zip） | `release/媒材工坊_0.6.0_aarch64.app.zip` | 3,342,346 bytes / 3.19 MiB | 由 .app 压缩 |
| 浏览器版单二进制（macOS arm64） | `release/mediadesk-server-macos-arm64` | 3,537,824 bytes / 3.37 MiB | 独立目录启动 + HTTP 验证 |
| 浏览器版单二进制（Windows x64） | 由 GitHub Actions 构建 | — | 未在本机执行 |
| Windows NSIS 安装包 | 由 GitHub Actions 构建 | — | 未在本机执行 |

SHA-256：

```
4e546d6d2640ecea267cf6529da2f28e76e36362b543e3ba0cc915a871ab1e45  mediadesk-server-macos-arm64
2e26910f534776b5a81a8297ec44c3b435675ba90acee8c787410bcfbd0a7d17  媒材工坊_0.6.0_aarch64.app.zip
5e4b6da03134a32a38df7bc4d1f001549c2dea874cc831ed66bf0b35bc9c1203  媒材工坊_0.6.0_aarch64.dmg
```

> macOS 构建为 **Apple Silicon（arm64）**。Intel Mac 需在对应机器重新构建；未提供 universal 包。

---

## 2. 自动化测试结果

### 2.1 基础检查

| 命令 | 结果 |
|---|---|
| `npm run typecheck` | 通过（0 error） |
| `npm test` | 5 个测试文件 / **19 tests passed** |
| `cargo test --workspace --exclude mediadesk-desktop` | **31 tests passed**（13 unit + 18 integration） |
| `cargo clippy --workspace --all-targets --exclude mediadesk-desktop -- -D warnings` | 通过，0 warning |
| `cargo fmt --all -- --check` | 通过 |
| `npm run build` | 通过；JS 242.99 kB（gzip 80.32 kB），CSS 118.69 kB（gzip 20.18 kB） |

### 2.2 Playwright 端到端

真实 Microsoft Edge（Chromium 内核）+ 真实 Rust 服务 + mock LLM + mock 媒体接口 + mock CLI + fake VS Code CLI：

```text
Running 10 tests using 1 worker
✓ creator workflow: capture → project → create → generate → VSCode → upload → skills → search
✓ mobile layout: drawer navigation and quick capture entry
✓ IME composition is not interrupted by Enter or controlled input resets
✓ switch workspace from settings and restore it
✓ CLI text kernel and project assistant edits
✓ assistant creates new card assets in an empty directory
✓ asset management deletes one item and a batch
✓ AI card studio generates an editorial PNG and saves it
✓ media generation saves an illustration into the project
✓ provider catalog: accordion and model list
10 passed (40.6s)
```

本轮新增覆盖：

- 服务商目录返回 12 个内置服务商：DeepSeek、Claude、Codex、OpenAI、OpenCode Go、字节方舟、小米、OpenRouter、Kimi、MiniMax、Ollama、自定义；
- 折叠列表同一时间只展开一个服务商；可搜索过滤；
- 选择 OpenRouter 后填 mock Base URL，点击「获取列表」→ 本地 Rust 代理请求 `/v1/models` → 前端展示 2 个 mock 模型，选择后回填模型 ID；
- 模型列表请求失败时显示错误与重试，不阻塞手动填写；
- 项目助手走 SSE 流式接口：建议式生成后出现「采纳并写入」，确认后覆盖素材，按钮变为「已写入」，刷新后状态保持；
- 空 card 目录场景：助手返回 `newAssets`，建议式下文件数仍为 0；确认后创建 3 个 `.md` 文卡图素材，项目 `assets.card` 数量变为 3；
- 输入法组合：合成 `compositionstart` / `isComposing` 事件期间按 Enter 不会添加标签或触发保存，组合结束后文字仍能提交；
- AI 文卡图：mock 文本模型返回 3 张结构化方案，mock 图像模型逐张返回 PNG → Canvas 生成 3 张 1080×1440 卡片 → 一次保存全部后 `assets.card` 有 3 个真实 `fileUrl`，同时保存 3 张 AI 底图插图；
- 批量删除：单张删除确认后卡片减少；批量选择 2 张后一次移入回收站，项目 `assets.article` 变为 0；
- 助手入口：切换到「脚本」Tab 后右下角悬浮「助手」仍可见并可打开右侧栏。

原有覆盖仍通过：速记、立项、手写文章、API 测试、生成/保存、VSCode 上下文、上传、灯箱、项目设置、技能、搜索、回收站、嵌套弹层 Esc、深色主题、移动端、工作区切换与恢复、CLI 文本内核、右侧栏助手流式建议/采纳覆盖、媒体生成落盘。

Playwright 默认复用系统 Microsoft Edge（`executablePath` 检测）；本机不再需要下载 `chromium-1228`。CI 或无 Edge 的环境仍可通过 `CHROMIUM_PATH` 指定托管浏览器，或设置 `PW_BROWSER=chromium` 回退。

### 2.3 Rust 测试覆盖（31）

- 13 个单元测试：frontmatter 解析、媒体响应解析、任务 ID/状态/URL 提取、媒体扩展名识别；服务商目录包含必需厂商、OpenCode `x-opencode-session` 自动头、Anthropic `x-api-key` + `anthropic-version`、协议 URL 生成；**文卡图 JSON 提取、按文章长度算卡数、空标题/空正文校验**；
- 18 个集成测试：服务商目录 API、模型列表归一化、Anthropic Messages 协议、OpenAI Responses 协议、**AI 文卡图结构化方案**、媒体三类协议、CLI 探测/cwd/stdin/错误、助手 suggest/apply/history、**助手 SSE 流式接口（API 与 Claude stream-json）**、**保存后的建议再次采纳**、**空 card 目录批量新建**、文件安全、VSCode、技能/预设等。

### 2.4 真实网络与流式验证（非自动化测试）

| 验证 | 结果 |
|---|---|
| OpenRouter `GET /api/v1/models` | 返回 477 个模型，已归一化为 id + 名称 |
| OpenCode Go `GET /inference/go/openai/v1/models` | 返回 30 个模型，公开可读 |
| OpenCode Go 聊天请求（故意用无效 Key） | HTTP 401 `Invalid credential`，**不再返回 `MissingSessionID`**；证明 `x-opencode-session` 已自动附加 |
| 小米 MiMo / MiniMax / 字节方舟 / Kimi 的 `/models` | 未带 Key 请求返回 401，端点可达；尚未用真实 Key 拉取列表 |
| Claude Messages / Codex Responses | 用本地 mock 验证请求体与响应解析；无真实付费账号，未发起真实请求 |
| 助手增量 SSE（慢速 stream-json CLI） | 状态事件后约 0.03s 收到首段 delta，1.4s 后收到下一段，1.9s 收到 done；界面在生成过程中显示文本与光标 |
| 助手建议采纳 | 保存 suggest 消息 → 通过 `/chat/apply` 覆盖 → 素材内容变化、`applied=true`、刷新后按钮不再出现 |
| 助手空目录新建 | 空 card 目录 → suggest 只返回 newAssets，文件数仍为 0 → apply 生成 3 个 `.md`；项目 card 计数为 3 |
| AI 文卡图 | 文章 → 按长度拆多张 JSON 方案 → 逐张 mock 图像配图 → Canvas 1080×1440 → 一次保存全部 card PNG |
| 批量删除 | 文章/脚本/插图/文卡/视频管理页支持勾选、全选、删除所选；卡片悬停独立删除 | Playwright 单项删除 + 批量删除；服务端统一移入 `.mediadesk/trash` |
| 助手随时唤起 | 项目任意 Tab 右下角悬浮入口；`⌘⇧A` / `Ctrl+Shift+A` 快捷键 | Playwright 在脚本 Tab 通过悬浮入口打开右侧栏 |

---

## 3. 功能验收对照

| 目标 | 实现 | 实测证据 |
|---|---|---|
| 常用服务商 + 自定义 | 12 个内置服务商；Anthropic Messages、OpenAI Responses、OpenAI Chat 三种协议；自定义 Base URL/协议/请求头 | 服务商目录 API 测试 + 设置页 E2E |
| 特殊字段/请求头 | OpenCode Go 自动 `x-opencode-session`；Anthropic 自动 `x-api-key` + `anthropic-version`；OpenRouter 自动 `HTTP-Referer` / `X-Title`；支持自定义请求头 | Rust 单元测试 + 真实 OpenCode 401 验证 |
| 模型列表获取 | 本地 Rust 代理 `GET {base}/models`，归一化 `data/models`，前端可搜索选择 | Rust mock 列表测试 + Playwright 模型列表 E2E + 真实 OpenRouter/OpenCode 列表 |
| 折叠式交互 | 服务商默认仅显示名称与说明；点击展开唯一面板；搜索过滤；切换服务商清空当前 Key 输入 | Playwright 断言 `.provider-panel` 数量、搜索过滤、模型选择 |
| 文本类生成走 CLI/API | 设置选择 API / Claude / Codex / OpenCode / DeepSeek；PATH 探测、绝对路径、`MEDIADESK_CLI_BIN` 覆盖；cwd 锁项目 | Playwright CLI 用例 + Rust CLI 集成测试 |
| 插图/视频走媒体模型 | 图像 OpenAI Images；视频 OpenAI / task / Seedance；URL 或 b64；任务轮询 | Rust 三类 mock 协议测试 + Playwright 图像生成落盘 |
| 项目助手会话 | 项目页右侧栏，素材同屏可浏览；历史 `.mediadesk/assistant/history.json`；SSE 流式；建议/覆盖模式及建议后采纳；空目录可批量新建文本素材 | Playwright 右侧栏、流式建议/采纳、空 card 新建；Rust SSE、suggest/apply/history/newAssets 测试 |
| AI 文卡图 | 文章/脚本文本 → 结构化卡片方案；可选图像模型配图；Canvas 1080×1440 PNG 入库 | Playwright AI 文卡图生成/保存；Rust 方案解析与路由测试 |
| 输入法兼容 | 组合输入期间屏蔽 Enter 快捷键，受控输入框延迟到 compositionend 提交 | Playwright 合成组合事件测试；`isImeComposing` 单元测试 |
| 原有素材/预设/技能/VSCode/文卡图 | 保持并接新媒体流程 | 原有 Playwright 与 Rust 测试全部通过 |
| 小体积、低占用 | Rust/Axum + Preact + 系统 WebView | 见下方实测 |
| Windows & macOS | 同一 Rust core + Tauri；CLI `.cmd` 适配 | macOS 本机验证；Windows 仅代码/CI，未实机 |

---

## 4. 体积与资源实测

| 指标 | 实测 |
|---|---|
| 浏览器服务二进制 | **3,537,824 bytes / 3.37 MiB** |
| 浏览器服务 RSS（空载） | **8,576 KiB / 8.4 MiB** |
| 冷启动到 `/api/health` 可用 | **0.147s**（缓存态） |
| 前端 JS / CSS（gzip） | **80.32 KiB / 20.18 KiB** |
| macOS `.app` | **6.17 MiB** |
| macOS `.dmg` | **3.63 MiB** |
| 原生主进程 RSS（打开工作台） | **94,816 KiB / 92.6 MiB**（系统 WKWebView，XPC 进程另计） |

原生窗口启动后 `/api/health` 返回 `version: 0.6.0`，窗口可交互；截图见 `docs/screenshots/native.png`。
DMG 已通过 `hdiutil verify` 校验。

---

## 5. 安全与数据验证

- 服务只监听 `127.0.0.1`。
- CLI 子进程 `current_dir` 固定为项目目录；路径来自 `require_project`，不接受前端任意 cwd。
- 文本/媒体 API Key 只返回 `hasApiKey`，不返回明文；模型列表代理同样不回传 Key。
- 模型列表请求中的服务商错误会打码 Key；测试断言配置接口不暴露 `apiKey`。
- 切换服务商时，前端不会把上一家的 Key 自动发给新服务商；保存时清空旧 Key 输入。
- 自定义请求头限制数量/长度，拒绝换行、控制字符与 Content-Type/Host 等受管头。
- 助手只接受项目内真实 assetId；删除/越权路径没有接口入口。
- 助手历史保存在隐藏目录 `.mediadesk/assistant/`，`/files/*` 拒绝隐藏文件。
- 路径穿越、隐藏文件、符号链接逃逸测试继续通过。
- Markdown 经 DOMPurify 清洗；无遥测、CDN、外部字体。

---

## 6. 未验证 / 已知限制

1. **真实第三方 CLI 生成与 Claude 流式未验证**：本机检测到 `claude`（Claude Code 2.1.181，`/usr/local/bin/claude`），但为避免消耗用户账号/额度，没有发起真实生成；`codex` / `opencode` / `deepseek` 未安装或未验证。已用 mock CLI 验证探测、cwd、stdin、超时/失败路径，以及 Claude `stream-json` 事件解析。不同 CLI 版本/账号的参数差异可通过「附加参数」和路径适配，但未逐一实测。
2. **服务商模型列表并非全部真实验证**：OpenRouter、OpenCode Go 已真实拉取；Anthropic / Codex / DeepSeek / Kimi / MiniMax / 小米 / 字节方舟需要用户自己的 Key，未用真实账号拉取。Anthropic Messages 与 Responses 协议仅用 mock 验证。
3. **真实媒体服务未验证**：没有图像/视频服务凭据；用本地 mock 验证 b64、URL、OpenAI 视频与 Seedance 任务协议。不同厂商的额外必填字段可能有差异。
4. **Windows 未实机运行**：Windows 代码/CI 已配置，未执行。
5. 仅 macOS arm64 产物；未提供 Intel/universal。
6. 普通素材生成仍为非流式；助手已流式化。视频在服务端轮询，UI 等待完成后显示。
7. 助手新建文卡图保存的是文案 `.md` sidecar，不渲染 PNG；需要图片时请用「AI 文卡图」生成。
8. AI 文卡图的正文与来源由文本模型提炼，未做事实校验；配图质量取决于媒体模型，未用真实付费图像账号验收。文卡配图会作为独立插图素材保存。
9. 输入法兼容用合成组合事件验证，未在每一种系统输入法/浏览器组合上实测。
10. 未做 Apple 签名/公证；公开分发需自行配置证书。

---

## 7. 复现命令

```bash
npm ci
npm run check
cargo clippy --workspace --all-targets --exclude mediadesk-desktop -- -D warnings
npm run e2e
npm run build
cargo build --release -p mediadesk-server
npx tauri build --bundles dmg,app
```

注意：`core/build.rs` 会把 `dist/` 文件注册为 Cargo 依赖；即使只改了前端，重新 `cargo build` 也会刷新内嵌 UI。
