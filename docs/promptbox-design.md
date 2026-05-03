# PromptBox 方案设计

## 0. 方案状态

MVP 方案已确认，可以进入实现拆分阶段。

本方案确认的是产品边界、领域术语、架构方向、数据模型、hooks 接入策略、隐私边界和 MVP 验收范围。Claude Code、Codex CLI、Tauri、Milkdown 在当前 Windows 环境下的具体行为仍需按“待验证清单”实测。

## 1. 背景

在 Claude Code 和 Codex CLI 中进行长对话时，用户经常需要先阅读模型的大段输出，再组织下一轮多条修改意见。直接在命令行里编辑长 prompt 不舒服，单独打开 Markdown 文件又无法自然绑定到具体对话，也容易丢失上下文。

PromptBox 的目标是提供一个本地优先的提示词工作台：用户在 PromptBox 中为某个 Agent 会话编写下一轮 prompt 草稿，复制到对应 Agent 客户端中提交；PromptBox 通过 hooks 记录用户实际提交的 prompt，并按会话回看、搜索和管理。

PromptBox 只记录用户 prompt，不记录模型回复。

## 2. 核心术语

本项目的权威术语以根目录 [CONTEXT.md](../CONTEXT.md) 为准。方案文档中使用以下核心词：

- **Agent 客户端**：PromptBox 支持接入的本地命令行 agent 工具，MVP 包含 Claude Code 和 Codex CLI。
- **Agent 会话**：一次可被 Agent 客户端 resume 的对话实例，由 `Agent 客户端 + session_id` 唯一标识。
- **活动 Agent 会话**：当前仍处于可交互状态、可继续接收用户 prompt 的 Agent 会话。
- **可能已关闭 Agent 会话**：长时间没有收到 hook 事件、发送目标状态不确定的 Agent 会话。
- **历史 Agent 会话**：只用于回看、搜索和复制历史 prompt，不作为发送目标。
- **会话工作区**：PromptBox 中绑定到一个活动 Agent 会话的可编辑界面，用于编写该会话的当前草稿。
- **当前草稿**：绑定到一个 Agent 会话的未发送 Markdown prompt。
- **复制草稿**：将当前草稿的 Markdown 源文本放入系统剪贴板，由用户自行粘贴到对应 Agent 客户端。
- **已发送 prompt**：由 Agent 客户端 hook 捕获到、用户已经实际提交给 agent 的 prompt。
- **低信息 prompt**：简短确认、寒暄或继续指令等仍需记录但可在展示层弱化的已发送 prompt。

不要把“终端窗口”建模为核心领域对象。终端窗口只是用户操作 Agent 客户端的环境，PromptBox 的稳定身份来自 `Agent 客户端 + session_id`。

## 3. 产品目标

MVP 必须验证完整最小闭环：

1. 用户在 Claude Code 或 Codex CLI 中提交 prompt。
2. `UserPromptSubmit` hook 捕获真实提交内容。
3. PromptBox 创建或更新对应 Agent 会话。
4. 用户在 PromptBox 中打开该会话的会话工作区。
5. 用户用 Milkdown 编写当前草稿。
6. 用户复制 Markdown 源文本到剪贴板。
7. 用户粘贴到对应 Agent 客户端并提交。
8. hook 捕获提交内容，形成已发送 prompt。
9. 如果提交内容与当前草稿 hash 一致，PromptBox 自动清空草稿。
10. 用户可以回看和简单搜索历史 prompt。

MVP 必须包含：

- 同时支持 Claude Code 和 Codex CLI 两种 Agent 客户端。
- 以 `UserPromptSubmit` 为唯一必须 hook 事件。
- SQLite 保存 Agent 会话、已发送 prompt、当前草稿和短期 raw hook 事件。
- 主窗口显示活动、可能已关闭和历史会话。
- 系统托盘入口，关闭主窗口默认隐藏到托盘。
- 会话工作区内嵌在主窗口中。
- Milkdown 所见即所得编辑当前草稿。
- Markdown 源文本只读查看。
- 复制草稿到剪贴板。
- hook 确认内容一致后清空草稿。
- PromptBox 未运行但配置可读且未暂停时，hook 写入 spool，主程序启动后导入。
- 简单搜索会话标题、首条 prompt、已发送 prompt 和当前草稿。
- 全局暂停记录。
- 用户级配置向导。

