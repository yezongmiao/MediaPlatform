# 验收记录 · 媒材工坊 MediaDesk v0.2.0

> 验收时间：2026-11-23 · 验收环境：macOS 26.6.2（Apple Silicon arm64，8GB）
> Node 22.18.0 · Rust 1.97.1 · Xcode 26 · VS Code 1.134.0

本记录只描述**实际执行过**的验证。第三方 AI CLI 与真实媒体服务没有可用凭据，
使用可控的 mock 验证完整链路；不把它们写成「已对接真实服务」。

---

## 1. 交付物

| 产物 | 路径 | 体积 | 校验 |
|---|---|---|---|
| macOS 安装包（DMG） | `release/媒材工坊_0.2.0_aarch64.dmg` | 3,680,102 bytes / 3.51 MiB | `hdiutil verify` VALID |
| macOS 应用（zip） | `release/媒材工坊_0.2.0_aarch64.app.zip` | 3,227,031 bytes / 3.08 MiB | 由 .app 压缩 |
| 浏览器版单二进制（macOS arm64） | `release/mediadesk-server-macos-arm64` | 3,339,136 bytes / 3.18 MiB | 独立目录启动 + HTTP 验证 |
| 浏览器版单二进制（Windows x64） | 由 GitHub Actions 构建 | — | 未在本机执行 |
| Windows NSIS 安装包 | 由 GitHub Actions 构建 | — | 未在本机执行 |

SHA-256：

```
b8d329fac92070f98ec2f328bc08be88d79b60daa54f181a649b0c4ae4c4fcb8  mediadesk-server-macos-arm64
88b3f9822be8a8af03d9e2d283c01aa5653fec1fbdb49702519a72cb131c5c09  媒材工坊_0.2.0_aarch64.app.zip
fc597985362a07a17b6896a1b9104d71ae106acac7997f272008a03b569d9d72  媒材工坊_0.2.0_aarch64.dmg
```

> macOS 构建为 **Apple Silicon（arm64）**。Intel Mac 需在对应机器重新构建；未提供 universal 包。

---

## 2. 自动化测试结果

### 2.1 基础检查

| 命令 | 结果 |
|---|---|
| `npm run typecheck` | 通过（0 error） |
| `npm test` | 3 个测试文件 / **12 tests passed** |
| `cargo test --workspace --exclude mediadesk-desktop` | **19 tests passed**（6 unit + 13 integration） |
| `cargo clippy --workspace --all-targets --exclude mediadesk-desktop -- -D warnings` | 通过，0 warning |
| `npm run build` | 通过；JS 222.34 kB（gzip 73.70 kB），CSS 101.83 kB（gzip 17.62 kB） |

### 2.2 Playwright 端到端

真实 Chromium + 真实 Rust 服务 + mock LLM + mock 媒体接口 + mock CLI + fake VS Code CLI：

```text
Running 5 tests using 1 worker
✓ creator workflow: capture → project → create → generate → VSCode → upload → card → skills → search
✓ mobile layout: drawer navigation and quick capture entry
✓ switch workspace from settings and restore it
✓ CLI text kernel and project assistant edits
✓ media generation saves an illustration into the project
5 passed
```

新增覆盖：

- 设置页选择 **Claude Code CLI** → 状态检测到 mock CLI → 保存；
- 生成文章时真实 spawn 本地进程，stdin 收到完整提示词，cwd 是项目目录；结果标识来源为 `claude`；
- 项目助手打开会话 → 覆盖式发送 → 模型输出 JSON 编辑 → 服务端写入素材 → 项目页刷新；测试通过 API 反查正文已变为助手版本；
- 设置媒体模型 → 插图生成 → mock 返回 base64 PNG → 服务端写入 `illustration/`、生成 sidecar 与提示词快照 → 弹层显示真实图片 → 打开素材编辑器。

原有覆盖仍通过：速记、立项、手写文章、API 测试、生成/保存、VSCode 上下文、上传、灯箱、文卡图、项目设置、技能、搜索、回收站、嵌套弹层 Esc、深色主题、移动端、工作区切换与恢复。

### 2.3 Rust 测试覆盖（19）

