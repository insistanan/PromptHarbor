import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type AppStatus = {
  appName: string;
  displayName: string;
  version: string;
  hookProtocolVersion: string;
  localEndpoint: string;
  dataPolicy: string;
  promptboxHome: string | null;
  configPath: string | null;
  databasePath: string | null;
  spoolPath: string | null;
  logsDir: string | null;
  hookBinaryPath: string | null;
  recordingPaused: boolean;
  maybeClosedAfterHours: number;
  retainRawHookEvents: boolean;
  rawHookEventsRetentionDays: number;
  autostart: boolean;
  configReady: boolean;
  hookBinaryReady: boolean;
  hookBinaryMessage: string;
  databaseReady: boolean;
  databaseMessage: string;
  sessionCount: number;
  promptEventCount: number;
  collectorReady: boolean;
  collectorMessage: string;
  importedSpoolEvents: number;
  receivedPromptEvents: number;
  startupErrors: string[];
};

type ClaudeHookStatus = {
  settingsPath: string;
  expectedCommand: string;
  installed: boolean;
  readable: boolean;
  message: string;
  backupPath: string | null;
};

type CodexHookStatus = {
  hooksPath: string;
  configPath: string;
  expectedCommand: string;
  hookInstalled: boolean;
  codexHooksEnabled: boolean;
  ready: boolean;
  message: string;
  hooksBackupPath: string | null;
  configBackupPath: string | null;
};

type SessionListItem = {
  provider: string;
  providerLabel: string;
  sessionId: string;
  shortSessionId: string;
  status: string;
  cwd: string | null;
  projectName: string;
  title: string;
  lastHookAt: string | null;
  updatedAt: string;
  promptCount: number;
  hasNonEmptyDraft: boolean;
};

type SessionList = {
  active: SessionListItem[];
  maybeClosed: SessionListItem[];
  archived: SessionListItem[];
};

type ArchiveSessionOutcome = {
  archived: boolean;
  requiresConfirmation: boolean;
  message: string;
};