MVP 不包含：

- 自动总结。
- 提示词优化。
- “坏了我成替身了”人设提炼模块。
- 历史导入。
- 导出。
- 自动注入终端输入。
- 独立弹出会话工作区窗口。
- 全局快捷键。
- SQLite FTS、中文分词或高级搜索语法。
- raw hook 事件 UI。
- 项目级 hooks 自动安装。
- 源码编辑模式。
- 多草稿或草稿版本管理。

## 4. 架构概览

```text
Claude Code / Codex CLI
        |
        | UserPromptSubmit hook
        v
promptbox-hook.exe
        |
        | localhost HTTP 127.0.0.1:<port> + token
        | 或 spool fallback
        v
promptbox-app 后端
        |
        | SQLite
        v
本地数据库
        |
        v
Tauri 主窗口 + React + Milkdown
```

推荐代码结构：

```text
promptbox/
  crates/
    promptbox-core/   # 共享类型、事件归一化、数据库、配置模型
    promptbox-hook/   # hook 采集器，可被 Claude Code / Codex CLI 调用
    promptbox-app/    # Tauri 后端、本地采集端点、托盘、窗口控制
  frontend/
    src/
      components/
      editor/
      pages/
  docs/
  migrations/
```

前端技术：

- Tauri 负责桌面壳、本地权限、托盘、剪贴板和后端通信。
- React 负责 UI 组织。
- Milkdown 负责 Markdown 所见即所得编辑。

后端技术：

- Rust。
- SQLite。
- `sqlx` 或 `rusqlite` 均可，MVP 推荐优先考虑 migration 管理清晰的方案。

## 5. 本地目录与配置

PromptBox home 统一规则：

```text
PromptBox home = PROMPTBOX_HOME 或 %APPDATA%\PromptBox
数据库 = <PromptBox home>\promptbox.sqlite
配置 = <PromptBox home>\config.toml
spool = <PromptBox home>\spool\events.jsonl
日志 = <PromptBox home>\logs\
hook = <PromptBox home>\bin\promptbox-hook.exe
```

默认 Windows 路径：

```text
%APPDATA%\PromptBox\config.toml
%APPDATA%\PromptBox\promptbox.sqlite
%APPDATA%\PromptBox\spool\events.jsonl
%APPDATA%\PromptBox\logs\
%APPDATA%\PromptBox\bin\promptbox-hook.exe
```

`PROMPTBOX_HOME` 可用于便携模式或调试。

PromptBox 用户配置保存：

- 本地采集端点地址。
- token。
- 暂停记录状态。
- maybe_closed 阈值。
- raw hook 事件保留设置。
- 开机自启动设置。

token 只能存在 PromptBox 用户配置中，不写入 Claude Code 或 Codex CLI 的 hook 命令行参数。

配置向导生成的 hooks 命令应使用 hook 可执行文件的绝对路径。MVP 不要求把 PromptBox 加入 PATH。

hook 可执行文件版本规则：

- 主程序随安装包携带当前版本 `promptbox-hook.exe`。
- 主程序启动时检查 `<PromptBox home>\bin\promptbox-hook.exe` 是否存在以及版本是否一致。
- 版本检测以运行 `promptbox-hook.exe --version` 的输出为准。
- `--version` 输出必须包含应用版本和 hook 协议版本，例如 `promptbox-hook <app_version>` 与 `hook_protocol <protocol_version>`。
- 如果 `--version` 调用失败、版本缺失或协议版本不匹配，则尝试替换。
- 版本缺失或不一致时尝试复制/替换。
- 替换失败时，主窗口显示“hook 可执行文件更新失败”。
- 配置向导显示当前 hook binary 路径和版本状态。
- hook 应尽量保持对旧配置文件的向后兼容。

## 6. 本地采集端点

PromptBox 主程序启动时自动启动本地采集端点。托盘常驻期间端点保持运行；只有用户选择退出 PromptBox 时才停止端点。

默认端点：

```text
127.0.0.1:9996
```

规则：

