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
          </span>
          <input
            checked={configDraft?.recordingPaused ?? false}
            onChange={(event) => onChange({ recordingPaused: event.currentTarget.checked })}
            type="checkbox"
          />
        </label>
        <label className="config-field">
          <span>本地采集端点</span>
          {/* 保存后写入配置，监听端口随应用重启切换。 */}
          <input
            onChange={(event) => onChange({ localEndpoint: event.currentTarget.value })}
            value={configDraft?.localEndpoint ?? ''}
          />
        </label>
        <div className="config-field-grid">
          <label className="config-field">
            <span>关闭判定（小时）</span>
            <input
              min="1"
              onChange={(event) =>
                onChange({ maybeClosedAfterHours: event.currentTarget.value })
              }
              type="number"
              value={configDraft?.maybeClosedAfterHours ?? '12'}
            />
          </label>
          <label className="config-field">
            <span>原始事件保留天数</span>
            {/* 0 表示启动后即过期。 */}
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
          </label>
        </div>
        <label className="switch-row">
          <span>
            <strong>保留原始钩子事件</strong>
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
