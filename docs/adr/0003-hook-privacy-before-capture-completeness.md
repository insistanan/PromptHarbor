# hook 采集优先保护隐私而不是完整捕获

PromptBox 的 hook 采集器启动后必须先读取 PromptBox 用户配置；如果配置不可读，或配置中暂停记录已开启，采集器不读取 stdin、不写 spool、不投递事件，并直接成功退出。这个决策会导致配置损坏或暂停期间的 prompt 无法补回，但它保证了用户选择暂停或配置状态不明时，PromptBox 不会处理 prompt 内容。

**Considered Options**

- 配置失败时继续读取 stdin 并写 spool：能减少丢失，但无法判断用户是否已暂停记录。
- 配置失败或暂停时不读取 stdin：会丢失部分 prompt，但隐私边界最清楚。
- 让 hook 读取 stdin 后再判断暂停：实现简单，但暂停期间仍会处理 prompt 内容。

**Consequences**

- PromptBox 主程序和配置向导必须清晰显示配置健康状态。
- hook 错误日志不得包含 prompt 内容。
- 采集完整性低于隐私承诺；PromptBox 不能把自己设计成阻断 Agent 客户端的关键链路。