- 只绑定 localhost。
- 默认端口为 `9996`，允许在配置中修改。
- 如果端口被占用，主窗口仍可打开，但必须显示“采集不可用”。
- 端口被占用时不自动漂移到随机端口。
- 修改端口后，配置向导必须引导用户同步更新 hooks 配置。
- HTTP 请求通过 header 携带 token，例如 `Authorization: Bearer <token>`。
- 请求体大小需要限制，例如单个 prompt 最大 512 KB。
- 写入失败时返回非 2xx，但 hook 采集器仍不阻断 Agent 客户端。

## 7. hook 采集器

`promptbox-hook` 是被 Claude Code 和 Codex CLI 调用的极小可执行文件。它的职责必须保持窄：

- 根据启动参数识别 provider，例如 `--provider claude` 或 `--provider codex`。
- 读取 PromptBox 用户配置。
- 在允许采集时读取 stdin hook JSON。
- 将不同 Agent 客户端的字段归一化为统一事件。
- 尝试投递到本地采集端点。
- 本地采集端点不可用时写入 spool。
- 默认退出码为 0，避免影响 Claude Code 或 Codex CLI 的正常工作。

### 7.1 隐私优先读取顺序

hook 采集器必须按以下顺序执行：

1. 解析 `--provider`。
2. 定位并读取 PromptBox 用户配置。
3. 如果配置读取失败：不读取 stdin、不写 spool、不投递事件，直接成功退出，只记录不含 prompt 的错误。
4. 如果 `recording_paused = true`：不读取 stdin、不写 spool、不投递事件，直接成功退出。
5. 配置读取成功且未暂停时，读取 stdin。
6. 记录 hook 采集器捕获事件的时间 `captured_at`。
7. 归一化事件。
8. 投递到本地采集端点。
9. 投递失败时写入 spool。
10. 无论采集是否成功，默认退出 0。

这个顺序来自 ADR：[0003-hook-privacy-before-capture-completeness.md](adr/0003-hook-privacy-before-capture-completeness.md)。

### 7.2 spool 规则

spool 是 PromptBox 主程序不可用时的临时队列。

规则：

- 只有在配置可读且未暂停时才可能写 spool。
- spool 保存完整归一化 hook 事件，包括 prompt 内容，因此必须按敏感数据处理。
- 每行一个 JSON。
- spool 事件必须包含 `captured_at`。
- 主程序启动后导入 spool；导入成功后截断、轮转或删除 spool。
- spool 不能长期作为第二份历史库保留。
- 删除 PromptBox 数据时，也应提供清空 spool。

## 8. Agent 客户端接入

### 8.1 共同原则

MVP 只强依赖 `UserPromptSubmit`：

- 收到 `UserPromptSubmit` 时，如果对应 Agent 会话不存在，应立即创建。
- `SessionStart` 只作为后续增强事件，不是 MVP 前置条件。
- 配置向导默认只安装 `UserPromptSubmit` hook。
- `SessionStart` 不默认安装，只作为高级选项或后续增强。

hook 命令不携带 token：

```powershell
promptbox-hook.exe --provider claude
promptbox-hook.exe --provider codex
```

provider 来自命令行参数，不从 hook JSON 猜测。

### 8.2 Claude Code

MVP 默认安装 Claude Code 的 `UserPromptSubmit` hook。

关键字段：

- `session_id`
- `transcript_path`
- `cwd`
- `hook_event_name`
- `prompt`

用户级配置示意：

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "C:\\Users\\<user>\\AppData\\Local\\PromptBox\\promptbox-hook.exe --provider claude"
          }
        ]
      }
    ]
  }
}
```

配置向导需要优先支持：

- 用户级：`~/.claude/settings.json`
- 项目级只提供片段：`.claude/settings.json`
- 项目本地级只提供片段：`.claude/settings.local.json`

### 8.3 Codex CLI

MVP 默认安装 Codex CLI 的 `UserPromptSubmit` hook，并检测 `codex_hooks` 是否开启。

Codex 需要：

```toml
[features]
codex_hooks = true
```

关键字段：

- `session_id`
- `transcript_path`
- `cwd`
- `hook_event_name`
- `model`
- `turn_id`
- `prompt`

配置示意：

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "C:\\Users\\<user>\\AppData\\Local\\PromptBox\\promptbox-hook.exe --provider codex"
          }
        ]
      }
    ]
  }
}
```

配置向导需要优先支持：

- 用户级：`~/.codex/hooks.json`
- 用户级：`~/.codex/config.toml`
- 项目级只提供片段：`.codex/hooks.json`
- 项目级只提供片段：`.codex/config.toml`