- 6 个单元测试：frontmatter 解析、媒体响应解析、任务 ID/状态/URL 提取、媒体扩展名识别；
- CLI：可执行文件探测、项目 cwd、真实子进程、CLI_NOT_FOUND、提示词快照；
- 媒体：图像 b64、视频 OpenAI 协议、Seedance 任务协议、落盘、sidecar、MEDIA_NOT_CONFIGURED；
- 助手：建议式不写文件、覆盖式写素材、历史持久化/清空、隐藏目录；
- 原有：工作区、五类素材、上传/Range/路径穿越、LLM 错误脱敏、VSCode、技能/预设、跨项目关联等。

---

## 3. 功能验收对照

| 目标 | 实现 | 实测证据 |
|---|---|---|
| 文本类生成走本地 CLI 内核 | 设置选择 API / Claude / OpenCode / DeepSeek；PATH 探测、绝对路径、`MEDIADESK_CLI_BIN` 覆盖；Windows `.cmd` 包装；cwd 锁项目；stdin 或 `{prompt}` | Playwright CLI 用例 + Rust CLI 集成测试；本机未装真实第三方 CLI，使用 mock |
| 插图/视频走媒体生成模型 | 图像 OpenAI Images；视频 OpenAI / task / Seedance；URL 或 b64；任务轮询 | Rust 三类 mock 协议测试 + Playwright 图像生成落盘 |
| 项目助手会话 | 项目页助手面板；历史 `.mediadesk/assistant/history.json`；建议/覆盖模式；只允许项目内 assetId | Playwright 覆盖式改稿 + Rust suggest/apply/history 测试 |
| 原有五类素材 / 预设 / 技能 / VSCode / 文卡图 | 保持并接入新媒体流程 | 原有 Playwright 与 Rust 测试全部通过 |
| 小体积、低占用 | Rust/Axum + Preact + 系统 WebView | 见下方实测 |
| Windows & macOS | 同一 Rust core + Tauri；CLI `.cmd` 适配 | macOS 本机验证；Windows 仅代码/CI，未实机 |

---

## 4. 体积与资源实测

| 指标 | 实测 |
|---|---|
| 浏览器服务二进制 | **3,339,136 bytes / 3.18 MiB** |
| 浏览器服务 RSS（空载） | **8.6 MiB** |
| 冷启动到 `/api/health` 可用 | **0.09s（缓存）– 0.55s（首次）** |
| 前端 JS / CSS（gzip） | **73.70 KiB / 17.62 KiB** |
| macOS `.app` | **5.95 MiB** |
| macOS `.dmg` | **3.51 MiB** |
| 原生进程 RSS | 空载约 35 MiB；打开工作台约 **94.7 MiB**（系统 WKWebView，XPC 进程另计） |

原生窗口启动后 `/api/health` 返回 `version: 0.2.0`，窗口 1320×901 可交互；截图见 `docs/screenshots/native.png`。

---

## 5. 安全与数据验证

- 服务只监听 `127.0.0.1`。
- CLI 子进程 `current_dir` 固定为项目目录；路径来自 `require_project`，不接受前端任意 cwd。
- 媒体配置的 API Key 只返回 `hasApiKey`，不返回明文（Rust 测试断言）。
- 助手只接受项目内真实 assetId；删除/越权路径没有接口入口。
- 助手历史保存在隐藏目录 `.mediadesk/assistant/`，`/files/*` 拒绝隐藏文件。
- 路径穿越、隐藏文件、符号链接逃逸测试继续通过。
- Markdown 经 DOMPurify 清洗；API 错误中的 Key 会被打码。
- 无遥测、CDN、外部字体。

---

## 6. 未验证 / 已知限制

1. **真实第三方 CLI 生成未验证**：本机检测到 `claude`（Claude Code 2.1.181，`/usr/local/bin/claude`），但为避免消耗用户账号/额度，没有发起真实生成；`opencode` / `deepseek` 未安装。已用 mock CLI 验证探测、cwd、stdin、超时/失败路径。不同 CLI 版本的参数差异可通过设置中的「附加参数」和路径适配，但未逐一实测。
2. **真实媒体服务未验证**：没有图像/视频服务凭据；用本地 mock 验证 b64、URL、OpenAI 视频与 Seedance 任务协议。不同厂商的额外必填字段可能有差异。
3. **Windows 未实机运行**：Windows 代码/CI 已配置，未执行。
4. 仅 macOS arm64 产物；未提供 Intel/universal。
5. 生成、助手均为非流式；视频在服务端轮询，UI 等待完成后显示。
6. 未做 Apple 签名/公证；公开分发需自行配置证书。

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
