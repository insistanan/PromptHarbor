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
  startupErrors: string[];
};

const emptySessions = [
  { label: '活动', count: 0, detail: '真实提交 prompt 后出现' },
  { label: '可能已关闭', count: 0, detail: '超过阈值未收到 hook' },
  { label: '历史', count: 0, detail: '只回看和复制' },
];

const menuItems = [
  { label: '会话', active: true },
  { label: '草稿', active: false },
  { label: '搜索', active: false },
  { label: '设置', active: false },
];

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppStatus>('app_status')
      .then(setStatus)
      .catch((reason) => setError(String(reason)));
  }, []);

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
            <strong>0</strong>
          </div>
          <div className="session-groups">
            {emptySessions.map((item) => (
              <button className="session-group" key={item.label}>
                <span>
                  <strong>{item.label}</strong>
                  <small>{item.detail}</small>
                </span>
                <em>{item.count}</em>
              </button>
            ))}
          </div>
        </section>
      </aside>

      <section className="detail-pane" aria-label="会话详情">
        <header className="detail-header">
          <div>
            <p className="eyebrow">会话工作区</p>
            <h2>选择一个活动 Agent 会话</h2>
          </div>
          <div className="status-strip" aria-label="应用状态">
            <span>{status?.version ? `v${status.version}` : '版本读取中'}</span>
            <span>{status?.localEndpoint ?? '采集端点待连接'}</span>
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
          </dl>
          {status?.startupErrors.length ? (
            <div className="runtime-errors">
              {status.startupErrors.map((item) => (
                <p key={item}>{item}</p>
              ))}
            </div>
          ) : null}
        </section>

        <section className="prompt-history" aria-label="prompt 历史">
          <div className="section-heading">
            <h3>prompt 历史</h3>
            <span>hook-first</span>
          </div>
          <div className="empty-state">
            <p className="empty-title">等待第一条已发送 prompt</p>
            <p>只记录用户真实提交的 prompt，模型回复不会进入 PromptHarbor。</p>
          </div>
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