Codex 侧注意：

- hooks 需要 feature flag。
- 多个匹配 hook 可能并发运行，PromptBox 不能依赖 hook 执行顺序。
- MVP 不扫描 Codex 会话目录，也不解析历史 rollout。

## 9. 统一事件模型

hook 原始输入先转换为统一结构：

```rust
struct PromptEvent {
    provider: Provider,
    event_name: String,
    session_id: String,
    turn_id: Option<String>,
    cwd: Option<PathBuf>,
    transcript_path: Option<PathBuf>,
    model: Option<String>,
    prompt: Option<String>,
    captured_at: DateTime<Utc>,
    raw_json: serde_json::Value,
}

enum Provider {
    Claude,
    Codex,
}
```

归一化规则：

- `provider` 来自 hook 启动参数。
- `session_id` 是 Agent 会话身份的一部分。
- `turn_id` 是通用可选字段，MVP 主要用于 Codex 去重。
- `cwd` 用于项目维度分组。
- `cwd` 入库前尽量转为绝对路径，去掉末尾分隔符；Windows 下比较大小写不敏感。
- MVP 不自动把 `cwd` 归并到 Git 仓库根目录。
- `transcript_path` 只作为 Agent 会话元信息和未来导入线索，不触发自动读取。
- `prompt` 只在 `UserPromptSubmit` 时写入正式 prompt 历史。
- `raw_json` 只进入短期 raw hook 事件。
- 正式业务表只保存通用字段和少量有明确业务价值的可选字段，例如 `turn_id`。
- 不在正式业务表长期保存完整 provider payload。

## 10. 数据库设计

### 10.1 sessions

```sql
create table sessions (
  id integer primary key autoincrement,
  provider text not null,
  session_id text not null,
  status text not null default 'active',
  cwd text,
  transcript_path text,
  model text,
  first_prompt text,
  title text,
  title_source text not null default 'session_id',
  last_hook_at text,
  maybe_closed_at text,
  archived_at text,
  created_at text not null,
  updated_at text not null,
  unique(provider, session_id)
);
```

说明：

- `status` 可取 `active`、`maybe_closed`、`archived`。
- `first_prompt` 是该 Agent 会话第一条已发送 prompt，是事实字段。
- `title` 是可编辑展示字段。
- `title_source` 可取 `session_id`、`first_non_low_info_prompt`、`manual`、`generated`。
- 第一版标题优先从第一条非低信息 prompt 截断生成。
- 如果没有可用内容，标题使用 `未命名会话` 或 session 短 ID。
- `last_hook_at` 用于判断会话是否长时间无 hook 事件。
- 默认 12 小时无 hook 事件后标记为 `maybe_closed`，该阈值可配置。
- 自动总结未来只更新 `title` 和 `title_source`，不修改 `first_prompt`。

### 10.2 drafts

```sql
create table drafts (
  id integer primary key autoincrement,
  session_db_id integer not null references sessions(id),
  content_md text not null,
  content_hash text not null,
  copy_state text not null default 'idle',
  copied_at text,
  last_copied_hash text,
  updated_at text not null,
  unique(session_db_id)
);
```

说明：

- 每个 Agent 会话同一时间最多一个当前草稿。
- `copy_state` 可取 `idle`、`copied`。
- 点击复制草稿只更新 `copy_state`、`copied_at` 和 `last_copied_hash`，不清空内容。
- hook 捕获到同一会话中 hash 一致的已发送 prompt 后，才清空草稿并恢复 `copy_state = 'idle'`。
- 归档有非空草稿的会话前必须二次确认。
- 归档后草稿仍保留，但历史会话不是发送目标；草稿只能查看或复制。
- 历史会话后续收到 `UserPromptSubmit` hook 时，自动恢复为活动会话，并恢复草稿编辑能力。
- 自动恢复活动时可以保留 `archived_at` 作为最后归档时间，用于审计和 UI 提示。

### 10.3 prompt_events

```sql
create table prompt_events (
  id integer primary key autoincrement,
  session_db_id integer not null references sessions(id),
  provider text not null,
  session_id text not null,
  turn_id text,
  prompt_md text not null,
  prompt_hash text not null,
  is_low_info integer not null default 0,
  matched_draft_id integer references drafts(id),
  source text not null default 'hook',
  sent_at text not null,
  created_at text not null
);
```

