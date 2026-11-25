# 会话记录：CLI 内核分流 + 媒体生成模型 + 项目助手（2026-11-22）

> 本文档记录本会话的**目标、决策、调研结论与实施状态**。
> **实施状态（2026-11-23）：三期功能已全部落地。** 本文保留原始调研与计划；
> 当前代码、命令和验收结果以 [README](../../README.md) 与
> [验收记录](../ACCEPTANCE.md) 为准。
>
> 已实现：`core/src/cli.rs`、`core/src/media_gen.rs`、`core/src/assistant.rs`；
> 设置页文本内核/媒体生成；项目页助手会话；mock CLI、媒体 mock 与 Playwright 用例。

---

## 1. 需求目标

用户最终需求，拆成三件事：

1. **文本类生成走本地 CLI 内核**：生成「文章 / 脚本 / 文卡图」时，调用本机的
   CLI 工具（`claude` / `deepseek` / `opencode`），而不是走 HTTP 大模型 API。
2. **插图 / 视频走常规生成模型**：生成「插图 / 视频」时，调用图像 / 视频生成
   模型的 HTTP API（如 seedance、即梦，或 OpenAI 风格接口）。
3. **项目内新增「助手」会话**：在项目页提供「助手」入口，点击打开右侧会话栏，
   可以就媒体内容与用户交流并**改动素材**；助手同样走 CLI 内核。

约束：

- CLI 子进程必须在**锁定的项目目录内执行**（cwd = 项目目录），沿用现有路径
  防逃逸逻辑（`store::ensure_inside` / `resolve_existing`）。
- 后端为纯 HTTP/JSON（无 WebSocket / SSE / Tauri command），错误格式
  `{error:{code,message,details}}`，错误码沿用现有风格（如 `CLI_NOT_FOUND`、
  `MEDIA_GEN_FAILED`）。
- 配置存工作区 `.mediadesk/config.json`（`store::load_config` / `save_config`）。

## 2. 用户已确认的三项决策

| # | 议题 | 决策 |
|---|------|------|
| ① | 本机没装任何 CLI | **自动探测 + 未找到时明确报错并给安装指引**（`CLI_NOT_FOUND`），先用 **mock CLI** 写链路和测试 |
| ② | 插图/视频接入方式 | **通用可配置 HTTP 接入**（`base_url` + `api_key` + `model`），兼容 seedance / 即梦 / OpenAI 风格 |
| ③ | 助手改动能力 | **右侧会话栏 + 一个开关**，切换「建议式改动」vs「直接覆盖式改动」 |

分期实施：**① CLI 内核 + 文本生成分流 → ② 插图/视频生成模型接入 → ③ 项目助手会话**。

## 2.1 三期落地结果（2026-11-23）

| 期 | 状态 | 落地内容 |
|---|---|---|
| ① CLI 内核 + 文本分流 | 完成 | PATH 探测、Windows `.cmd` 包装、stdin/`{prompt}`、超时与结构化错误；article/script/card 走所选内核；API 为默认兼容路径 |
| ② 插图/视频媒体生成 | 完成 | 图像 OpenAI Images 兼容；视频 `openai` / `task` / `seedance` 协议；URL/base64 解析、轮询、落盘、sidecar、提示词快照 |
| ③ 项目助手 | 完成 | 会话历史持久化、建议/覆盖模式、JSON 编辑协议、复用 `assets::update_asset`、项目页右侧会话栏 |

验证命令：`npm run check`、`cargo clippy --workspace --all-targets --exclude mediadesk-desktop -- -D warnings`、`npm run e2e`。当前测试规模：Rust 15 unit + 20 integration，Vitest 19，Playwright 15。

## 3. 本会话已完成的部分

### 3.1 环境修复（已完成，工程可运行）

- **磁盘清理**：C 盘从 0 可用清理到 2.56GB。
- **仓库迁移**：仓库从 `C:\...` 迁至 `D:\worktrees\glowing-wolf`
  （主仓库 `E:/code_project/softDev/MediaPlatform`）。步骤：robocopy 复制 →
  `git worktree repair` → `session_move` 切会话。C 盘旧副本已删（556MB），
  仅剩空壳目录待 OpenCode 重启后删除。
