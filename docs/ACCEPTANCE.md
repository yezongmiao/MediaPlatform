# 验收记录 · 媒材工坊 MediaDesk v0.1.0

> 验收时间：2026-11-20 · 验收环境：macOS 26.6.2（Apple Silicon arm64，8GB）  
> Node 22.18.0 · Rust 1.97.1 · Xcode 26 · VS Code 1.134.0

本记录只描述**实际执行过**的验证。Windows 未在本机实机运行，单独列出边界。

---

## 1. 交付物

| 产物 | 路径 | 体积 | 校验 |
|---|---|---|---|
| macOS 安装包（DMG） | `release/媒材工坊_0.1.0_aarch64.dmg` | 3.41 MiB | `hdiutil verify` CRC 有效 |
| macOS 应用（zip） | `release/媒材工坊_0.1.0_aarch64.app.zip` | 3.0 MiB | 解压后启动验证 |
| 浏览器版单二进制（macOS arm64） | `release/mediadesk-server-macos-arm64` | 3.0 MiB | 独立目录启动 + 实际 HTTP 验证 |
| 浏览器版单二进制（Windows x64） | 由 GitHub Actions 构建 | — | 未在本机执行 |
| Windows NSIS 安装包 | 由 GitHub Actions 构建 | — | 未在本机执行 |

SHA-256：

```
c2a5354e0314d9056f6e6658985242bd493c5aacc878a388a9ebe17c64e58ded  mediadesk-server-macos-arm64
128d05931ecd0f93b963709b019772dd4408eba6781f28e73a4bb1ff1a9b7a05  媒材工坊_0.1.0_aarch64.app.zip
8862b4d09ea6c0640b3afcb88747f786a628027da7cbc6eed8f04647e6917771  媒材工坊_0.1.0_aarch64.dmg
```

> 当前 macOS 构建为 **Apple Silicon（arm64）**。Intel Mac 需在对应机器上重新构建；未提供 universal 包。

---

## 2. 自动化测试结果

### 2.1 基础检查

| 命令 | 结果 |
|---|---|
| `npm run typecheck` | 通过（0 error） |
| `npm test` | 3 个测试文件 / **12 tests passed** |
| `cargo test --workspace --exclude mediadesk-desktop` | **12 tests passed**（3 unit + 9 integration） |
| `cargo clippy -p mediadesk-core -p mediadesk-server --all-targets -- -D warnings` | 通过，0 warning |
| `npm run build` | 通过；JS 203.06 kB（gzip 68.08 kB），CSS 78.10 kB（gzip 14.09 kB） |

### 2.2 Playwright 端到端（真实 Chromium + 真实 Rust 服务 + mock LLM + fake VS Code CLI）

```text
Running 3 tests using 1 worker
✓ creator workflow: capture → project → create → generate → VSCode → upload → card → skills → search (11.9s)
✓ mobile layout: drawer navigation and quick capture entry (4.5s)
✓ switch workspace from settings and restore it (2.5s)
3 passed (23.2s)
```

覆盖的真实操作：

- `⌘N` 速记 → 灵感卡片 → 立项 → 项目页；
- 新建文章、编辑正文、保存、重开验证；
- 设置 Base URL / 模型 / API Key → **保存** → **真实发起测试请求** → 生成；
- 生成弹层中预览编译后的提示词，再调用 mock OpenAI-compatible 服务，确认结果、保存素材、写入 prompt 快照；
- 点击「在 VSCode 打开」，检查 `AGENTS.md` / `CLAUDE.md` / `.vscode/*` / `.mediadesk/context.json` / `skills/` / `presets/` 实际落盘，fake CLI 收到项目路径；
- 上传 PNG 插图 → 显示 → 修改标题 → 灯箱预览；
- 文卡图 Studio 输入文案 → 选择主题 → Canvas 输出 1080×1440 → 上传进项目；
- 项目设置：改状态、加标签 → 徽章更新；
- 新建技能并出现在技能库；
- `⌘K` 搜索项目并跳转；
- 删除文卡图 → 二次确认 → 素材消失 → `.mediadesk/trash` 中出现 PNG；嵌套弹层按 `Esc` 只关闭最上层；
- 深色主题切换 → 刷新后保持；
- 移动端抽屉导航 + 移动端速记；
- 设置中切换工作区到空目录 → 数据变为空 → 再切回 → 原想法/项目/素材恢复。工作区切换测试额外重复 3 次全部通过。

### 2.3 Rust 集成测试覆盖

- 工作区初始化、内置 7 技能 / 5 预设、配置脱敏、Unix 0600；
- 想法 CRUD、含 `---` 的正文、立项、解除关联、回收站；
- 五类素材 CRUD、非法 kind 拒绝；
- PNG/视频上传、Range 206、外部文件收编、中文文件名；
- `../` 路径穿越、`.mediadesk` 隐藏文件、符号链接逃逸全部拒绝；
- Prompt 编译包含项目、灵感、预设、技能与类型约束；
- LLM 成功、未配置 412、401 错误脱敏、提示词快照落盘；
- VSCode 上下文生成、选中技能复制、fake CLI 启动、CLI 缺失时给出 `VSCODE_NOT_FOUND` 与手动命令；
- 跨项目关联同一想法时，旧项目自动移除链接；
- 技能/预设 CRUD 与删除进回收站。

---

## 3. 功能验收对照