说明：

- `source` 在 MVP 中固定为 `hook`。
- `sent_at` 表示 hook 采集器捕获事件的时间，用于历史排序。
- `created_at` 表示 PromptBox 写入数据库的时间。
- `prompt_hash` 使用统一 hash 规则。
- `is_low_info` 只影响展示弱化和筛选，不影响搜索、导出或数据完整性。
- MVP 自动计算 `is_low_info`，不提供逐条手动覆盖。
- 未来如需强调某条 prompt，优先增加收藏或重要标记，不直接把 `is_low_info` 变成用户编辑字段。
- `matched_draft_id` 用于记录该 prompt 是否与某个草稿复制状态匹配。
- 有 `turn_id` 的事件，优先使用 `(provider, session_id, turn_id)` 去重。
- 没有 `turn_id` 的事件，使用 `provider + session_id + prompt_hash + 时间窗口` 近似去重。

建议索引：

```sql
create index idx_sessions_status_updated_at on sessions(status, updated_at);
create index idx_sessions_cwd on sessions(cwd);
create index idx_prompt_events_session_sent_at on prompt_events(session_db_id, sent_at);
create unique index idx_prompt_events_turn_id
  on prompt_events(provider, session_id, turn_id)
  where turn_id is not null;
```

### 10.4 raw_hook_events

```sql
create table raw_hook_events (
  id integer primary key autoincrement,
  provider text not null,
  session_id text,
  event_name text not null,
  raw_json text not null,
  received_at text not null,
  expires_at text not null
);
```

说明：

- raw hook 事件只用于诊断和兼容字段变化。
- 默认短期保留 7 天。
- 用户可以关闭 raw hook 事件保留。
- 删除某个 Agent 会话时，应同步删除该会话相关 raw hook 事件。
- 正式历史以归一化后的已发送 prompt 为准，不以 raw hook 事件为准。

## 11. 会话状态规则

状态：

- `active`：活动 Agent 会话，可打开会话工作区并编辑当前草稿。
- `maybe_closed`：可能已关闭 Agent 会话，状态不确定。
- `archived`：历史 Agent 会话，只用于回看和复制。

规则：

- 新 `UserPromptSubmit` 会创建或更新 Agent 会话，并标记为 `active`。
- `active` 默认 12 小时没有收到 hook 事件后变为 `maybe_closed`。
- 12 小时阈值可配置。
- `maybe_closed` 收到新 hook 后恢复为 `active`。
- 用户可以手动将 `active` 或 `maybe_closed` 归档为 `archived`。
- `archived` 收到新 `UserPromptSubmit` 后自动恢复为 `active`。
- 归档不删除 prompt 或草稿。
- 历史会话不是发送目标。

PromptBox 不能把“当前会话”建模为全局单例。多个活动 Agent 会话可以同时存在。

## 12. 草稿、复制与清空规则

当前草稿绑定到 Agent 会话，不绑定到窗口实例。

复制流程：

1. 用户打开某个活动 Agent 会话的会话工作区。
2. 用户编辑当前草稿。
3. 用户点击“复制”。
4. PromptBox 复制 Markdown 源文本到剪贴板。
5. PromptBox 将草稿标记为 `copied`，记录 `last_copied_hash` 和 `copied_at`。
6. 用户在对应 Agent 客户端中粘贴并提交。
7. hook 捕获真实提交内容。
8. 如果提交内容 hash 与 `last_copied_hash` 一致，PromptBox 记录已发送 prompt，并清空草稿。
9. 如果提交内容不一致，PromptBox 记录真实已发送 prompt，但保留当前草稿。

hash 规则：

- 草稿和 hook 捕获的 prompt 使用同一函数计算 hash。
- 计算前只做首尾空白 `trim`。
- 不修改内部空格、换行、列表缩进或代码块内容。
- 使用 SHA-256。
- 空字符串不允许复制。

已发送 prompt 的事实来源永远是 hook 捕获内容，不是草稿或剪贴板内容。

## 13. 低信息 prompt

低信息 prompt 仍然是已发送 prompt，必须记录。

MVP 判定规则使用简单启发式：

- 去空白后长度小于等于 8。
- 或命中低信息词表，例如 `同意`、`继续`、`好的`、`收到`、`hi`、`你好`。

