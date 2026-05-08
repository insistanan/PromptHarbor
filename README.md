# 提示港 PromptHarbor

提示港是一个面向 Claude Code 和 Codex CLI 的本地提示词工作台。它帮助用户为每个 Agent 会话编写 Markdown 草稿、复制到对应 Agent 客户端中提交，并通过 hooks 记录用户实际发送的 prompt，支持历史回看与搜索。

## 项目状态

当前 MVP 核心闭环已在 Windows 环境完成一轮实机验证，仓库包含 Tauri + React + Rust workspace 的可运行实现。近期代码已完成一轮架构拆分：后端按运行时、hook 适配、采集、存储、检索拆分；前端按会话、草稿、历史、搜索、设置拆分。

已确认的 MVP 目标：

- 同时支持 Claude Code 和 Codex CLI。
- 只记录用户 prompt，不记录模型回复。
- 使用 `UserPromptSubmit` hook 捕获用户实际提交内容。
- 使用 Milkdown 提供 Typora-like Markdown 草稿编辑体验。
- 使用 Tauri + Rust + SQLite 构建本地桌面应用。
- 默认本地优先、不联网、不调用外部大模型 API。
- 支持全局暂停记录。
- 支持系统托盘常驻。
- 支持 prompt 图片附件采集、历史图片读取和缺图提示。
- 支持会话、历史 prompt、当前草稿的全局搜索。

## 开发启动

当前实现包含：

- `crates/promptbox-core`：共享领域类型、运行时配置、hook 适配、SQLite 存储、会话/草稿/历史检索和附件处理。
- `crates/promptbox-hook`：Claude Code / Codex CLI 调用的轻量采集器，支持 `--version`、本地端点投递和 spool fallback。
- `crates/promptbox-app`：Tauri 应用、本地采集端点、采集入库、命令层、托盘、窗口生命周期和开机启动。
- `frontend`：React + Vite + Milkdown 工作区，支持会话浏览、草稿编辑、图片暂存、历史查看、搜索和运行设置。

### 安装前端依赖

```powershell
npm --prefix frontend install
```

### 开发模式（Tauri dev）

`tauri:dev` 只编译 `promptbox-app`，**不会编译 `promptbox-hook`**。首次启动或改了 hook 代码时，需要先单独构建 hook，否则稳定位置没有兼容版本会报错。

```powershell
cargo build -p promptbox-hook
npm --prefix frontend run tauri:dev
```

`tauri:dev` 实际执行 `cargo tauri dev`，会：
1. 启动 Vite 前端开发服务器（`http://localhost:5173`）
2. 编译 `promptbox-app`（debug），产物在 `target/debug/promptbox-app.exe`

> **注意**：`tauri:dev` 只编译 `promptbox-app`，不会编译 `promptbox-hook`。hook 需单独构建。
>
> 启动时会检查稳定位置的 hook 是否就绪。如果 `target/debug/promptbox-hook.exe` 不存在（sibling 查找失败）、且稳定位置也没有兼容版本，会报错。

### Hook 路径

Codex CLI / Claude Code 的 hook 配置中 `command` 指向稳定路径，不能指向 `target/` 临时构建目录。

**稳定位置（hook 运行路径）：**
```
%APPDATA%\PromptBox\bin\promptbox-hook.exe
```
可通过 `PROMPTBOX_HOME` 环境变量覆盖。

**Dev 模式下的 hook 源查找顺序**（`find_hook_source`）：
1. `PROMPTBOX_HOOK_SOURCE` 环境变量 → 直接指向已构建的 hook
2. 打包资源 `resources/promptbox-hook.exe`（dev 模式不存在）
3. 当前进程同目录（sibling 查找）→ `target/debug/promptbox-hook.exe`（需先 `cargo build -p promptbox-hook`）

如果三个源都找不到，但稳定位置已有兼容版本，不会报错也不会覆盖现有文件。

**构建模式（打包发布）**：`resources/promptbox-hook.exe` 随安装包分发，首次启动时复制到稳定位置。

### 更新 hook 可执行文件

改了 hook 代码后：

```powershell
cargo build -p promptbox-hook
npm --prefix frontend run tauri:dev
```

由于 `target/debug/promptbox-hook.exe` 与稳定位置文件内容不同，启动时 `HookBinaryManager` 会自动复制到稳定位置。如果内容相同则跳过。

也可以手动覆盖（无需重启 Tauri，下次 hook 调用时生效）：

```powershell
Copy-Item -Force "target/debug/promptbox-hook.exe" "$env:APPDATA\PromptBox\bin\promptbox-hook.exe"
```

### 打包发布

```powershell
cargo build --release -p promptbox-hook
New-Item -ItemType Directory -Force -Path "crates/promptbox-app/resources" | Out-Null
Copy-Item -Force "target/release/promptbox-hook.exe" "crates/promptbox-app/resources/promptbox-hook.exe"
npm --prefix frontend run tauri:build
```

`tauri.conf.json` 中 `bundle.resources: ["resources/"]` 会将 `resources/` 目录打进安装包。安装后首次启动时自动同步到稳定位置。

