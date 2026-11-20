# 媒材工坊 MediaDesk 实施计划

> **For agentic workers:** 本计划按任务顺序执行。用户已明确要求「不要提问，直接开干」，因此采用本会话内直接实现（native execution），每个任务以可运行测试收口。

**Goal:** 交付可跨 Windows/macOS 使用的自媒体素材管理应用：浏览器版完整可用，macOS 原生包可启动，Windows 构建配置就绪。

**Architecture:** Rust + Axum 核心负责文件系统、LLM 代理、Prompt 编译与 VSCode 工程生成，前端 Preact SPA 只通过同源 HTTP 访问；`mediadesk-server` 与 Tauri 壳复用同一核心和内嵌 UI。

**Tech Stack:** Rust 1.97 / Axum 0.8 / Tauri 2 / Preact 10 / TypeScript / Vite / Playwright / Vitest

**Spec:** `docs/ARCHITECTURE.md`

## Global Constraints

- 服务仅绑定 `127.0.0.1`；无遥测、无 CDN、无外部字体。
- 所有工作区读写走 `core::store`；禁止路径逃逸；删除进 `.mediadesk/trash/`。
- 单用户、文件系统为源，不引入数据库。
- 前端 gzip < 250KB；安装包目标 < 20MB。
- 密钥永不明文返回前端；Markdown 预览必须清洗。
- 五项核心能力（速记、立项、五类素材、技能/预设+LLM、VSCode 快捷方式）必须有自动化测试。
- 用户界面中文；代码标识英文。
- 每个任务完成后提交 Git；最终以测试输出与截图验收。

## Review Focus

- **空工作区首次启动**：必须自动初始化技能/预设/目录，界面不能白屏或报错。
- **中文/空格/特殊字符路径**：项目名与文件名含中文或空格时，文件 URL、上传、VSCode 路径必须正常。
- **外部文件混入**：用户手动往素材目录放图片/视频时，刷新后应被收养为素材而非消失或报错。
- **LLM 未配置/超时/401**：必须给出可操作错误与提示词复制降级，不得假成功。
- **VSCode CLI 不存在**：返回手动命令，界面明确失败，不得静默。
- **删除后误操作**：素材/项目可恢复（回收站），界面有确认。
- **路径穿越**：`../`、绝对路径、编码变体访问工作区外文件必须 403/404。

---

## Task 1: 项目骨架与共享类型

**Files:** `package.json`, `tsconfig.json`, `vite.config.ts`, `Cargo.toml`, `core/Cargo.toml`, `core/src/lib.rs`, `core/src/models.rs`, `core/src/error.rs`

- [ ] 初始化 Vite+Preact+TS 与 Cargo workspace。
- [ ] 定义实体、错误响应、状态枚举（与架构文档一致，camelCase）。
- [ ] `cargo check` 与 `npm run build` 通过（先放占位 UI）。

## Task 2: 工作区存储层（含测试）

**Files:** `core/src/frontmatter.rs`, `core/src/store.rs`, `core/src/seed.rs`, `core/tests/store.rs`

- [ ] 测试：初始化生成 `.mediadesk/{config.json,skills,presets}`、`ideas/`、`projects/`。
- [ ] 测试：frontmatter 中文往返、正文含 `---` 不截断。
- [ ] 实现原子写、slug、ULID 命名、安全 join、目录扫描、回收站。
- [ ] 测试：`safe_join` 拒绝 `..`、绝对路径、符号链接逃逸。

## Task 3: 想法与项目 CRUD / 立项

**Files:** `core/src/store.rs`, `core/src/api.rs`, `core/tests/api.rs`

- [ ] 集成测试：创建/读取/更新/删除想法；删除后列表消失且回收站存在文件。
- [ ] 集成测试：单想法 promote 与多想法建项目；想法的 `status=promoted`、`projectId` 正确。
- [ ] 实现 API 与项目目录、`project.md`。

## Task 4: 五类素材、上传与收养

**Files:** `core/src/store.rs`, `core/src/api.rs`, `core/tests/api.rs`

- [ ] 测试：五种 kind 创建/更新/删除；非法 kind 400。
- [ ] 测试：上传 PNG 生成媒体文件 + sidecar，`fileUrl` 可 GET，大小/类型正确。
- [ ] 测试：外部放入的 JPG 在 rescan 后成为素材；中文文件名可访问。
- [ ] 测试：媒体 Range 请求返回 206（视频拖动）。