展示规则：

- 默认可以用更紧凑、低权重样式展示。
- 用户可以筛选隐藏低信息 prompt。
- 搜索和完整历史仍包含低信息 prompt。
- 数据层不能丢弃低信息 prompt。

## 14. 前端信息架构

主窗口：

```text
┌─────────────────────────────────────────────────────────────┐
│ 顶栏：项目筛选 / Agent 客户端筛选 / 搜索 / 采集状态 / 设置 │
├───────────────┬───────────────────────┬─────────────────────┤
│ 会话列表       │ prompt 历史            │ 会话工作区           │
│ 活动           │ 已发送 prompt 时间线    │ Milkdown 当前草稿     │
│ 可能已关闭     │ 低信息 prompt 弱化      │ 复制 / 查看 Markdown  │
│ 历史           │ 复制历史 prompt         │ 清空 / 插入历史       │
└───────────────┴───────────────────────┴─────────────────────┘
```

### 14.1 会话列表

默认分区：

- 活动：`active`，优先显示，可打开会话工作区。
- 可能已关闭：`maybe_closed`，次级显示，打开工作区前提示确认。
- 历史：`archived`，默认折叠，只用于回看和复制。

每条会话显示：

- Agent 客户端：Claude Code 或 Codex CLI。
- session ID 短 ID。
- 项目目录名，来自规范化 `cwd` 的 basename。
- 会话标题。
- 最近更新时间。
- 采集状态或归档状态。

MVP 只显示 hook 发现的 Agent 会话：

- 不扫描 Claude/Codex 会话目录。
- 不手动输入 `session_id` 登记会话。
- 空状态提示用户安装 hooks 后，在 Agent 客户端中提交一次 prompt。

### 14.2 prompt 历史

只展示用户 prompt，不展示模型回复。

每条 prompt 显示：

- `sent_at`。
- prompt 内容。
- 是否低信息 prompt。
- 复制按钮。
- 插入到当前草稿按钮。

历史按 `sent_at` 排序，而不是按数据库入库时间排序。

### 14.3 会话工作区

MVP 中会话工作区内嵌在主窗口中，不做独立弹出窗口。

能力：

- Milkdown 所见即所得编辑。
- Markdown 源文本只读查看。
- 自动保存，建议 500 ms debounce。
- 复制 Markdown 源文本。
- 清空当前草稿。
- 从历史 prompt 插入引用。

按钮：

- `复制`：复制当前 Markdown 源文本到剪贴板。
- `查看 Markdown`：只读查看将被复制的 Markdown 源文本。
- `清空`：清空当前草稿。
- `插入历史`：把选中的历史 prompt 插入到光标处。

MVP 不做源码编辑模式。

## 15. 托盘与应用生命周期

MVP 包含系统托盘入口。

规则：

- PromptBox 启动后自动启动本地采集端点。
- 关闭主窗口默认隐藏到托盘。
- 隐藏到托盘时，本地采集端点继续运行。
- 托盘菜单至少包含“打开主窗口”和“退出 PromptBox”。
- 用户选择退出 PromptBox 时，停止本地采集端点。
- 开机自启动设置默认关闭，必须由用户显式开启。

MVP 不做全局快捷键。全局快捷键属于后续增强，未来需要可配置、可禁用，并处理冲突提示。

## 16. 搜索

MVP 提供简单搜索，不做 SQLite FTS、中文分词或高级查询语法。

搜索范围：

- 会话标题。
- 首条 prompt。
- 全部已发送 prompt，包括低信息 prompt。
- 当前草稿。

不搜索：

- raw hook 事件。
- spool 事件。
- Claude Code 或 Codex CLI 原始会话文件。
- 模型回复。

结果按会话聚合，点击结果应定位到匹配 prompt 或草稿。

## 17. 配置向导

配置向导用于检测并引导安装 Claude Code 和 Codex CLI hooks。

MVP 能力：

- 显示 `promptbox-hook.exe` 路径。
- 显示 PromptBox home。
- 显示本地采集端点地址和采集状态。
- 检测 Claude Code 用户级 hook 是否已安装。
- 检测 Codex CLI 用户级 hook 是否已安装。
- 检测 Codex `codex_hooks` 是否开启。
- 一键写入用户级 hook 配置。
- 写入前备份原文件。
- 保留用户已有 hooks 和未知字段。
- 无法安全解析或合并时，降级为手动配置模式。
- 项目级 hooks 只提供配置片段，不自动写入。
- 开机自启动设置，默认关闭。
- 暂停记录开关。

