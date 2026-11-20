# 媒材工坊 MediaDesk · 架构设计

> 面向自媒体创作者的个人灵感与素材工作台。浏览器优先，桌面原生收口。
> 版本：v0.1 · 2026-11-20

---

## 1. 产品定义

**一句话**：把「随手想法」变成「可发布素材」的本地工作台。

三条主线：

1. **灵感不丢**：全局快捷速记，想法有独立文件、标签、状态。
2. **从想法到项目**：立项后项目单独成页，按「文章 / 脚本 / 插图 / 文卡图 / 视频」五类管理素材。
3. **人机协作可复用**：技能（Skill）与预设（Preset）沉淀创作方法；手动调用大模型；一键生成带上下文的 VSCode 工程，让外部 AI 工具可读、可写、可继续创作。

**边界（v0.1 不做）**：多端云同步、团队协作、账号体系、自动发布到平台、原生图像生成、视频剪辑。所有数据保存在本地工作区，可被 Git / 任意编辑器直接管理。

---

## 2. 关键决策

| 决策点 | 方案 | 理由 |
|---|---|---|
| 前端框架 | **Preact 10 + TypeScript + Vite** | 相比 React 运行时更小（~4KB gzip），API 兼容；无重型组件库，自建轻量设计系统 |
| 后端 | **Rust + Axum** | 单二进制、常驻内存低、启动快；跨平台一致；原生文件系统与进程能力 |
| 浏览器模式 | 本地 `mediadesk-server`（127.0.0.1）+ 系统浏览器 | 满足「浏览器优先」，无需安装 Electron；开发期热更新走 Vite |
| 桌面模式 | **Tauri 2** | 使用系统 WebView（WKWebView / WebView2），安装包通常 < 15MB，内存远低于 Electron |
| 逻辑复用 | Rust `core` crate 同时被 server 与 Tauri 引用；前端资产内嵌进二进制 | 两种形态只有一层壳不同，业务逻辑零重复 |
| 数据存储 | **文件系统即数据库**（Markdown + YAML frontmatter + 媒体文件） | 对人和大模型天然友好；无数据库依赖、安装包小、可直接 Git 版本管理 |
| 索引 | 请求时扫描工作区（个人级数据量）+ 轻量内存缓存 | 避免 SQLite 原生依赖；数据规模通常 < 几千文件，扫描毫秒级 |
| 大模型 | OpenAI-compatible Chat Completions，本地代理 | 兼容 DeepSeek / OpenAI / Moonshot / Ollama 等；密钥不进前端、不受浏览器 CORS 限制 |
| 流式输出 | v0.1 非流式（可配置超时），UI 展示等待态 | 降低错误面，先保证可靠保存；v2 再加 SSE 流式 |
| 主题 | 自建 CSS 变量 + 明暗双主题 | 体积小、视觉可控、不依赖 UI 框架 |
| 分发 | 浏览器单二进制 / macOS .app+.dmg / Windows NSIS+MSI（CI） | 覆盖 Windows 与 macOS；Windows 可先用浏览器版，零安装 |

### 体积与内存预算

| 指标 | 目标 | 说明 |
|---|---|---|
| 前端产物 | JS+CSS < 250KB gzip | Preact + marked + DOMPurify + 自写 UI |
| 桌面安装包 | < 20MB | Tauri + 系统 WebView；无 Chromium 运行时 |
| 常驻后端内存 | < 40MB | 单用户、无数据库、无后台轮询 |
| 冷启动 | < 500ms（server）/ < 2s（桌面窗口） | 无迁移、无重型初始化 |
| 网络 | 仅用户主动调用 LLM 时产生 | 无遥测、无自动更新请求 |

---

## 3. 系统架构

```
┌─────────────────────────────── WebView / Browser ───────────────────────────────┐
│  Preact SPA                                                                     │
│  ┌────────────┬────────────┬────────────┬────────────┬───────────────────────┐  │
│  │ 工作台      │ 灵感收件箱  │ 项目/素材   │ 技能与预设  │ 设置 / 搜索 / 命令面板  │  │
│  └────────────┴────────────┴────────────┴────────────┴───────────────────────┘  │
│  自研 Router · API Client · Markdown Preview · Card Studio(Canvas) · Lightbox    │
└───────────────────────────────────┬─────────────────────────────────────────────┘
                                    │ same-origin HTTP (JSON / raw / range)
┌───────────────────────────────────▼─────────────────────────────────────────────┐
│  mediadesk-core (Rust + Axum)                                                   │
│  REST API · 工作区扫描/原子写入 · 路径安全 · Prompt 编译器 · LLM 代理             │
│  VSCode 工程生成器 · code 启动器 · 内嵌 SPA 静态资源                              │
└───────┬──────────────────────────────┬─────────────────────────┬────────────────┘
        │                              │                         │
┌───────▼────────┐           ┌─────────▼─────────┐      ┌────────▼────────┐
│ Workspace 文件  │           │ OpenAI-compatible │      │ VS Code CLI     │
│ Markdown/媒体   │           │ LLM API (可选)     │      │ code <project>  │
└────────────────┘           └───────────────────┘      └─────────────────┘

两种运行形态：
  ① mediadesk-server  → 127.0.0.1:4317 → 系统浏览器   （Windows / macOS / Linux 通用）
  ② Tauri 壳          → 启动同一 core，随机本地端口 → 原生窗口（WKWebView / WebView2）
```

