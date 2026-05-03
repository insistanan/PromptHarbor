# Rust 代码规范

## 模块边界

PromptBox 的 Rust 代码建议拆分为三个职责清晰的部分：

- `promptbox-core`：共享类型、事件归一化、数据库访问、配置模型。
- `promptbox-hook`：Claude Code / Codex CLI 调用的轻量采集器。
- `promptbox-app`：Tauri 后端、本地服务、窗口控制和应用级命令。

不要让 hook 采集器直接依赖 Tauri，也不要把 provider 适配逻辑散落在 UI 层。

## hook 采集器

- hook 程序必须快速退出，不能长时间阻塞 CLI。
- 默认失败也退出 `0`，避免影响 Claude Code 或 Codex CLI 主流程。
- 读取 stdin 后立即归一化事件，再尝试投递到本地服务。
- 本地服务不可用时写入 spool JSONL。
- 不在 hook 里做数据库迁移、复杂解析、历史导入或网络同步。
- provider 必须来自命令行参数，例如 `--provider claude` 或 `--provider codex`，不要从 JSON 内容猜测。

## 错误处理

- 应用内部使用结构化错误类型，优先保留上下文。
- 用户可见错误要说明发生在哪个环节，例如 hook 投递、spool 写入、数据库写入或配置更新。
- 不要吞掉会影响数据完整性的错误；可以不中断 CLI，但必须写日志或保留 raw event。
- 对外部配置文件写入要先备份，再做最小修改。

## 数据模型

- 数据库表结构应通过 migration 管理。
- prompt 的事实来源是 `UserPromptSubmit` hook 捕获的内容。
- `raw_json` 可以保存用于兼容未来字段变化，但业务逻辑不要依赖未归一化字段。
- session 唯一性使用 `(provider, session_id)`。
- prompt 去重应结合 `provider`、`session_id`、`turn_id` 和内容 hash。

## SQLite

- 写入路径要可重试，避免短暂锁表导致数据丢失。
- migration 必须可重复执行。
- 删除操作只删除 PromptBox 自己的数据，不修改 Claude Code 或 Codex CLI 原始会话文件。
- 后续做全文搜索时优先考虑 SQLite FTS，不要先引入外部搜索服务。

## 配置文件处理

- 解析 JSON、TOML 等结构化配置时使用正式 parser，不用字符串拼接。
- 写回配置时尽量保留用户已有字段。
- 对未知字段保持透明，不删除、不重排大块用户配置。
- Windows 路径要注意空格、反斜杠和命令转义。

## 日志

- 日志应能定位 provider、event_name、session_id 和失败环节。
- 不要默认把完整 prompt 写入普通日志；完整 prompt 应进入数据库或受控 raw event。
- hook 侧日志要短，避免污染 CLI 输出。

## 命名

- 类型名使用明确领域词：`PromptEvent`、`Provider`、`SessionRecord`、`PromptRecord`。
- provider 适配模块建议命名为 `providers::claude` 和 `providers::codex`。
- 避免 `Manager`、`Helper`、`Util` 这类含义过宽的名称。

## 测试约定

只有在用户明确要求运行测试时才运行测试。编写测试代码时优先覆盖：

- Claude / Codex hook JSON 归一化。
- 缺失字段、未知字段和非法 JSON 的处理。
- prompt 去重。
- spool 写入和恢复导入。
- 配置文件增量写入。
