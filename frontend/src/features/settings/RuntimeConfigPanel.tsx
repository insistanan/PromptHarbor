import type { RuntimeConfigDraft } from '../../appTypes';

export function RuntimeConfigPanel({
  configDirty,
  configDraft,
  configSaving,
  onChange,
  onSave,
}: {
  configDirty: boolean;
  configDraft: RuntimeConfigDraft | null;
  configSaving: boolean;
  onChange: (patch: Partial<RuntimeConfigDraft>) => void;
  onSave: () => void;
}) {
  return (
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
            onChange={(event) => onChange({ autostart: event.currentTarget.checked })}
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
            onChange={(event) => onChange({ recordingPaused: event.currentTarget.checked })}
            type="checkbox"
          />
        </label>
        <label className="config-field">
          <span>本地采集端点</span>
          <input
            onChange={(event) => onChange({ localEndpoint: event.currentTarget.value })}
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
                onChange({ maybeClosedAfterHours: event.currentTarget.value })
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
                onChange({
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
              onChange({ retainRawHookEvents: event.currentTarget.checked })
            }
            type="checkbox"
          />
        </label>
      </div>
      <div className="wizard-actions">
        <button
          className="primary-action"
          disabled={!configDirty || configSaving || !configDraft}
          onClick={onSave}
          type="button"
        >
          {configSaving ? '保存中' : '保存配置'}
        </button>
      </div>
    </section>
  );
}
