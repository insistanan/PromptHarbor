# Agent 协作规范

## 语言

始终使用中文与用户沟通，包括解释、计划、总结、提交说明和新建文档内容。

## 文件同步约束

根目录下的 `AGENTS.md` 与 `CLAUDE.md` 是同一份协作规范的两个入口。修改任一文件时，必须同步更新另一个文件，保持正文内容一致。不要只改其中一份。

## 项目背景

本项目用于设计并实现提示港 PromptHarbor：一个面向 Claude Code 与 Codex CLI 的本地提示词编辑、暂存和会话记录工具。当前设计方向是 Rust + Tauri + React + Milkdown + SQLite。

关键设计文档：

- `docs/promptbox-design.md`：产品目标、架构、hooks 接入、数据模型和里程碑。
- `docs/code-style/README.md`：代码规范索引。
- `docs/code-style/rust.md`：Rust 后端与 hook 采集器规范。
- `docs/code-style/frontend.md`：Tauri 前端、React 和 Milkdown 规范。
- `docs/code-style/docs.md`：项目文档写作规范。

## 默认行为

除非用户明确要求，否则不要：

- 创建新文档。
- 运行代码。
- 编译。
- 测试。
- 做频繁总结；多轮协作中最多四轮做一次阶段总结。

如果需要使用终端命令，优先使用 Windows PowerShell 或 cmd 语义。读文件、列目录和搜索时优先使用轻量命令；涉及代码搜索时优先使用 `rg`。

## 编辑原则

- 先阅读相关上下文，再修改文件。
- 修改范围要贴合当前请求，不做无关重构。
- 如果写入或编辑文件失败，先判断是否因为文本过长；如果是，分批写入或分批编辑。
- 不覆盖用户已有改动；遇到不相关的未提交改动时保持原样。
- 不把详细代码规范堆在 `AGENTS.md` 或 `CLAUDE.md`，应放入 `docs/code-style/`。

## 架构约束

- hooks 采集逻辑必须轻量、稳定、失败不阻断 Claude Code 或 Codex CLI。
- 已发送 prompt 以 CLI hook 捕获内容为准，草稿只代表编辑状态。
- 默认只记录用户 prompt，不记录模型回复。
- 默认数据只保存在本机。
- Claude Code 与 Codex CLI 的适配层要分离，统一转换为项目内部事件模型。

## 代码规范入口

实现代码时必须参考 `docs/code-style/` 下的规范：

- Rust、SQLite、hook、Tauri 后端：参考 `docs/code-style/rust.md`。
- React、Milkdown、Tauri 前端和交互：参考 `docs/code-style/frontend.md`。
- Markdown 文档、设计说明和 ADR：参考 `docs/code-style/docs.md`。

若规范与用户当前明确指令冲突，以用户当前明确指令为准，并在回复中说明取舍。

## 交付说明

完成任务后，只说明关键改动和必要的验证情况。若未运行代码、编译或测试，需要明确说明未运行。