- **gcc 根因定位**：PATH 中 Anaconda 的 `E:\code_software\anaconda\Library\mingw-w64\bin`
  （含 2016 年 `libwinpthread-1.dll`）排在 `D:\msys64\mingw64\bin` 之前，导致
  `cc1.exe` 报 `0xC0000135` 静默失败。
  **绕过方式**：启动前把 `D:\msys64\mingw64\bin` 插到 PATH 最前。
- **构建链路**：`npm run build`（必须先执行，否则 `core/src/ui.rs:8` 的
  `include_dir!("$CARGO_MANIFEST_DIR/../dist")` 编译报错）→ `npm run dev` 可编译运行。
- 前端 dev：Vite `:5173`，后端 API `:4317`（`MEDIADESK_PORT`），访问
  http://127.0.0.1:5173。

### 3.2 架构调研（已完成，实施前必读）

后端生成链路与改造点：

| 文件 | 关键内容 |
|------|----------|
| `core/src/llm.rs` | `build_prompt()`（项目+灵感+预设+技能 → 提示词）、`chat_completion()`（OpenAI 兼容 HTTP 调用）、`generate()`（总入口，**按 kind 分流的改造点**）、`test_connection()`、`preview()` |
| `core/src/models.rs` | `Config { version, llm, appearance }`、`LlmConfig`、`GenerateRequest`、`GenerateResult`、`ASSET_KINDS`（`article/script/illustration/card/video`）、`kind_label()`；新增配置字段处（需 `#[serde(default)]` 保证向后兼容） |
| `core/src/api.rs` | 路由表（96–138 行）、`put_config` 校验（215 行）、`AppState { workspace, state_dir, code_bin, client }`、`generate`/`preview_prompt`/`test_llm` handler |
| `core/src/context.rs` | `find_code_bin()`（374 行，PATH 探测先例）、`launch()`（419 行，子进程 + Windows `cmd /C` 包装 `.cmd/.bat` 的先例）—— **CLI 探测与启动直接参照这两个函数** |
| `core/src/store.rs` | `load_config`/`save_config`、`ensure_inside`/`resolve_existing` 路径防逃逸、`require_project`、`ProjectRecord { dir_name, dir_path, meta, brief }`、`resolve_public_file`（仅 `projects/` 下非隐藏文件可被浏览器访问） |
| `core/src/assets.rs` | `is_media_kind()`（illustration/card/video）、`allowed_exts()`、`plan_upload()`/`finish_upload()`（**媒体文件落盘复用这套**）、`create_asset()` |
| `core/src/error.rs` | `ApiError { status, code, message, details }`、`bad_request/not_found/forbidden/internal/with_details` |
| `core/src/lib.rs` | 模块声明（新增 `cli.rs` 需在此注册） |

测试基建：`core/tests/api.rs` 有 `TestServer`（临时 workspace + state dir）、
`mock_llm()`（axum mock OpenAI 服务，138 行）、`walk_files()`；
CLI mock 测试可参照 `mock_llm` 的写法。

前端改造点：

| 文件 | 关键内容 |
|------|----------|
| `src/components/GenerateDialog.tsx` | 生成弹窗（全文 461 行）：kind 选择、预设/技能、`api.generate()`、结果确认后 `api.createAsset()` 保存；**分流后需显示内核来源** |
| `src/pages/SettingsPage.tsx` | 设置页（392 行）：`PROVIDERS` 预设、`api.saveConfig()`/`api.testLlm()`；**新增内核 / 媒体生成配置区块** |
| `src/pages/ProjectPage.tsx` | 项目页（597 行）：挂载 `GenerateDialog`；**新增「助手」按钮 + 右侧会话栏** |
| `src/lib/api.ts` / `src/lib/types.ts` | 前端 API 封装与类型定义 |
| `src/components/ui.tsx` | 组件库：`Button/IconButton/Badge/Spinner/Field/TagInput/Modal/ConfirmDialog/EmptyState/Segmented/InlineError/Skeleton` |
| `src/lib/app-context.ts` | `useApp()`：`data.config`、`refreshConfig()`、`toast()` 等 |

