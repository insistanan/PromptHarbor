# 提示港 PromptHarbor

提示港是一个面向 Claude Code 和 Codex CLI 的本地提示词工作台。它帮助用户为每个 Agent 会话编写 Markdown 草稿、复制到对应 Agent 客户端中提交，并通过 hooks 记录用户实际发送的 prompt，支持历史回看与搜索。

## 项目状态

当前 MVP 核心闭环已在 Windows 环境完成一轮实机验证，仓库包含 Tauri + React + Rust workspace 的可运行实现。

已确认的 MVP 目标：

- 同时支持 Claude Code 和 Codex CLI。
- 只记录用户 prompt，不记录模型回复。
- 使用 `UserPromptSubmit` hook 捕获用户实际提交内容。
- 使用 Milkdown 提供 Typora-like Markdown 草稿编辑体验。
- 使用 Tauri + Rust + SQLite 构建本地桌面应用。
- 默认本地优先、不联网、不调用外部大模型 API。
- 支持全局暂停记录。
- 支持系统托盘常驻。

## 开发启动

当前实现包含：

- `crates/promptbox-core`：共享领域类型和应用状态。
- `crates/promptbox-hook`：Claude Code / Codex CLI 调用的轻量采集器，支持 `--version`、本地端点投递和 spool fallback。
- `crates/promptbox-app`：Tauri 应用、本地采集端点、SQLite 持久化、hook 配置向导、托盘和窗口生命周期。
- `frontend`：React + Vite + Milkdown 会话工作区，支持草稿、复制、历史、搜索和暂停记录。

PromptBox home 默认路径：

```text
%APPDATA%\PromptBox
```

可以通过 `PROMPTBOX_HOME` 覆盖。主程序启动时会创建 `config.toml`、spool/log/bin 目录，并检查：

```text
<PromptBox home>\bin\promptbox-hook.exe
```

开发模式下如需让 hook 状态显示为就绪，需要先构建 hook 可执行文件，或用 `PROMPTBOX_HOOK_SOURCE` 指向一个已构建的 `promptbox-hook.exe`。hook 可执行文件必须支持：

```powershell
promptbox-hook.exe --version
```

本地采集端点默认监听 `127.0.0.1:9996`。隔离测试 hook 投递与 spool fallback 时，可以先设置临时 home：

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

安装前端依赖：

```powershell
npm --prefix frontend install
```

启动 Tauri 开发模式：

```powershell
npm --prefix frontend run tauri:dev
```

仅启动前端开发服务器：

```powershell
npm --prefix frontend run dev
```

Windows MVP 实机验证记录见 [MVP 方案设计](docs/promptbox-design.md#23-windows-mvp-实机验证记录)。

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