const menuItems = [
  { label: '会话', active: true },
  { label: '草稿', active: false },
  { label: '搜索', active: false },
  { label: '设置', active: false },
];

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [sessions, setSessions] = useState<SessionList>({
    active: [],
    maybeClosed: [],
    archived: [],
  });
  const [selectedSession, setSelectedSession] = useState<SessionListItem | null>(null);
  const [claudeStatus, setClaudeStatus] = useState<ClaudeHookStatus | null>(null);
  const [codexStatus, setCodexStatus] = useState<CodexHookStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [installingClaude, setInstallingClaude] = useState(false);
  const [installingCodex, setInstallingCodex] = useState(false);

  useEffect(() => {
    let disposed = false;
    const loadStatus = () => {
      invoke<AppStatus>('app_status')
        .then((nextStatus) => {
          if (!disposed) {
            setStatus(nextStatus);
            setError(null);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };
    const loadSessions = () => {
      invoke<SessionList>('list_sessions')
        .then((nextSessions) => {
          if (!disposed) {
            setSessions(nextSessions);
            setSelectedSession((current) => {
              if (!current) {
                return nextSessions.active[0] ?? nextSessions.maybeClosed[0] ?? null;
              }

              return findSession(nextSessions, current.provider, current.sessionId);
            });
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };

    loadStatus();
    loadSessions();
    const loadClaudeStatus = () => {
      invoke<ClaudeHookStatus>('claude_hook_status')
        .then((nextStatus) => {
          if (!disposed) {
            setClaudeStatus(nextStatus);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };
    const loadCodexStatus = () => {
      invoke<CodexHookStatus>('codex_hook_status')
        .then((nextStatus) => {
          if (!disposed) {
            setCodexStatus(nextStatus);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };

    loadClaudeStatus();
    loadCodexStatus();
    const timer = window.setInterval(() => {
      loadStatus();
      loadSessions();
    }, 1000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, []);

  const installClaudeHook = () => {
    setInstallingClaude(true);
    invoke<ClaudeHookStatus>('install_claude_hook')
      .then((nextStatus) => {
        setClaudeStatus(nextStatus);
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setInstallingClaude(false));
  };
  const installCodexHook = () => {
    setInstallingCodex(true);
    invoke<CodexHookStatus>('install_codex_hook')
      .then((nextStatus) => {
        setCodexStatus(nextStatus);
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setInstallingCodex(false));
  };
  const archiveSelectedSession = () => {
    if (!selectedSession || selectedSession.status === 'archived') {
      return;
    }

    archiveSession(selectedSession, false).then((outcome) => {
      if (outcome.requiresConfirmation) {
        const confirmed = window.confirm(outcome.message);
        if (confirmed) {
          archiveSession(selectedSession, true);
        }
      }
    });
  };
  const archiveSession = (session: SessionListItem, force: boolean) =>
    invoke<ArchiveSessionOutcome>('archive_session', {
      provider: session.provider,
      sessionId: session.sessionId,
      force,
    })
      .then((outcome) => {
        if (outcome.archived) {
          return invoke<SessionList>('list_sessions').then((nextSessions) => {
            setSessions(nextSessions);
            setSelectedSession(findSession(nextSessions, session.provider, session.sessionId));
            return outcome;
          });
        }

        return outcome;
      })
      .catch((reason) => {
        setError(String(reason));
        return {
          archived: false,
          requiresConfirmation: false,
          message: String(reason),
        };
      });

  return (
    <main className="app-shell" aria-label="PromptHarbor 工作区">
      <aside className="left-rail" aria-label="导航和会话列表">
        <header className="brand-block">
          <p className="eyebrow">提示港</p>
          <h1>PromptHarbor</h1>
          <p className="status-dot">本地优先</p>
        </header>

        <nav className="menu-list" aria-label="主菜单">
          {menuItems.map((item) => (
            <button className={item.active ? 'menu-item active' : 'menu-item'} key={item.label}>
              {item.label}
            </button>
          ))}
        </nav>

        <section className="session-list" aria-label="Agent 会话列表">
          <div className="rail-heading">
            <span>Agent 会话</span>
            <strong>{status?.sessionCount ?? 0}</strong>
          </div>
          <SessionGroup
            items={sessions.active}
            label="活动"
            onSelect={setSelectedSession}
            selected={selectedSession}
          />
          <SessionGroup
            items={sessions.maybeClosed}
            label="可能已关闭"
            onSelect={setSelectedSession}
            selected={selectedSession}
          />
          <SessionGroup
            items={sessions.archived}
            label="历史"
            onSelect={setSelectedSession}
            selected={selectedSession}
          />
        </section>
      </aside>

      <section className="detail-pane" aria-label="会话详情">
        <header className="detail-header">
          <div>
            <p className="eyebrow">会话工作区</p>
            <h2>{selectedSession?.title ?? '选择一个活动 Agent 会话'}</h2>
          </div>
          <div className="status-strip" aria-label="应用状态">
            <span>{status?.version ? `v${status.version}` : '版本读取中'}</span>
            <span>{status?.localEndpoint ?? '采集端点待连接'}</span>
            <span>
              {status ? (status.collectorReady ? '采集就绪' : '采集不可用') : '采集状态读取中'}
            </span>
            <span>{status?.hookBinaryReady ? 'hook 就绪' : 'hook 待处理'}</span>
          </div>
        </header>

        <section className="runtime-panel" aria-label="本地运行时状态">
          <div className="section-heading">
            <h3>本地运行时</h3>
            <span>{status?.configReady ? '配置就绪' : '配置异常'}</span>
          </div>
          <dl className="runtime-list">
            <div>
              <dt>PromptBox home</dt>
              <dd>{status?.promptboxHome ?? '未初始化'}</dd>
            </div>
            <div>
              <dt>用户配置</dt>
              <dd>{status?.configPath ?? '未初始化'}</dd>
            </div>
            <div>
              <dt>hook 可执行文件</dt>
              <dd>{status?.hookBinaryPath ?? '未初始化'}</dd>
            </div>
            <div>
              <dt>hook 状态</dt>
              <dd className={status?.hookBinaryReady ? 'ok-text' : 'warning-text'}>
                {status?.hookBinaryMessage ?? '等待检测'}
              </dd>
            </div>
            <div>
              <dt>数据库</dt>
              <dd className={status?.databaseReady ? 'ok-text' : 'warning-text'}>
                {status?.databaseMessage ?? '等待初始化'}
              </dd>
            </div>
            <div>
              <dt>采集端点</dt>
              <dd className={status?.collectorReady ? 'ok-text' : 'warning-text'}>
                {status?.collectorMessage ?? '等待启动'}
              </dd>
            </div>
            <div>
              <dt>Agent 会话</dt>
              <dd>{status ? `${status.sessionCount} 个` : '0 个'}</dd>
            </div>
            <div>
              <dt>正式 prompt</dt>
              <dd>{status ? `${status.promptEventCount} 条` : '0 条'}</dd>
            </div>
            <div>
              <dt>已采集事件</dt>
              <dd>{status ? `${status.receivedPromptEvents} 条` : '0 条'}</dd>
            </div>
            <div>
              <dt>spool 导入</dt>
              <dd>{status ? `${status.importedSpoolEvents} 条` : '0 条'}</dd>
            </div>
          </dl>
          {status?.startupErrors.length ? (
            <div className="runtime-errors">
              {status.startupErrors.map((item) => (
                <p key={item}>{item}</p>
              ))}
            </div>
          ) : null}
        </section>

        <section className="wizard-panel" aria-label="Claude Code 配置向导">
          <div className="section-heading">
            <h3>Claude Code</h3>
            <span className={claudeStatus?.installed ? 'ok-text' : 'warning-text'}>
              {claudeStatus?.installed ? 'hook 已安装' : 'hook 未安装'}
            </span>
          </div>
          <dl className="runtime-list">
            <div>
              <dt>配置文件</dt>
              <dd>{claudeStatus?.settingsPath ?? '读取中'}</dd>
            </div>
            <div>
              <dt>hook 命令</dt>
              <dd>{claudeStatus?.expectedCommand ?? '读取中'}</dd>
            </div>
            <div>
              <dt>检测结果</dt>
              <dd>{claudeStatus?.message ?? '等待检测'}</dd>
            </div>
            {claudeStatus?.backupPath ? (
              <div>
                <dt>备份文件</dt>
                <dd>{claudeStatus.backupPath}</dd>
              </div>
            ) : null}
          </dl>
          <div className="wizard-actions">
            <button
              className="primary-action"
              disabled={installingClaude || claudeStatus?.installed}
              onClick={installClaudeHook}
              type="button"
            >
              {installingClaude ? '安装中' : '安装用户级 hook'}
            </button>
          </div>
        </section>

        <section className="wizard-panel" aria-label="Codex CLI 配置向导">
          <div className="section-heading">
            <h3>Codex CLI</h3>
            <span className={codexStatus?.ready ? 'ok-text' : 'warning-text'}>
              {codexStatus?.ready ? 'hook 可用' : 'hook 未就绪'}
            </span>
          </div>
          <dl className="runtime-list">
            <div>
              <dt>hooks.json</dt>
              <dd>{codexStatus?.hooksPath ?? '读取中'}</dd>
            </div>
            <div>
              <dt>config.toml</dt>
              <dd>{codexStatus?.configPath ?? '读取中'}</dd>
            </div>
            <div>
              <dt>hook 命令</dt>
              <dd>{codexStatus?.expectedCommand ?? '读取中'}</dd>
            </div>
            <div>
              <dt>hook 状态</dt>
              <dd className={codexStatus?.hookInstalled ? 'ok-text' : 'warning-text'}>
                {codexStatus?.hookInstalled ? '已安装' : '未安装'}
              </dd>
            </div>
            <div>
              <dt>feature</dt>
              <dd className={codexStatus?.codexHooksEnabled ? 'ok-text' : 'warning-text'}>
                {codexStatus?.codexHooksEnabled ? 'codex_hooks 已开启' : 'codex_hooks 未开启'}
              </dd>
            </div>
            <div>
              <dt>检测结果</dt>
              <dd>{codexStatus?.message ?? '等待检测'}</dd>
            </div>
            {codexStatus?.hooksBackupPath ? (
              <div>
                <dt>hooks 备份</dt>
                <dd>{codexStatus.hooksBackupPath}</dd>
              </div>
            ) : null}
            {codexStatus?.configBackupPath ? (
              <div>
                <dt>config 备份</dt>
                <dd>{codexStatus.configBackupPath}</dd>
              </div>
            ) : null}
          </dl>
          <div className="wizard-actions">
            <button
              className="primary-action"
              disabled={installingCodex || codexStatus?.ready}
              onClick={installCodexHook}
              type="button"
            >
              {installingCodex ? '安装中' : '安装用户级 hook'}
            </button>
          </div>
        </section>

        <section className="prompt-history" aria-label="prompt 历史">
          <div className="section-heading">
            <h3>prompt 历史</h3>
            <span>{selectedSession ? selectedSession.providerLabel : 'hook-first'}</span>
          </div>
          {selectedSession ? (
            <div className="session-detail">
              <dl className="runtime-list">
                <div>
                  <dt>Agent 客户端</dt>
                  <dd>{selectedSession.providerLabel}</dd>
                </div>
                <div>
                  <dt>session ID</dt>
                  <dd>{selectedSession.shortSessionId}</dd>
                </div>
                <div>
                  <dt>项目</dt>
                  <dd>{selectedSession.projectName}</dd>
                </div>
                <div>
                  <dt>状态</dt>
                  <dd>{sessionStatusLabel(selectedSession.status)}</dd>
                </div>
                <div>
                  <dt>最近 hook</dt>
                  <dd>{formatDateTime(selectedSession.lastHookAt)}</dd>
                </div>
                <div>
                  <dt>已发送 prompt</dt>
                  <dd>{selectedSession.promptCount} 条</dd>
                </div>
              </dl>
              <div className="wizard-actions">
                <button
                  className="secondary-action"
                  disabled={selectedSession.status === 'archived'}
                  onClick={archiveSelectedSession}
                  type="button"
                >
                  归档
                </button>
              </div>
            </div>
          ) : (
            <div className="empty-state">
              <p className="empty-title">等待第一条已发送 prompt</p>
              <p>只记录用户真实提交的 prompt，模型回复不会进入 PromptHarbor。</p>
            </div>
          )}
        </section>

        <section className="draft-panel" aria-label="当前草稿">
          <div className="section-heading">
            <h3>当前草稿</h3>
            <span>Milkdown 待接入</span>
          </div>
          <div className="draft-surface">
            <p>当前骨架已经打通 React 到 Tauri 的最小 IPC。下一步会接入本地配置、hook 采集端点和真实会话数据。</p>
            <div className="draft-input" aria-label="草稿输入占位">
              选择活动会话后，在这里编写下一轮 Markdown prompt。
            </div>
            {error ? <p className="error-text">IPC 调用失败：{error}</p> : null}
          </div>
        </section>
      </section>
    </main>
  );
}

function SessionGroup({
  items,
  label,
  onSelect,
  selected,
}: {
  items: SessionListItem[];
  label: string;
  onSelect: (session: SessionListItem) => void;
  selected: SessionListItem | null;
}) {
  return (
    <section className="session-group-block" aria-label={`${label} Agent 会话`}>
      <div className="session-group-heading">
        <span>{label}</span>
        <em>{items.length}</em>
      </div>
      {items.length ? (
        <div className="session-groups">
          {items.map((session) => {
            const active =
              selected?.provider === session.provider && selected?.sessionId === session.sessionId;
            return (
              <button
                className={active ? 'session-group active' : 'session-group'}
                key={`${session.provider}:${session.sessionId}`}
                onClick={() => onSelect(session)}
                type="button"
              >
                <span>
                  <strong>{session.title}</strong>
                  <small>
                    {session.providerLabel} · {session.shortSessionId} · {session.projectName}
                  </small>
                  <small>{formatDateTime(session.updatedAt)}</small>
                </span>
                <em>{session.promptCount}</em>
              </button>
            );
          })}
        </div>
      ) : (
        <p className="session-empty">暂无</p>
      )}
    </section>
  );
}

function findSession(
  sessions: SessionList,
  provider: string,
  sessionId: string,
): SessionListItem | null {
  return (
    [...sessions.active, ...sessions.maybeClosed, ...sessions.archived].find(
      (session) => session.provider === provider && session.sessionId === sessionId,
    ) ?? null
  );
}

function sessionStatusLabel(status: string) {
  if (status === 'active') {
    return '活动';
  }
  if (status === 'maybe_closed') {
    return '可能已关闭';
  }
  if (status === 'archived') {
    return '历史';
  }
  return status;
}

function formatDateTime(value: string | null) {
  if (!value) {
    return '暂无';
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}
