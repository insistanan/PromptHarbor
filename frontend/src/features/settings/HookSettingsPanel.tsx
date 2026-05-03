import type { RuntimeHookSettingsItem } from './useRuntimeSettingsState';

export function HookSettingsPanel({ item }: { item: RuntimeHookSettingsItem }) {
  const status = item.status;
  const installingOrUninstalling = item.installing || item.uninstalling;
  const installDisabled =
    installingOrUninstalling || (item.provider === 'codex' ? status?.ready : status?.installed);
  const uninstallDisabled = installingOrUninstalling || !status?.installed;

  return (
    <section className="wizard-panel" aria-label={`${item.title} 配置向导`}>
      <div className="section-heading">
        <h3>{item.title}</h3>
        <span className={hookHeaderClass(item)}>
          {hookHeaderLabel(item)}
        </span>
      </div>
      <dl className="runtime-list">
        {status?.configPaths.length ? (
          status.configPaths.map((path) => (
            <div key={path.label}>
              <dt>{path.label}</dt>
              <dd>{path.path}</dd>
            </div>
          ))
        ) : (
          <div>
            <dt>配置文件</dt>
            <dd>读取中</dd>
          </div>
        )}
        <div>
          <dt>钩子命令</dt>
          <dd>{status?.expectedCommand ?? '读取中'}</dd>
        </div>
        {item.provider === 'codex' ? (
          <>
            <div>
              <dt>钩子状态</dt>
              <dd className={status?.installed ? 'ok-text' : 'warning-text'}>
                {status?.installed ? '已安装' : '未安装'}
              </dd>
            </div>
            <div>
              <dt>实验功能</dt>
              <dd className={status?.codexHooksEnabled ? 'ok-text' : 'warning-text'}>
                {status?.codexHooksEnabled ? 'codex_hooks 已开启' : 'codex_hooks 未开启'}
              </dd>
            </div>
          </>
        ) : null}
        <div>
          <dt>检测结果</dt>
          <dd>{status?.message ?? '等待检测'}</dd>
        </div>
        {status?.backupPaths.map((path) => (
          <div key={path.label}>
            <dt>{path.label}</dt>
            <dd>{path.path}</dd>
          </div>
        ))}
      </dl>
      <div className="wizard-actions">
        <button
          className="primary-action"
          disabled={installDisabled}
          onClick={item.onInstall}
          type="button"
        >
          {item.installing ? '安装中' : '安装用户级钩子'}
        </button>
        <button
          className="secondary-action"
          disabled={uninstallDisabled}
          onClick={item.onUninstall}
          type="button"
        >
          {item.uninstalling ? '取消中' : '取消钩子'}
        </button>
      </div>
    </section>
  );
}

function hookHeaderLabel(item: RuntimeHookSettingsItem) {
  if (item.provider === 'codex') {
    return item.status?.ready ? '钩子可用' : '钩子未就绪';
  }

  return item.status?.installed ? '钩子已安装' : '钩子未安装';
}

function hookHeaderClass(item: RuntimeHookSettingsItem) {
  if (item.provider === 'codex') {
    return item.status?.ready ? 'ok-text' : 'warning-text';
  }

  return item.status?.installed ? 'ok-text' : 'warning-text';
}
