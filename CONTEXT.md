# PromptBox

PromptBox 是一个本地优先的提示词编辑与记录工具，用于在 Claude Code 和 Codex CLI 对话过程中暂存下一轮提示词，并按会话记录用户实际发送的 prompt。

## Language

**Agent 会话**:
一次可被 Claude Code 或 Codex CLI resume 的对话实例，由 **Agent 客户端** 与 `session_id` 共同唯一标识。
_Avoid_: 终端窗口、项目会话、聊天窗口

**Agent 客户端**:
PromptBox 支持接入的本地命令行 agent 工具，例如 Claude Code 或 Codex CLI。
_Avoid_: 终端、模型提供商、CLI 类型

**活动 Agent 会话**:
当前仍处于可交互状态、可继续接收用户 prompt 的 **Agent 会话**。
_Avoid_: 当前会话、最近会话、打开的历史

**历史 Agent 会话**:
已经不再作为当前输入目标使用，只用于回看、搜索和复制历史 prompt 的 **Agent 会话**。
_Avoid_: 关闭窗口、过期窗口

**可能已关闭 Agent 会话**:
长时间没有收到 hook 事件、发送目标状态不确定的 **Agent 会话**。
_Avoid_: 已关闭会话、失效会话

**会话工作区**:
PromptBox 中绑定到一个 **活动 Agent 会话** 的可编辑工作界面，用于编写该会话下一轮 prompt 草稿。
_Avoid_: 当前窗口、全局编辑器、历史窗口

**主窗口**:
PromptBox 中用于管理活动会话、历史会话、搜索和设置的入口界面。
_Avoid_: 编辑窗口、终端窗口

**托盘入口**:
PromptBox 常驻时在系统托盘中提供打开主窗口和退出应用的入口。
_Avoid_: 全局快捷键、后台服务

**暂停记录**:
用户临时禁止 PromptBox 保存 hook 捕获内容的全局隐私开关。
_Avoid_: 停止服务、关闭 hooks、删除历史

**配置向导**:
PromptBox 中检测并引导安装 Claude Code 与 Codex CLI hooks 的设置流程。
_Avoid_: 自动修复、强制安装

**spool 事件**:
PromptBox 主程序不可用时，hook 采集器临时写入本地队列、等待后续导入的 hook 事件。
_Avoid_: 正式记录、已入库 prompt

**raw hook 事件**:
PromptBox 为诊断和兼容字段变化而短期保留的原始 hook 输入。
_Avoid_: 正式 prompt 历史、长期备份

**本地采集端点**:
PromptBox 主程序在本机监听、供 hook 采集器投递事件的 localhost HTTP 端点。
_Avoid_: 远程服务、云端接口

**PromptBox 用户配置**:
PromptBox 保存在用户数据目录下、供主程序和 hook 采集器共同读取的本机配置。
_Avoid_: Agent 客户端配置、项目配置、hook 命令参数

**hook 可执行文件**:
配置到 Agent 客户端 hooks 中、负责采集 hook 事件并投递给 PromptBox 的本机可执行文件。
_Avoid_: 主程序、Tauri 应用、PATH 命令

**当前草稿**:
绑定到一个 **Agent 会话** 的未发送 Markdown prompt。
_Avoid_: 临时文本、编辑器内容、待发送消息

**复制草稿**:
将 **当前草稿** 的 Markdown 源文本放入系统剪贴板，由用户自行粘贴到对应 **Agent 客户端**。
_Avoid_: 发送、提交、注入终端

**已复制草稿**:
已经被复制到剪贴板、但尚未被 hook 确认为 **已发送 prompt** 的 **当前草稿**。
_Avoid_: 已发送、已提交

**已发送 prompt**:
由 **Agent 客户端** 的 hook 捕获到、用户已经实际提交给 agent 的 prompt。
_Avoid_: 草稿、复制内容、待发送 prompt

**低信息 prompt**:
简短确认、寒暄或继续指令等仍需记录但默认可在历史视图中弱化或隐藏的 **已发送 prompt**。
_Avoid_: 垃圾 prompt、无效 prompt