### 代码结构

```
MediaPlatform/
├── core/                    # 可复用 Rust 核心：模型、存储、API、Prompt、VSCode
│   ├── src/models.rs        # Idea / Project / Asset / Skill / Preset / Config
│   ├── src/frontmatter.rs   # YAML frontmatter 解析与序列化
│   ├── src/store.rs         # Workspace 初始化、扫描、CRUD、回收站、上传
│   ├── src/llm.rs           # Prompt 编译 + OpenAI-compatible 调用
│   ├── src/context.rs       # VSCode 工程与 AI 上下文文件生成
│   ├── src/api.rs           # Axum 路由与 HTTP 层
│   ├── src/seed.rs          # 内置技能/预设/说明书
│   └── tests/api.rs         # 真实 HTTP 集成测试（temp workspace）
├── server/                  # 浏览器模式单二进制：内嵌前端 + 打开浏览器
├── src-tauri/               # 桌面壳：启动核心服务 + 原生窗口 + 打包配置
│   ├── src/main.rs
│   ├── tauri.conf.json
│   └── icons/
├── src/                     # Preact 前端
│   ├── components/          # 通用 UI、布局、编辑器、弹层
│   ├── pages/               # Dashboard / Ideas / Projects / Project / Library / Settings
│   ├── lib/                 # api、router、format、markdown、canvas
│   ├── styles.css           # 设计系统（明/暗）
│   └── main.tsx
├── scripts/                 # dev / e2e / mock LLM / fake code / release
├── tests/e2e/               # Playwright 端到端
├── docs/                    # 架构、实施计划、验收记录
└── .github/workflows/       # Windows / macOS 双平台构建
```

依赖方向严格单向：`web → HTTP → core ← server / tauri`。前端不感知运行形态。

---

## 4. 工作区数据模型

用户选择一个工作区目录（默认 `~/Documents/MediaDesk`），全部数据为普通文件：

```
<Workspace>/
├── .mediadesk/
│   ├── config.json              # LLM 配置（密钥仅本地）、schemaVersion
│   ├── skills/*.md              # 技能库（YAML frontmatter + 方法正文）
│   ├── presets/*.md             # 创作预设（按素材类型）
│   ├── seed.json                # 内置模板播种标记
│   └── trash/                   # 删除 = 可恢复回收站
├── ideas/
│   └── 01JXXXX-想法标题.md       # 想法，frontmatter + 正文
└── projects/
    └── 我的项目/
        ├── project.md            # 项目 frontmatter + 简介
        ├── article/              # 文章（.md）
        ├── script/               # 脚本（.md）
        ├── illustration/         # 插图（图片 + 同名 sidecar .md）
        ├── card/                 # 文卡图（PNG + sidecar .md）
        ├── video/                # 视频（mp4/webm/mov + sidecar .md）
        ├── prompts/              # 每次 LLM 生成的提示词快照
        ├── skills/               # 本项目加载的技能副本（供外部 AI 读取）
        ├── presets/              # 本项目启用的预设副本
        ├── AGENTS.md             # AI 协作总说明书
        ├── CLAUDE.md             # Claude Code / 同类工具入口（内容同源）
        ├── README.md             # 人类可读项目索引
        ├── .mediadesk/context.json # 机器可读上下文
        └── .vscode/              # settings / tasks / extensions
```

**设计原则**

- 文件名稳定：重命名只改 frontmatter 的 `title`，不移动文件，避免外链与 Git 噪音。
- 月名 ID：使用 ULID（时间有序、URL 安全），文件名 `<ULID>-<slug>.md`。
- 媒体 sidecar：任何二进制素材都有同名 `.md`，记录标题、状态、提示词、备注；删除 sidecar 不会删除媒体，扫描时会自动收养（adopt）并重建元数据。
- 原子写：先写 `.tmp`，`fsync` 后 rename，避免断电/崩溃产生半文件。
- 路径安全：所有 API 路径经过 canonicalize + 前缀校验，拒绝 `..`、绝对路径逃逸和符号链接逃逸。
- 删除进 `.mediadesk/trash/`，不做硬删除。