### 仅前端开发（不含 Tauri）

```powershell
npm --prefix frontend run dev
```

Vite 开发服务器在 `http://localhost:5173`，纯前端预览，无后端/采集功能。

### 隔离测试 hook 投递

本地采集端点默认监听 `127.0.0.1:9996`。可以设置临时 home 隔离测试：

```powershell
$env:PROMPTBOX_HOME = Join-Path $PWD ".tmp\promptbox-home-issue3"
$env:PROMPTBOX_HOOK_SOURCE = Join-Path $PWD "target\debug\promptbox-hook.exe"
```

hook 的最小输入示例：

```json
{
  "hook_event_name": "UserPromptSubmit",
  "session_id": "demo-session",
  "cwd": "D:\\code\\some\\prompt",
  "prompt": "测试 PromptHarbor 采集链路"
}
```

Windows MVP 实机验证记录见 [MVP 方案设计](docs/promptbox-design.md#23-windows-mvp-实机验证记录)。

## 当前代码结构

后端核心模块：

- `crates/promptbox-core/src/runtime/`：路径、配置、运行状态等本地运行时能力。
- `crates/promptbox-core/src/hook_adapter.rs`：Claude Code 与 Codex CLI 的统一 hook 适配接口。
- `crates/promptbox-core/src/claude/`、`crates/promptbox-core/src/codex/`：各 Agent 客户端的 hook 配置、状态检测和测试。
- `crates/promptbox-core/src/hook_binary/`：hook 可执行文件查找、版本检查、复制和状态报告。
- `crates/promptbox-core/src/event/`：hook 事件规范化、spool fallback 和本地端点配置。
- `crates/promptbox-core/src/store/`：SQLite 表结构、类型、文本工具、会话、草稿、检索和附件。
- `crates/promptbox-core/src/store/attachments/`：图片附件提取、文件保存、历史读取、数据地址解析和缺图判定。
- `crates/promptbox-core/src/store/retrieval/`：历史查询和全局搜索。搜索内部按会话、已发送 prompt、草稿分别查询，再统一排序截断。

Tauri 应用模块：

- `crates/promptbox-app/src/local_http.rs`：本地 HTTP 请求解析。
- `crates/promptbox-app/src/collector.rs`：采集端点、token 校验和暂停判断。
- `crates/promptbox-app/src/ingestion.rs`：采集事件入库。
- `crates/promptbox-app/src/commands/`：状态、会话、草稿、历史、hook、运行时和桌面命令。
- `crates/promptbox-app/src/startup.rs`、`state.rs`、`desktop.rs`、`autostart.rs`：启动、共享状态、窗口/托盘和开机启动。

前端模块：

- `frontend/src/features/app/`：应用轮询状态。
- `frontend/src/features/sessions/`：会话列表、会话详情和历史状态。
- `frontend/src/features/drafts/`：草稿工作区、草稿状态、右键菜单、图片暂存和图片操作。
- `frontend/src/features/history/`：历史 prompt 列表、历史图片读取和图片条展示。
- `frontend/src/features/search/`：全局搜索界面。
- `frontend/src/features/settings/`：运行状态、运行配置和 hook 设置。
- `frontend/src/features/shared/`：共享弹窗等通用界面。
- `frontend/src/types/`：按运行时、会话、草稿、历史和界面状态拆分的前端类型；`frontend/src/appTypes.ts` 继续作为统一导出入口。

## 核心概念

- **Agent 客户端**：Claude Code 或 Codex CLI。
- **Agent 会话**：由 `Agent 客户端 + session_id` 唯一标识的一次可 resume 对话。
- **会话工作区**：PromptHarbor 中绑定到某个活动 Agent 会话的编辑界面。
- **当前草稿**：绑定到 Agent 会话的未发送 Markdown prompt。
- **已发送 prompt**：由 Agent 客户端 hook 捕获到的用户真实提交内容。

完整领域术语见 [CONTEXT.md](CONTEXT.md)。

## 设计文档

- [MVP 方案设计](docs/promptbox-design.md)
- [代码规范索引](docs/code-style/README.md)
- [ADR 0001：默认本地优先且不联网](docs/adr/0001-local-first-and-no-network-by-default.md)
- [ADR 0002：MVP 优先验证本地提示词工作流](docs/adr/0002-mvp-validates-the-local-prompt-workflow.md)
- [ADR 0003：hook 采集优先保护隐私而不是完整捕获](docs/adr/0003-hook-privacy-before-capture-completeness.md)

## 计划技术栈

- Rust
- Tauri
- React
- Milkdown
- SQLite

## 隐私边界

PromptHarbor 默认只在本机保存数据：

- 不上传 prompt。
- 不记录模型回复。
- 不自动读取 Claude Code 或 Codex CLI 原始会话文件。
- 不默认调用外部大模型 API。
- 暂停记录开启时，hook 采集器不会读取 stdin。

## License

本项目使用 MIT 协议，见 [LICENSE](LICENSE)。