**首条 prompt**:
一个 **Agent 会话** 中被 hook 捕获到的第一条 **已发送 prompt**。
_Avoid_: 会话标题、摘要

**会话标题**:
用于在 PromptBox 中识别 **Agent 会话** 的可编辑展示名称。
_Avoid_: 首条 prompt、会话 ID、自动总结

## Relationships

- 一个 **Agent 会话** 属于一个 **Agent 客户端**。
- 一个 **Agent 会话** 可以关联一个项目目录；项目目录来自 hook 事件的规范化 `cwd`，但不能唯一标识 **Agent 会话**。
- MVP 按规范化 `cwd` 分组项目，不自动归并到 Git 仓库根目录。
- 一个 **Agent 会话** 同一时间最多拥有一个 **当前草稿** 和多条 **已发送 prompt**。
- 多个 **活动 Agent 会话** 可以同时存在，不能把“当前会话”建模为全局单例。
- PromptBox 记录和编辑 prompt 时必须绑定到明确的 **Agent 会话**，由 `Agent 客户端 + session_id` 唯一确定。
- 一个 **会话工作区** 绑定且只绑定一个 **活动 Agent 会话**。
- 一个 **活动 Agent 会话** 同一时间最多对应一个打开的 **会话工作区**。
- **主窗口** 负责管理会话列表、历史、搜索和设置；**会话工作区** 可以内嵌在 **主窗口** 中，也可以弹出为独立窗口。
- **主窗口** 默认按 **活动 Agent 会话**、**可能已关闭 Agent 会话**、**历史 Agent 会话** 分区展示会话。
- **历史 Agent 会话** 分区默认折叠；搜索可以跨所有会话状态。
- MVP 包含 **托盘入口**；关闭 **主窗口** 默认隐藏到托盘，只有用户选择退出时才停止 **本地采集端点**。
- PromptBox 主程序启动时自动启动 **本地采集端点**；托盘常驻期间端点保持运行。
- **本地采集端点** 启动失败时，**主窗口** 仍可打开，但必须显示采集不可用状态。
- MVP 提供开机自启动设置，但默认关闭，必须由用户显式开启。
- MVP 提供全局 **暂停记录**；暂停期间 **本地采集端点** 仍返回成功，但不写入会话、prompt 或 raw hook 事件。
- **暂停记录** 不等于停止 **本地采集端点**，也不修改 **Agent 客户端** hooks 配置。
- hook 采集器读取 **PromptBox 用户配置** 后，如果发现 **暂停记录** 已开启，应不写 spool、不投递事件，直接成功退出。
- hook 采集器应先读取 **PromptBox 用户配置**；若 **暂停记录** 已开启，必须在读取 stdin 之前退出。
- hook 采集器无法读取 **PromptBox 用户配置** 时，应不读取 stdin、不写 spool、不投递事件，直接成功退出，并只记录不含 prompt 的错误。
- 同一个 **Agent 会话** 同一时间只能有一个 **会话工作区** 实例，避免同一 **当前草稿** 被两个窗口同时编辑。
- **配置向导** 可以一键安装用户级 hooks，但必须先备份并保留用户已有配置。
- **配置向导** 第一版只为项目级 hooks 提供配置片段，不自动修改项目文件。
- 当用户配置无法安全解析或合并时，**配置向导** 应降级为手动配置模式。
- PromptBox 的删除操作只影响自己的本地数据，不修改 Claude Code 或 Codex CLI 的原始 transcript、rollout 或会话文件。
- PromptBox 可以展示原始会话文件路径，但不负责编辑或清理这些文件。
- hook 采集失败时，优先保证 **Agent 客户端** 对话不中断；采集器应尽最大努力写入 **spool 事件**，但最终仍默认成功退出。
- MVP 的 **本地采集端点** 默认使用 `127.0.0.1:9996`，端口可在配置中修改，并通过 token 校验 hook 请求。
- 如果采集端口被占用，PromptBox 主窗口仍可打开，但采集状态必须提示错误；不自动漂移到随机端口。
- 修改 **本地采集端点** 端口后，**配置向导** 必须引导用户同步更新 hooks 配置。
- **本地采集端点** 的 token 存放在 **PromptBox 用户配置** 中，不写入 hook 命令行参数。
- hook 采集器启动后读取 **PromptBox 用户配置**，获取采集端点地址和 token，再向主程序投递事件。
- **Agent 客户端** 的 hook 配置只记录 `promptbox-hook.exe --provider <agent>` 这类命令，不承载 token。
- Windows MVP 默认使用 `%APPDATA%\PromptBox\config.toml` 作为 **PromptBox 用户配置** 路径。
- Windows MVP 默认使用 `%APPDATA%\PromptBox\promptbox.sqlite` 作为数据库路径。
- Windows MVP 默认使用 `%APPDATA%\PromptBox\spool\events.jsonl` 保存 **spool 事件**。
- Windows MVP 默认使用 `%APPDATA%\PromptBox\bin\promptbox-hook.exe` 作为 **hook 可执行文件** 路径。
- `PROMPTBOX_HOME` 环境变量可以覆盖默认用户数据目录。
- PromptBox 主程序启动时应检查 **hook 可执行文件** 版本；缺失或版本不一致时尝试更新。
- **hook 可执行文件** 更新失败时，**主窗口** 必须显示可见错误，配置向导也应显示当前 hook 路径和版本状态。
- **spool 事件** 只是待导入事件，导入成功后才形成 PromptBox 的正式会话记录或 **已发送 prompt**。
- **spool 事件** 第一版保存完整归一化 hook 事件，包括 prompt 内容，因此必须按敏感数据处理。
- **spool 事件** 导入成功后应被截断、轮转或删除，不能长期作为第二份历史库保留。
- **raw hook 事件** 只是诊断数据，默认短期保留 7 天，并允许用户关闭保留。
- 删除某个 **Agent 会话** 时，应同时删除与它相关的 **raw hook 事件**。
- 正式历史以归一化后的 **已发送 prompt** 为准，不以 **raw hook 事件** 为准。
- 正式业务表只保存通用字段和有明确业务价值的少量可选字段，例如 `turn_id`；不长期保存完整 provider payload。
- provider 特有但暂时没有业务价值的字段只进入短期 **raw hook 事件**。
- 搜索覆盖 **会话标题**、**首条 prompt**、全部 **已发送 prompt** 和 **当前草稿**。
- 搜索不覆盖 **raw hook 事件**、**spool 事件** 或 **Agent 客户端** 原始会话文件。
- MVP 提供简单搜索，不做 SQLite FTS、中文分词或高级查询语法。
- **已发送 prompt** 的历史排序使用 hook 采集器捕获事件的时间，而不是数据库导入时间。
- **spool 事件** 必须保存 hook 采集器捕获事件的时间，延迟导入时不能把导入时间当作用户提交时间。
- 第一版不做历史导入；PromptBox 从 hooks 安装后捕获的新事件开始建立正式历史。
- `transcript_path` 只作为 **Agent 会话** 的元信息和未来导入线索，不触发自动读取。
- MVP 必须覆盖从 hook 采集、会话登记、草稿编辑、复制草稿、hook 确认清空、历史回看到简单搜索的完整最小闭环。
- MVP 不包含自动总结、提示词优化、人设提炼、历史导入、导出、自动注入终端、独立弹出工作区窗口、全局快捷键、raw 事件 UI 或项目级自动 hook 安装。
- MVP 只显示 hook 发现的 **Agent 会话**；不扫描 **Agent 客户端** 会话目录，也不允许手动输入 `session_id` 登记会话。
- MVP 只强依赖 `UserPromptSubmit` hook；`SessionStart` 只能作为提前登记或状态更新的增强事件。
- 收到 `UserPromptSubmit` 时，如果对应 **Agent 会话** 不存在，应立即创建。
- MVP 的 **配置向导** 默认只安装 `UserPromptSubmit` hook；`SessionStart` 不默认安装，只作为后续增强或高级选项。
- MVP 必须同时支持 Claude Code 与 Codex CLI 两种 **Agent 客户端** 的 `UserPromptSubmit` 采集。
- Claude Code 与 Codex CLI 的 hook 适配必须隔离，最终统一转换为同一种内部事件。
- **历史 Agent 会话** 可用于回看和复制历史 prompt，但不是发送目标。
- hook 事件只登记或更新 **活动 Agent 会话**，不会自动打开 **会话工作区** 或抢占用户焦点。
- 用户需要在 PromptBox 中明确打开某个 **活动 Agent 会话** 的 **会话工作区** 后，才能编辑它的 **当前草稿**。
- **会话工作区** 是编辑 **当前草稿** 的界面，**当前草稿** 绑定在 **Agent 会话** 上，而不是绑定在窗口实例上。
- MVP 中 **会话工作区** 使用 Milkdown 进行所见即所得编辑，并提供 Markdown 源文本只读查看。
- PromptBox 第一版只支持 **复制草稿**，不负责直接发送或注入终端输入。
- **已发送 prompt** 的事实来源是 **Agent 客户端** hook，而不是 **当前草稿** 或剪贴板内容。
- hook 捕获到的所有用户输入都应记录为 **已发送 prompt**，包括直接在 CLI 中输入的内容。
- **低信息 prompt** 不能在数据层丢弃，只能在展示层弱化显示或由用户筛选隐藏。
- **低信息 prompt** 仍应参与搜索和完整导出，不应影响数据完整性。
- MVP 自动计算 **低信息 prompt** 标记，但不提供逐条手动覆盖。
- 未来如需强调某条 prompt，应优先增加收藏或重要标记，而不是把低信息启发式字段变成用户编辑字段。
- **首条 prompt** 是事实记录，不应被自动总结或用户编辑覆盖。
- **会话标题** 是展示字段，第一版默认从第一条非 **低信息 prompt** 截断生成；如果没有可用内容，则使用 `未命名会话` 或 session 短 ID。
- 用户可以手动修改 **会话标题**；未来自动总结只更新 **会话标题**，不修改 **首条 prompt**。
- **复制草稿** 不会清空 **当前草稿**；只有 hook 捕获到同一 **Agent 会话** 中内容一致的 **已发送 prompt** 后，才自动清空 **当前草稿**。
- 如果 hook 捕获到的 **已发送 prompt** 与 **当前草稿** 内容不一致，PromptBox 记录真实发送内容，但保留 **当前草稿**。
- **当前草稿** 与 **已发送 prompt** 的内容匹配使用同一 hash 规则：对文本做首尾空白 trim 后计算 SHA-256，不修改内部空格、换行、列表缩进或代码块内容。
- 长时间没有收到 hook 事件的 **活动 Agent 会话** 会变为 **可能已关闭 Agent 会话**，但不会自动归档为 **历史 Agent 会话**。
- **活动 Agent 会话** 默认 12 小时没有收到 hook 事件后变为 **可能已关闭 Agent 会话**；该阈值可配置。
- **可能已关闭 Agent 会话** 后续收到 hook 事件时，会恢复为 **活动 Agent 会话**。
- 用户可以手动将 **活动 Agent 会话** 或 **可能已关闭 Agent 会话** 归档为 **历史 Agent 会话**。
- 归档有非空 **当前草稿** 的会话前必须二次确认。
- 归档后草稿仍保留，但 **历史 Agent 会话** 不是发送目标；草稿只能查看或复制，不能作为可发送 **当前草稿** 继续编辑。
- **历史 Agent 会话** 后续收到 hook 事件时，应恢复为 **活动 Agent 会话**，并恢复草稿编辑能力。
- 终端窗口只是用户操作 **Agent 客户端** 的环境，不是 PromptBox 的核心领域对象。

