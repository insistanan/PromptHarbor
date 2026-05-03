import { useEffect, useState } from 'react';
import * as api from '../../api';

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
  pausedPromptEvents: number;
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

type RuntimeConfigDraft = {
  localEndpoint: string;
  recordingPaused: boolean;
  maybeClosedAfterHours: string;
  retainRawHookEvents: boolean;
  rawHookEventsRetentionDays: string;
  autostart: boolean;
};

export function RuntimeSettings({
  onError,
  onNotice,
  onStatusChange,
  status,
}: {
  status: AppStatus | null;
  onStatusChange: (status: AppStatus) => void;
  onNotice: (message: string) => void;
  onError: (message: string | null) => void;
}) {
  const [claudeStatus, setClaudeStatus] = useState<ClaudeHookStatus | null>(null);
  const [codexStatus, setCodexStatus] = useState<CodexHookStatus | null>(null);
  const [installingClaude, setInstallingClaude] = useState(false);
  const [installingCodex, setInstallingCodex] = useState(false);
  const [uninstallingClaude, setUninstallingClaude] = useState(false);
  const [uninstallingCodex, setUninstallingCodex] = useState(false);
  const [configDraft, setConfigDraft] = useState<RuntimeConfigDraft | null>(null);
  const [configDirty, setConfigDirty] = useState(false);
  const [configSaving, setConfigSaving] = useState(false);

  useEffect(() => {
    let disposed = false;

    api
      .getClaudeHookStatus<ClaudeHookStatus>()
      .then((nextStatus) => {
        if (!disposed) {
          setClaudeStatus(nextStatus);
        }
      })
      .catch((reason) => {
        if (!disposed) {
          onError(String(reason));
        }
      });

    api
      .getCodexHookStatus<CodexHookStatus>()
      .then((nextStatus) => {
        if (!disposed) {
          setCodexStatus(nextStatus);
        }
      })
      .catch((reason) => {
        if (!disposed) {
          onError(String(reason));
        }
      });

    return () => {
      disposed = true;
    };
  }, [onError]);

  useEffect(() => {
    if (!status || configDirty || configSaving) {
      return;
    }

    setConfigDraft(configDraftFromStatus(status));
  }, [configDirty, configSaving, status]);

  const updateConfigDraft = (patch: Partial<RuntimeConfigDraft>) => {
    setConfigDraft((current) => {
      const base = current ?? (status ? configDraftFromStatus(status) : emptyConfigDraft());
      return { ...base, ...patch };
    });
    setConfigDirty(true);
  };

  const saveRuntimeConfig = () => {
    if (!configDraft) {
      return;
    }

    const maybeClosedAfterHours = Number(configDraft.maybeClosedAfterHours);
    const rawHookEventsRetentionDays = Number(configDraft.rawHookEventsRetentionDays);
    if (!Number.isFinite(maybeClosedAfterHours) || maybeClosedAfterHours < 1) {
      onError('可能关闭判定时间必须大于 0 小时');
      return;
    }
    if (!Number.isFinite(rawHookEventsRetentionDays) || rawHookEventsRetentionDays < 0) {
      onError('原始钩子事件保留天数不能小于 0');
      return;
    }

    setConfigSaving(true);
    api
      .updateRuntimeConfig<AppStatus>({
        localEndpoint: configDraft.localEndpoint,
        recordingPaused: configDraft.recordingPaused,
        maybeClosedAfterHours,
        retainRawHookEvents: configDraft.retainRawHookEvents,
        rawHookEventsRetentionDays,
        autostart: configDraft.autostart,
      })
      .then((nextStatus) => {
        onStatusChange(nextStatus);
        setConfigDraft(configDraftFromStatus(nextStatus));
        setConfigDirty(false);
        onNotice('运行配置已保存');
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setConfigSaving(false));
  };

  const installClaudeHook = () => {
    setInstallingClaude(true);
    api
      .installClaudeHook<ClaudeHookStatus>()
      .then((nextStatus) => {
        setClaudeStatus(nextStatus);
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setInstallingClaude(false));
  };

  const uninstallClaudeHook = () => {
    setUninstallingClaude(true);
    api
      .uninstallClaudeHook<ClaudeHookStatus>()
      .then((nextStatus) => {
        setClaudeStatus(nextStatus);
        onNotice('Claude Code 钩子已取消');
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setUninstallingClaude(false));
  };

  const installCodexHook = () => {
    setInstallingCodex(true);
    api
      .installCodexHook<CodexHookStatus>()
      .then((nextStatus) => {
        setCodexStatus(nextStatus);
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setInstallingCodex(false));
  };

  const uninstallCodexHook = () => {
    setUninstallingCodex(true);
    api
      .uninstallCodexHook<CodexHookStatus>()
      .then((nextStatus) => {
        setCodexStatus(nextStatus);
        onNotice('Codex CLI 钩子已取消');
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setUninstallingCodex(false));
  };

  return (
    <div className="settings-grid">
      <section className="config-panel" aria-label="运行配置">
        <div className="section-heading">
          <h3>运行配置</h3>
          <span>{configDirty ? '有未保存修改' : '已同步'}</span>
        </div>
        <div className="config-form">
          <label className="switch-row">
            <span>
              <strong>开机启动</strong>
              <small>写入当前用户的 Windows 启动项</small>
            </span>
            <input
              checked={configDraft?.autostart ?? false}
              onChange={(event) => updateConfigDraft({ autostart: event.currentTarget.checked })}
              type="checkbox"
            />
          </label>
          <label className="switch-row">
            <span>
              <strong>暂停记录</strong>
              <small>开启后钩子仍可运行，但不会写入新提示词</small>
            </span>
            <input
              checked={configDraft?.recordingPaused ?? false}
              onChange={(event) =>
                updateConfigDraft({ recordingPaused: event.currentTarget.checked })
              }
              type="checkbox"
            />
          </label>
          <label className="config-field">
            <span>本地采集端点</span>
            <input
              onChange={(event) => updateConfigDraft({ localEndpoint: event.currentTarget.value })}
              value={configDraft?.localEndpoint ?? ''}
            />
            <small>端口修改保存后写入配置；监听端口需重启应用后切换。</small>
          </label>
          <div className="config-field-grid">
            <label className="config-field">
              <span>可能关闭判定</span>
              <input
                min="1"
                onChange={(event) =>
                  updateConfigDraft({ maybeClosedAfterHours: event.currentTarget.value })
                }
                type="number"
                value={configDraft?.maybeClosedAfterHours ?? '12'}
              />
              <small>小时</small>
            </label>
            <label className="config-field">
              <span>原始事件保留天数</span>
              <input
                min="0"
                onChange={(event) =>
                  updateConfigDraft({
                    rawHookEventsRetentionDays: event.currentTarget.value,
                  })
                }
                type="number"
                value={configDraft?.rawHookEventsRetentionDays ?? '7'}
              />
              <small>0 表示启动后即过期</small>
            </label>
          </div>
          <label className="switch-row">
            <span>
              <strong>保留原始钩子事件</strong>
              <small>仅用于短期诊断，正式历史仍只保存用户提示词</small>
            </span>
            <input
              checked={configDraft?.retainRawHookEvents ?? true}
              onChange={(event) =>
                updateConfigDraft({ retainRawHookEvents: event.currentTarget.checked })
              }
              type="checkbox"
            />
          </label>
        </div>
        <div className="wizard-actions">
          <button
            className="primary-action"
            disabled={!configDirty || configSaving || !configDraft}
            onClick={saveRuntimeConfig}
            type="button"
          >
            {configSaving ? '保存中' : '保存配置'}
          </button>
        </div>
      </section>

      <section className="runtime-panel" aria-label="本地运行时状态">
        <div className="section-heading">
          <h3>本地运行时</h3>
          <span>{status?.recordingPaused ? '记录暂停' : '记录中'}</span>
        </div>
        <dl className="runtime-list">
          <div>
            <dt>数据目录</dt>
            <dd>{status?.promptboxHome ?? '未初始化'}</dd>
          </div>
          <div>
            <dt>用户配置</dt>
            <dd>{status?.configPath ?? '未初始化'}</dd>
          </div>
          <div>
            <dt>钩子可执行文件</dt>
            <dd>{status?.hookBinaryPath ?? '未初始化'}</dd>
          </div>
          <div>
            <dt>钩子状态</dt>
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
            <dt>记录状态</dt>
            <dd className={status?.recordingPaused ? 'warning-text' : 'ok-text'}>
              {status?.recordingPaused ? '已暂停，不写入提示词' : '记录中'}
            </dd>
          </div>
          <div>
            <dt>智能体会话</dt>
            <dd>{status ? `${status.sessionCount} 个` : '0 个'}</dd>
          </div>
          <div>
            <dt>正式提示词</dt>
            <dd>{status ? `${status.promptEventCount} 条` : '0 条'}</dd>
          </div>
          <div>
            <dt>已采集事件</dt>
            <dd>{status ? `${status.receivedPromptEvents} 条` : '0 条'}</dd>
          </div>
          <div>
            <dt>暂停丢弃</dt>
            <dd>{status ? `${status.pausedPromptEvents} 条` : '0 条'}</dd>
          </div>
          <div>
            <dt>启动导入</dt>
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
            {claudeStatus?.installed ? '钩子已安装' : '钩子未安装'}
          </span>
        </div>
        <dl className="runtime-list">
          <div>
            <dt>配置文件</dt>
            <dd>{claudeStatus?.settingsPath ?? '读取中'}</dd>
          </div>
          <div>
            <dt>钩子命令</dt>
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
            disabled={installingClaude || uninstallingClaude || claudeStatus?.installed}
            onClick={installClaudeHook}
            type="button"
          >
            {installingClaude ? '安装中' : '安装用户级钩子'}
          </button>
          <button
            className="secondary-action"
            disabled={installingClaude || uninstallingClaude || !claudeStatus?.installed}
            onClick={uninstallClaudeHook}
            type="button"
          >
            {uninstallingClaude ? '取消中' : '取消钩子'}
          </button>
        </div>
      </section>

      <section className="wizard-panel" aria-label="Codex CLI 配置向导">
        <div className="section-heading">
          <h3>Codex CLI</h3>
          <span className={codexStatus?.ready ? 'ok-text' : 'warning-text'}>
            {codexStatus?.ready ? '钩子可用' : '钩子未就绪'}
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
            <dt>钩子命令</dt>
            <dd>{codexStatus?.expectedCommand ?? '读取中'}</dd>
          </div>
          <div>
            <dt>钩子状态</dt>
            <dd className={codexStatus?.hookInstalled ? 'ok-text' : 'warning-text'}>
              {codexStatus?.hookInstalled ? '已安装' : '未安装'}
            </dd>
          </div>
          <div>
            <dt>实验功能</dt>
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
            disabled={installingCodex || uninstallingCodex || codexStatus?.ready}
            onClick={installCodexHook}
            type="button"
          >
            {installingCodex ? '安装中' : '安装用户级钩子'}
          </button>
          <button
            className="secondary-action"
            disabled={installingCodex || uninstallingCodex || !codexStatus?.hookInstalled}
            onClick={uninstallCodexHook}
            type="button"
          >
            {uninstallingCodex ? '取消中' : '取消钩子'}
          </button>
        </div>
      </section>
    </div>
  );
}

function configDraftFromStatus(status: AppStatus): RuntimeConfigDraft {
  return {
    localEndpoint: status.localEndpoint,
    recordingPaused: status.recordingPaused,
    maybeClosedAfterHours: String(status.maybeClosedAfterHours),
    retainRawHookEvents: status.retainRawHookEvents,
    rawHookEventsRetentionDays: String(status.rawHookEventsRetentionDays),
    autostart: status.autostart,
  };
}

function emptyConfigDraft(): RuntimeConfigDraft {
  return {
    localEndpoint: '127.0.0.1:9996',
    recordingPaused: false,
    maybeClosedAfterHours: '12',
    retainRawHookEvents: true,
    rawHookEventsRetentionDays: '7',
    autostart: false,
  };
}