配置写入要求：

- JSON 和 TOML 必须用结构化 parser 处理。
- 不能用字符串拼接改配置。
- 写入前备份，例如 `settings.json.promptbox.bak`。
- 不删除未知字段。
- Windows 路径必须正确处理空格和反斜杠。

## 18. 隐私与数据边界

默认策略：

- 本地优先。
- 默认不联网。
- 默认不调用外部大模型 API。
- 不做云同步。
- 不记录模型回复。
- 不上传 prompt。
- 不自动读取项目源码。
- 不自动读取 Claude/Codex 原始 transcript 或 rollout。
- 删除 PromptBox 数据不会修改 Agent 客户端原始会话文件。

暂停记录：

- MVP 提供全局暂停记录开关。
- 暂停记录不停止本地采集端点。
- 暂停记录不修改 Agent 客户端 hooks 配置。
- 暂停期间本地采集端点仍返回成功。
- 暂停期间不写入 sessions、prompt_events 或 raw_hook_events。
- hook 采集器如果读到暂停开启，必须在读取 stdin 之前退出。

删除：

- 删除会话、prompt 或草稿只影响 PromptBox 数据库。
- 删除会话时同步删除该会话相关 raw hook 事件。
- 不修改 Claude Code 或 Codex CLI 原始会话文件。
- 可以展示原始会话文件路径，但不负责编辑或清理这些文件。

这部分隐私边界来自 ADR：[0001-local-first-and-no-network-by-default.md](adr/0001-local-first-and-no-network-by-default.md) 和 [0003-hook-privacy-before-capture-completeness.md](adr/0003-hook-privacy-before-capture-completeness.md)。

## 19. 历史导入

MVP 不做历史导入。

规则：

- PromptBox 从 hooks 安装后捕获的新事件开始建立正式历史。
- `transcript_path` 只作为 Agent 会话元信息和未来导入线索保存。
- 不自动读取 Claude Code transcript。
- 不自动读取 Codex rollout。
- 不扫描历史会话目录。

后续如果做历史导入：

- 优先做手动导入单个会话。
- 只导入用户 prompt。
- 不导入 assistant 回复。
- 解析失败必须是非致命错误。
- 不修改源文件。

## 20. 后续 TODO

基础增强：

- 单个会话 Markdown 导出。
- 全库 JSON 导出。
- 历史导入。
- SQLite FTS。
- 中文分词。
- 独立弹出会话工作区窗口。
- 全局快捷键。
- 项目级 hook 自动安装。
- Prompt 收藏或重要标记。
- 草稿快照或多草稿。
- 与终端窗口更深集成。

需要大模型 API 的后续模块：

- 自动生成会话标题：根据用户 prompt 生成更好的展示标题。
- 提示词优化模块：基于用户草稿提供改写、压缩、结构化和语气调整建议。
- “坏了我成替身了”模块：提炼用户所有提示词中的表达习惯、语气和偏好，生成用户人设；支持用该人设与用户对话，也支持导出 Markdown 人设包。

这些大模型 API 模块必须显式开启，并在启用前说明会发送哪些数据。

## 21. 里程碑

### M1：采集与存储闭环

- 实现 `promptbox-core` 的统一事件模型。
- 实现 `promptbox-hook`。
- 实现 PromptBox 用户配置读取。
- 实现隐私优先 hook 读取顺序。
- 实现 localhost HTTP 本地采集端点。
- 实现 spool fallback。
- 实现 SQLite schema 和 migration。
- Claude Code 与 Codex CLI 的 `UserPromptSubmit` 都能入库。

验收标准：

- Claude Code 提交 prompt 后，数据库出现对应 Agent 会话和已发送 prompt。
- Codex CLI 提交 prompt 后，数据库出现对应 Agent 会话和已发送 prompt。
- app 未启动但配置可读且未暂停时，hook 写入 spool；app 启动后导入。
- 暂停记录开启时，hook 不读取 stdin。
- 配置读取失败时，hook 不读取 stdin。

### M2：桌面工作台