## Example dialogue

> **Dev:** “这个草稿应该绑定到当前终端吗？”
> **Domain expert:** “不，应该绑定到 **Agent 会话**。同一个会话可以 resume 到不同终端窗口，但 `provider + session_id` 才是稳定身份。”

> **Dev:** “我同时开了两个 Codex，会不会写串？”
> **Domain expert:** “不会。每个打开的 **活动 Agent 会话** 都有自己的 **会话工作区**，草稿和已发送 prompt 都绑定到对应的 `Agent 客户端 + session_id`。”

> **Dev:** “我关掉 PromptBox 窗口以后草稿还在吗？”
> **Domain expert:** “在。**当前草稿** 属于 **Agent 会话**，不是窗口实例。重新打开该会话的 **会话工作区** 后继续编辑同一份草稿。”

> **Dev:** “我点按钮以后 PromptBox 会直接发给 Codex 吗？”
> **Domain expert:** “不会。第一版只做 **复制草稿**。只有 **Agent 客户端** hook 捕获到用户真实提交的内容后，才形成 **已发送 prompt**。”

> **Dev:** “复制后没粘贴，草稿会丢吗？”
> **Domain expert:** “不会。复制后只是 **已复制草稿**，只有 hook 确认内容一致的 **已发送 prompt** 后才清空。”