### 实体

```ts
Idea {
  id: string; title: string; content: string;
  status: 'inbox' | 'promoted' | 'archived';
  tags: string[]; createdAt: string; updatedAt: string;
  projectId?: string;
}

Project {
  id: string; title: string; brief: string;
  status: 'active' | 'paused' | 'done' | 'archived';
  tags: string[]; ideaIds: string[]; skillIds: string[]; presetIds: string[];
  createdAt: string; updatedAt: string;
  dirName: string;                      // projects/ 下的真实目录名
  assets: Record<AssetKind, Asset[]>;   // 服务端分组返回
  counts: Record<AssetKind, number>;
  cover?: string;                       // 首个可用图片的 URL
}

Asset {
  id: string; kind: 'article'|'script'|'illustration'|'card'|'video';
  title: string; content: string;       // Markdown 正文 / 媒体备注
  status: 'draft' | 'review' | 'ready' | 'published';
  tags: string[]; createdAt: string; updatedAt: string;
  file?: string;                        // 媒体相对文件名（不含目录）
  fileUrl?: string;                     // 服务端计算
  prompt?: string; source?: 'manual' | 'llm';
  projectId: string;
}

Skill  { id; name; description; appliesTo: AssetKind[]; content }
Preset { id; name; description; kind: AssetKind; skillIds: string[]; content }
```

---

## 5. HTTP API

全部返回 JSON；错误统一 `{ error: { code, message, details? } }`。

| 方法 | 路径 | 作用 |
|---|---|---|
| GET | `/api/health` | 健康检查 / 版本 |
| GET | `/api/workspace` | 当前工作区、统计、磁盘占用 |
| POST | `/api/workspace` | 切换并初始化工作区 |
| GET | `/api/fs/browse?path=` | 目录浏览器（浏览器模式选文件夹） |
| POST | `/api/workspace/reveal` | 在 Finder / 资源管理器打开 |
| GET/PUT | `/api/config` | 读取（密钥脱敏）/ 更新 LLM 配置 |
| POST | `/api/llm/test` | 真实调用一次最小请求验证连通性 |
| POST | `/api/llm/prompt` | 仅编译并返回提示词（不调用模型） |
| POST | `/api/llm/generate` | 手动生成；返回内容、用量、提示词、快照路径 |
| GET/POST | `/api/skills` | 技能列表 / 新建 |
| PUT/DELETE | `/api/skills/:id` | 编辑 / 删除技能 |
| GET/POST | `/api/presets` | 预设列表 / 新建 |
| PUT/DELETE | `/api/presets/:id` | 编辑 / 删除预设 |
| GET/POST | `/api/ideas` | 想法列表（含正文）/ 新建 |
| GET/PUT/DELETE | `/api/ideas/:id` | 详情 / 更新 / 移入回收站 |
| POST | `/api/ideas/:id/promote` | 从想法立项 |
| GET/POST | `/api/projects` | 项目列表 / 新建（可关联多个想法） |
| GET/PUT/DELETE | `/api/projects/:id` | 详情（素材分组）/ 更新 / 回收站 |
| POST | `/api/projects/:id/vscode` | 生成 AI 上下文文件并启动 VS Code |
| POST | `/api/projects/:id/assets` | 新建素材 |
| PUT/DELETE | `/api/projects/:id/assets/:assetId` | 编辑 / 删除素材 |
| POST | `/api/projects/:id/uploads?kind=&assetId=&filename=` | 原始二进制上传（图片/视频） |
| POST | `/api/projects/:id/rescan` | 收养外部放入的媒体文件 |
| GET | `/api/search?q=` | 跨想法/项目/素材搜索 |
| GET | `/files/*` | 工作区原始文件（Range 支持，视频可拖动） |
| GET | `/*` | 内嵌 SPA（History 路由回退） |

**关键错误码**：`WORKSPACE_INVALID`、`NOT_FOUND`、`INVALID_INPUT`、`PATH_FORBIDDEN`、`LLM_NOT_CONFIGURED`、`LLM_REQUEST_FAILED`、`VSCODE_NOT_FOUND`。

---

## 6. LLM 与 Prompt 编译

一次生成由四层拼装，全部可追溯：

```
系统角色（固定）
  + 项目上下文（标题、简介、关联想法正文、已有素材标题）
  + 预设 Preset（创作方法、结构、输出规范）
  + 技能 Skills（可叠加的方法论）
  + 本次指令（用户填写）+ 输出格式约束（按素材类型）
```