- Tauri 主窗口。
- 系统托盘入口。
- 活动、可能已关闭、历史会话分区。
- prompt 历史列表。
- 会话工作区。
- Milkdown 当前草稿。
- Markdown 源文本只读查看。
- 复制草稿到剪贴板。
- hook 确认内容一致后清空草稿。
- 简单搜索。

验收标准：

- 用户能看到 hook 发现的会话。
- 用户能打开活动会话的会话工作区。
- 用户能用 Milkdown 写 prompt。
- 用户能查看并复制 Markdown 源文本。
- 用户真实提交后，prompt 历史自动出现。
- 内容一致时草稿自动清空。
- 关闭主窗口后隐藏到托盘，本地采集端点继续运行。

### M3：配置与隐私控制

- 配置向导检测 Claude Code hook。
- 配置向导检测 Codex CLI hook 与 `codex_hooks`。
- 一键安装用户级 `UserPromptSubmit` hook。
- 项目级配置片段展示。
- 采集状态显示。
- 暂停记录。
- 开机自启动设置。
- raw hook 事件保留设置。

验收标准：

- 配置向导不破坏用户已有 hooks。
- 写配置前自动备份。
- 复杂配置无法安全合并时降级手动模式。
- 暂停记录开启后不保存 prompt。

## 22. 风险与缓解

### hooks 行为可能变化

Claude Code 和 Codex CLI 的 hooks 都是外部集成点，字段和配置格式可能变化。

缓解：

- provider 适配层隔离。
- 正式表只保存通用字段。
- raw hook 事件短期保留 7 天用于诊断。
- 配置向导显示检测结果。

### PromptBox 不能阻断主工作流

PromptBox 是辅助工具，不能因为采集失败打断 Agent 客户端。

缓解：

- hook 默认退出 0。
- 本地服务不可用时写 spool。
- spool 写失败只记录不含 prompt 的错误。
- 主窗口显示采集健康状态。

### 隐私压力

prompt 可能包含敏感信息。

缓解：

- 默认本地优先、不联网。
- 不记录模型回复。
- 暂停记录时 hook 不读取 stdin。
- 配置不可读时 hook 不读取 stdin。
- raw hook 事件短期保留且可关闭。

### 自动注入终端复杂

自动发送 prompt 需要处理焦点、终端类型、TUI 状态和误粘贴风险。

缓解：

- MVP 只做复制草稿。
- 已发送 prompt 以 hook 捕获为准。
- 自动注入作为后续独立决策。

## 23. 待验证清单

实现前需要实测：

- Claude Code 当前版本 `UserPromptSubmit` 输入字段。
- Claude Code Windows 下 hook 命令路径包含空格时的转义规则。
- Claude Code 用户级和项目级 settings 合并行为。
- Codex CLI 当前版本启用 `codex_hooks` 的准确配置位置和优先级。
- Codex CLI `UserPromptSubmit` 是否始终提供 `session_id`、`turn_id`、`transcript_path`。
- Codex CLI 在 Windows 下 hooks 并发执行时的超时和 stdin 行为。
- PromptBox hook 在配置读取失败时不读取 stdin 的实现可行性。
- Milkdown 在 Tauri WebView2 中的大文档性能。
- Tauri 剪贴板 API 在 Windows 终端工作流里的稳定性。
- Tauri 托盘在 Windows 下关闭隐藏和退出行为。

## 24. ADR

已记录的关键决策：

- [0001 PromptBox 默认本地优先且不联网](adr/0001-local-first-and-no-network-by-default.md)
- [0002 MVP 优先验证本地提示词工作流](adr/0002-mvp-validates-the-local-prompt-workflow.md)
- [0003 hook 采集优先保护隐私而不是完整捕获](adr/0003-hook-privacy-before-capture-completeness.md)

## 25. 参考资料

- Claude Code hooks 文档：<https://docs.anthropic.com/en/docs/claude-code/hooks>
- Claude Code CLI reference：<https://docs.anthropic.com/en/docs/claude-code/cli-reference>
- Codex hooks 文档：<https://developers.openai.com/codex/hooks>
- Codex config 文档：<https://developers.openai.com/codex/config>
- Milkdown：<https://milkdown.dev/>
- Milkdown listener plugin：<https://github.com/Milkdown/milkdown/blob/main/docs/api/plugin-listener.md>