> **Dev:** “像‘同意’、‘继续’这种也要存吗？”
> **Domain expert:** “要存，它们也是 **已发送 prompt**。但它们可以作为 **低信息 prompt** 在历史视图中弱化显示，用户也可以手动筛选隐藏。”

> **Dev:** “会话列表里的标题就是第一条 prompt 吗？”
> **Domain expert:** “不是。**首条 prompt** 是事实字段，**会话标题** 是展示字段。第一版标题默认从第一条非 **低信息 prompt** 截断生成。”

> **Dev:** “昨天的 Codex 会话还能作为发送目标吗？”
> **Domain expert:** “如果长时间没有 hook 事件，它只是 **可能已关闭 Agent 会话**。用户可以确认后继续打开工作区，也可以手动归档为 **历史 Agent 会话**。”

> **Dev:** “我在 PromptBox 里删除一条 prompt，会删掉 Claude Code 原始记录吗？”
> **Domain expert:** “不会。PromptBox 只删除自己的本地数据，不修改 **Agent 客户端** 的原始会话文件。”

> **Dev:** “PromptBox 崩了会不会卡住 Codex？”
> **Domain expert:** “不会。hook 会尽量写入 **spool 事件**，但优先保证 **Agent 客户端** 对话不中断。”

> **Dev:** “PromptBox 没开时写下的 prompt 能补回来吗？”
> **Domain expert:** “可以。**spool 事件** 会临时保存完整归一化事件；导入成功后再清理它。”