其他要点：

- 后端依赖已含 tokio `process` 特性（workspace `Cargo.toml`），可直接用
  `tokio::process::Command`。
- 现有「助手」概念仅存在于 `llm.rs` 系统提示词文案，**无多轮会话功能，
  助手会话需从零新建**。

## 4. 未完成 / 待决事项

- **三期功能代码完全未写**（0 行功能代码；工作区仅有 `package.json` /
  `package-lock.json` 的 vitest 版本变更等未提交改动）。
- **全局 PATH 修复方案待用户选择**：
  - A：调整顺序（把 `D:\msys64\mingw64\bin` 提到 Anaconda mingw 之前）
  - B：移除 Anaconda 的 mingw 路径
  - C：只改 `scripts/dev.mjs`（仅本项目启动时注入）
- 调研时本机无 `claude`/`opencode`/`deepseek` 可执行文件；实施验收时检测到 Claude Code 2.1.181，但真实生成仍使用 mock CLI，避免消耗账号额度。

## 5. 下一步实施计划（三期）

### 第①期：CLI 内核 + 文本生成分流

1. 新增 `core/src/cli.rs`，并在 `lib.rs` 注册：
   - PATH 探测（参照 `context::find_code_bin`，Windows 需处理 `.cmd/.bat`
     的 `cmd /C` 包装）；
   - `tokio::process::Command` 子进程：stdin 传 prompt、stdout 取结果、
     cwd 锁定项目目录、超时控制、非零退出 → 结构化报错；
   - 找不到可执行文件 → `CLI_NOT_FOUND`，details 附安装指引；
2. `models.rs`：`Config` 新增 `text_kernel`（`"api" | "claude" | "opencode" |
   "deepseek"`）与 CLI 配置字段（全部 `#[serde(default)]` 向后兼容）；
3. `llm.rs::generate` 按 `kind` 分流：`article/script/card` → CLI 内核，
   `illustration/video` → 第②期的媒体生成（本期仍走原 HTTP）；
4. 测试：mock CLI 脚本 + `CLI_NOT_FOUND` 分支。

### 第②期：插图 / 视频生成模型接入

1. `models.rs` 新增 `media_gen` 配置（`base_url` / `api_key` / `model` /
   `style`，兼容 OpenAI 风格与任务式异步风格）；
2. 新增媒体生成模块：`illustration` → 图像 API，`video` → 视频 API
   （如 seedance 的任务式接口，需轮询）；
3. 产物落项目 `illustration/`、`video/` 目录，复用 `assets::plan_upload` /
   `finish_upload`；错误码 `MEDIA_GEN_FAILED`；
4. `SettingsPage` 加媒体生成配置区块。

### 第③期：项目助手会话

1. `POST /api/projects/{id}/chat` 助手会话；历史存
   `projects/<dir>/.mediadesk/assistant/`（注意 `resolve_public_file` 不会
   服务隐藏目录，历史不外泄）；
2. 前端 `ProjectPage` 加「助手」按钮 + 右侧会话栏 + 「建议式 / 覆盖式」开关；
3. 助手调用同样按第①期的内核分流；改动素材时复用 `assets::update_asset`。

**每期完成后跑 `npm run check`**（typecheck + vitest + `cargo test --workspace
--exclude mediadesk-desktop`）。

## 6. 常用命令备忘

```powershell
# 编译前端（cargo 之前必须执行，否则 include_dir! 报错）
npm run build

# 开发启动（需先注入 MSYS2 gcc 到 PATH 最前）
$env:PATH = "D:\msys64\mingw64\bin;" + $env:PATH
npm run dev            # Vite :5173 + 后端 :4317

# 全量检查
npm run check          # typecheck && vitest && cargo test
```