## Task 5: 技能、预设与 Prompt 编译

**Files:** `core/src/seed.rs`, `core/src/llm.rs`, `core/tests/llm.rs`

- [ ] 测试：内置技能/预设可列出；新建/编辑/删除。
- [ ] 测试：Prompt 包含项目简介、关联想法、预设、所选技能、类型约束。
- [ ] 测试：未配置密钥返回 `LLM_NOT_CONFIGURED`；dry-run 不调用网络。
- [ ] 测试：mock OpenAI 服务成功返回内容、用量与提示词快照；401/超时映射为 `LLM_REQUEST_FAILED`。

## Task 6: VSCode 工程生成与启动

**Files:** `core/src/context.rs`, `core/src/api.rs`, `core/tests/context.rs`

- [ ] 测试：生成 `AGENTS.md`、`CLAUDE.md`、`README.md`、`.vscode/tasks.json`、`skills/`、`presets/`、`prompts/README.md`、`context.json`，内容含项目标题与已选技能。
- [ ] 测试：`MEDIADESK_CODE_BIN=/usr/bin/true` 时返回成功；不存在时返回 `VSCODE_NOT_FOUND` 与手动命令。
- [ ] 实现跨平台 CLI 探测（PATH、macOS 默认路径、Windows code.cmd）。

## Task 7: 前端设计系统与页面

**Files:** `src/styles.css`, `src/lib/*`, `src/components/*`, `src/pages/*`, `src/main.tsx`

- [ ] 布局/侧栏/主题/Toast/Modal/确认框/空状态/骨架屏。
- [ ] 工作台：速记、统计、最近项目与灵感。
- [ ] 灵感页：筛选、标签、编辑、立项。
- [ ] 项目页：五类标签、素材卡片、编辑器、上传、图片灯箱、视频播放。
- [ ] 生成弹层：预设/技能/指令/预览/保存/复制提示词。
- [ ] 文卡图 Studio：主题/比例/字号、Canvas 导出 PNG 并保存。
- [ ] 技能与预设库、设置（工作区选择、LLM 测试）、⌘K 搜索、⌘N 速记。
- [ ] 响应式与暗色主题；Playwright 截图后视觉迭代。

## Task 8: 前端单测与 E2E 实测

**Files:** `tests/e2e/*.spec.ts`, `playwright.config.ts`, `src/lib/*.test.ts`, `scripts/e2e-server.mjs`, `scripts/mock-llm.mjs`, `scripts/fake-code.sh`

- [ ] Vitest：格式化、错误映射、Markdown 清洗。
- [ ] E2E：速记→立项→手动素材→上传→生成（mock LLM）→保存→VSCode（fake CLI）→文卡图→技能→搜索→主题刷新。
- [ ] 验证工作区磁盘文件、VSCode 上下文文件、fake CLI 调用日志。
- [ ] 收集无 console error；输出截图到 `artifacts/`。

## Task 9: 浏览器单二进制构建

**Files:** `server/Cargo.toml`, `server/src/main.rs`, `core/src/ui.rs`, `scripts/dev.mjs`

- [ ] `npm run build` 后 Rust 内嵌 `dist/`；单二进制启动并自动打开浏览器。
- [ ] 实测 `/`、`/api/health`、`/files/...`、History 路由回退。
- [ ] `scripts/dev.mjs` 同时启动 Rust API + Vite，代理 `/api`、`/files`。

## Task 10: Tauri 打包与 Windows CI

**Files:** `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/icons/*`, `.github/workflows/build.yml`

- [ ] Tauri 启动时绑定随机本地端口、拉起 core、创建窗口。
- [ ] 生成图标，构建 macOS `.app` / `.dmg`，实际启动并用 Playwright/截图验证。
- [ ] GitHub Actions：`macos-latest` + `windows-latest` 构建 bundle artifacts。
- [ ] 体积与空载内存实测记录。

## Task 11: 文档与最终验收

**Files:** `README.md`, `docs/ACCEPTANCE.md`

- [ ] README：功能、截图、开发/构建、数据位置、Windows 说明、隐私。
- [ ] 验收记录：命令输出、测试数量、产物路径、体积、已知限制（Windows 未实机）。
- [ ] `cargo fmt`、`cargo clippy`、`cargo test`、`npm test`、`npm run build`、E2E、Tauri 包全部复跑。