> **Dev:** “raw hook 事件是不是另一份永久历史？”
> **Domain expert:** “不是。**raw hook 事件** 只是短期诊断数据，默认 7 天后清理，正式历史是归一化后的 **已发送 prompt**。”

> **Dev:** “搜索会搜到 Claude Code 原始 transcript 里的内容吗？”
> **Domain expert:** “不会。搜索只覆盖 PromptBox 的正式业务数据和 **当前草稿**，不搜原始会话文件。”

> **Dev:** “安装前的 Claude Code 历史会自动进 PromptBox 吗？”
> **Domain expert:** “不会。第一版不做历史导入，只从 hooks 安装后的新事件开始记录。”

> **Dev:** “MVP 可以先只做 Milkdown 编辑器吗？”
> **Domain expert:** “不够。MVP 要验证完整闭环：写草稿、复制、真实提交、hook 记录、确认清空和历史回看。”

> **Dev:** “PromptBox 第一次打开为什么没有会话？”
> **Domain expert:** “MVP 只显示 hook 发现的 **Agent 会话**。安装 hooks 后，在 Claude Code 或 Codex CLI 中提交一次 prompt 即可出现。”

> **Dev:** “没有 SessionStart 事件会不会影响 MVP？”
> **Domain expert:** “不会。MVP 只强依赖 `UserPromptSubmit`，它既能创建 **Agent 会话**，也能形成 **已发送 prompt**。”

## Flagged ambiguities

- “终端”曾被用来指 Claude Code 或 Codex CLI；已统一为 **Agent 客户端**。终端窗口本身不进入核心领域模型。
- “当前会话”容易暗示全局唯一；已改用 **活动 Agent 会话** 表示可继续接收输入的打开会话，并允许多个同时存在。
- “窗口”在产品语境中容易混淆 OS 终端窗口和 PromptBox 编辑界面；已统一使用 **会话工作区** 指 PromptBox 内可编辑的会话绑定界面。