- 素材类型对应不同默认输出：文章=长文 Markdown；脚本=口播/分镜；插图=3 组中英双语图像提示词 + 负面词；文卡图=多组短文案；视频=分镜表格 + 口播稿 + 标题封面建议。
- 生成成功后将完整提示词写入 `projects/<dir>/prompts/<timestamp>-<kind>.md`，正文结果由用户确认后再落库，避免污染素材库。
- 支持「只编译提示词 → 复制到任意模型」作为无密钥/离线场景的完整降级路径。
- 密钥只保存在 `.mediadesk/config.json`（Unix 权限 0600），GET 接口永不明文返回；所有请求由 Rust 后端发出。

---

## 7. VSCode 协作工程

点击「在 VSCode 打开」：

1. 服务端刷新项目上下文文件（幂等覆盖生成区，保留用户素材）。
2. 复制本项目启用的技能与预设到 `skills/`、`presets/`。
3. 写入 `AGENTS.md` / `CLAUDE.md` / `README.md` / `.mediadesk/context.json` / `.vscode/*`。
4. 按平台解析 `code` CLI（PATH → macOS 默认路径 → Windows `code.cmd`），`spawn` 且不阻塞。
5. 失败时返回可复制的手动命令，UI 降级提示，不伪装成功。

`AGENTS.md` 明确告诉外部 AI：素材目录、frontmatter 字段、如何读取技能/预设、修改规范、禁止删除 sidecar 等。

---

## 8. UI / UX

**视觉方向**：编辑部工作台。暖纸色背景、墨色文字、朱砂强调色、宋体标题 + 系统无衬线正文。克制阴影与圆角，像一本会呼吸的工作手册，而不是 SaaS 仪表盘。

- 布局：左侧 248px 导航，主区最大 1280px；项目页为编辑式抬头 + 类型标签页。
- 核心动线：`⌘N 速记` → 灵感卡片「立项」→ 项目页「生成素材」→ 预览确认「保存」→「VSCode 打开」。
- 全局：`⌘K` 命令面板搜索想法/项目/素材；`ESC` 关闭弹层；focus ring；`prefers-reduced-motion`。
- 卡片 / 空状态 / 骨架屏 / Toast / 确认对话框全部自绘，统一设计 token。
- 明暗主题持久化到 localStorage，跟随系统。
- 移动端（< 900px）侧栏变抽屉，编辑器单栏，操作按钮收纳。

---

## 9. 安全与隐私

- 服务只绑定 `127.0.0.1`，不监听局域网；无遥测、无外部字体、无 CDN。
- 路径校验在服务端统一入口执行；上传扩展名白名单；大小上限默认 2GB。
- 前端 Markdown 预览使用 DOMPurify 清洗，禁止脚本/事件属性。
- API 密钥仅本地存储、脱敏返回；错误信息中剔除密钥；Tauri 不向前端暴露文件系统原生权限（走本机 HTTP）。
- 外部 LLM 仅收到用户主动提交的上下文，UI 在生成前展示「将发送的提示词」。

---

## 10. 测试与验收

| 层级 | 工具 | 覆盖 |
|---|---|---|
| Rust 单元/集成 | `cargo test` + tempdir + 本地 mock LLM | 工作区初始化、frontmatter、CRUD、立项、回收站、上传、路径穿越、Prompt、LLM 成功/失败、VSCode 生成与假 CLI 启动、搜索 |
| 前端单测 | Vitest | API 错误映射、格式化、Markdown 清洗等纯函数 |
| 端到端 | Playwright（真实 Chromium + 真实 Rust 服务） | 速记→立项→素材→编辑→上传→生成→保存→VSCode→技能→文卡图导出→主题→搜索，截图存档 |
| 构建验证 | Vite build + `cargo build --release` + Tauri bundle | 产物可启动、静态资源 200、API 200、安装包存在 |
| 跨平台 | macOS 本机构建；Windows GitHub Actions | macOS 实机验证；Windows 通过同一 Tauri 配置 + CI 构建，浏览器版 Windows 可直接运行 |

**拒绝幻觉的规则**：所有「已完成」以命令输出、测试报告、截图和实际启动为准；Windows 本机无法实测的部分明确标注为「CI 构建配置已就绪，未在 Windows 实机验收」。

---

## 11. 交付物

- 架构文档 + 实施计划 + 验收记录。
- 浏览器生产版：`dist/` + `mediadesk-server` 单二进制（内嵌 UI），双击/命令行启动。
- macOS `.app` / `.dmg`（本机构建并启动验证）。
- Windows 构建工作流与说明（NSIS/MSI），浏览器版在 Windows 直接可用。
- 内置技能 6 个、预设 5 个、示例工作区初始化器。

## 12. 后续路线（不进 v0.1）

图像模型直连、SSE 流式生成、素材版本历史 diff、全文索引（tantivy）、拖拽排序、发布日历、Git 快照、移动端 PWA 离线缓存、插件系统。