| 用户要求 | 实现 | 实测证据 |
|---|---|---|
| 随手记录想法 + 文件管理 | 全局速记、灵感页、YAML frontmatter + Markdown、标签/状态/搜索、回收站 | E2E 步骤 1；Rust idea 测试；`ideas/*.md` 实际落盘 |
| 想法立项，项目独立一页 | 立项按钮 / 新建项目；项目页、概览与设置；想法关联去重 | E2E 立项；Rust promote + 跨项目链接测试 |
| 项目内文章/脚本/插图/文卡图/视频分类 | 五类目录与标签页；媒体同名 sidecar；上传/替换/删除 | E2E 文章、插图、文卡图；Rust 五类 CRUD + 视频 |
| 各环节预设、可加载 Skill | 7 个内置技能、5 个内置预设、可视化 CRUD、生成时勾选、项目级选择 | E2E 新建技能；Rust 技能/预设测试；生成 Prompt 包含预设与技能 |
| 手动调用 LLM 生成各类素材 | OpenAI-compatible 代理；文章/脚本/插图提示词/文卡文案/视频分镜；结果确认后入库 | E2E 真实 HTTP 调用 mock LLM；Rust mock 成功/401/未配置 |
| 每个项目打开 VSCode + 自动准备文件 | 生成 AGENTS/CLAUDE/README/context/.vscode/skills/presets，调用 `code`；缺失时给出命令 | E2E fake CLI 与文件断言；Rust 真实 spawn `/usr/bin/true`；本机 `code` 1.134.0 已安装 |
| 美观简洁 UI | 自建设计系统、明暗主题、响应式、文卡图 Studio、命令面板 | 截图为实际运行画面；E2E 无 console error |
| 内存小、安装包小 | Rust/Axum + 系统 WebView；内嵌 UI 单二进制 | 见下方实测 |
| Windows & macOS | 同一 Rust core + Tauri 2；Windows `.cmd` 启动适配；CI 双平台 | macOS 本机验证；Windows 仅代码/CI 配置，未实机 |

---

## 4. 体积与资源实测

| 指标 | 实测 |
|---|---|
| 浏览器服务二进制 | **3.0 MiB** |
| 浏览器服务 RSS（空载，macOS） | **8.3–8.9 MiB** |
| 浏览器服务冷启动到 `/api/health` 可用 | **0.09s（文件已缓存）～0.71s（首次启动）** |
| 前端 JS / CSS（gzip） | **68.07 KiB / 14.09 KiB** |
| macOS `.app` | **5.75 MiB** |
| macOS `.dmg` | **3.41 MiB** |
| 原生进程 RSS（随页面波动） | 空载约 **35 MiB**；打开完整工作台约 **98 MiB**；系统 WKWebView XPC 进程另计且系统共享 |
| 对比 Electron 典型体积 | 安装包通常 70–120MB、空载内存 150MB+（仅作量级参考，未在本机测 Electron） |

单二进制验证方式：复制到 `/tmp` 的空目录，从该目录启动，在无 `dist/`、无 `node_modules/` 的情况下访问 `/` 与 `/assets/*.js` 均返回 200。

---

## 5. 安全与数据验证

- 服务只监听 `127.0.0.1`，未监听局域网地址。
- `/files/*` 仅允许 `projects/` 下非隐藏文件；Rust 测试验证：
  - `projects/../../.mediadesk/config.json` → `PATH_FORBIDDEN`；
  - `.mediadesk/config.json` → `PATH_FORBIDDEN`；
  - 指向配置文件的符号链接 → `PATH_FORBIDDEN`。
- 配置接口不返回 `apiKey` 明文；`hasApiKey` 仅表示是否配置。
- mock 服务 401 错误返回中不出现测试 Key（测试断言）。
- Markdown 预览经 DOMPurify 清洗；Vitest 验证 `<script>`、`onerror`、`javascript:` 被移除。
- 删除一律进 `.mediadesk/trash/`；E2E 验证素材删除后 PNG 出现在回收站。
- 没有遥测、CDN、外部字体请求（E2E 仅本机请求）。

---

## 6. 原生应用启动验证

- 启动 `target/release/bundle/macos/媒材工坊.app/Contents/MacOS/mediadesk-desktop`，日志输出随机本地端口；
- CoreGraphics 检测到可见窗口：**1320 × 901**，标题「媒材工坊」；
- 该窗口截图（`docs/screenshots/native.png`）显示实际 WebView 页面；
- 对同一本地端口请求 `/api/health` 返回 `ok: true`；
- 退出应用后进程结束。

---

## 7. 未在本机验证 / 已知限制

1. **Windows 未实机运行**。Windows 的 NSIS 构建、WebView2 加载、`code.cmd` 启动均由同一代码与 CI 配置覆盖，但本环境没有 Windows 机器，因此不声称 Windows 实机通过。CI 工作流也尚未在远端执行过（仓库未推送）。
2. **macOS 仅 arm64 产物**；未构建 Intel 或 universal 包。
3. 未直连图像/视频生成模型；插图通过提示词交给外部图像模型，文卡图可通过 Studio 本地生成。
4. LLM 为非流式；长文生成期间只显示等待态，不显示逐字输出。
5. 无代码签名、公证、自动更新和账号同步；公开分发需自行补证书。
6. 单用户本地应用；没有多人协作与冲突合并。
7. 内置模板只在工作区首次初始化时播种；删除后不会自动恢复。

---

## 8. 复现命令

```bash
npm ci
npm run check                          # typecheck + vitest + cargo test
cargo clippy -p mediadesk-core -p mediadesk-server --all-targets -- -D warnings
npm run e2e                            # 构建 UI/debug server，再跑 3 个 Playwright 用例
npm run build
cargo build --release -p mediadesk-server
npx tauri build --bundles dmg,app
```

截图目录：`docs/screenshots/`（dashboard / ideas / project / generate / card-studio / library / settings / dark / mobile / native）。
