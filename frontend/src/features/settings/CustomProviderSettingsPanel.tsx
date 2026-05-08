import type { CustomProviderProtocol } from '../../appTypes';
import { useCustomProviderSettingsState } from './useCustomProviderSettingsState';

const protocolOptions: Array<{
  value: CustomProviderProtocol;
  label: string;
  supported: boolean;
}> = [
  { value: 'openai_chat', label: 'OpenAI Chat', supported: true },
  { value: 'openai_responses', label: 'OpenAI Responses', supported: false },
  { value: 'anthropic', label: 'Anthropic', supported: false },
  { value: 'gemini', label: 'Gemini', supported: false },
  { value: 'zhipu_v4', label: '智谱 v4', supported: false },
];

export function CustomProviderSettingsPanel({
  onError,
  onNotice,
}: {
  onNotice: (message: string) => void;
  onError: (message: string | null) => void;
}) {
  const state = useCustomProviderSettingsState({
    onError,
    onNotice,
  });
  const currentProtocol = protocolOptions.find(
    (option) => option.value === state.draft.protocol,
  );
  const protocolSupported = currentProtocol?.supported ?? false;
  const secretReady =
    state.draft.apiKey.trim().length > 0 || state.draft.secretConfigured;
  const canSave =
    state.draftDirty &&
    !state.saving &&
    state.draft.name.trim().length > 0 &&
    (!state.draft.enabled || protocolSupported);
  const canTest =
    !state.testing &&
    protocolSupported &&
    state.draft.name.trim().length > 0 &&
    state.draft.baseUrl.trim().length > 0 &&
    state.draft.defaultModel.trim().length > 0 &&
    secretReady;

  return (
    <section className="config-panel provider-panel" aria-label="自定义供应商">
      <div className="section-heading">
        <h3>自定义供应商</h3>
        <span>
          {state.providersLoading
            ? '读取中'
            : state.providers.length
              ? `已配置 ${state.providers.length} 项`
              : '尚未配置'}
        </span>
      </div>
      <div className="provider-split-layout">
        <div className="provider-left-col">
          <div className="provider-toolbar">
            <button
              className="secondary-action"
              onClick={state.createProvider}
              type="button"
            >
              新增供应商
            </button>
            <p className="provider-toolbar-note">
              仅 OpenAI Chat 已打通真实请求；其他协议先占位展示。
            </p>
          </div>
          <div className="provider-list" aria-label="供应商列表">
            {state.providers.length ? (
              state.providers.map((provider) => (
                <button
                  className={
                    provider.id === state.draft.providerId
                      ? 'provider-list-item active'
                      : 'provider-list-item'
                  }
                  key={provider.id}
                  onClick={() => state.selectProvider(provider.id)}
                  type="button"
                >
                  <div className="provider-list-item-header">
                    <strong>{provider.name}</strong>
                    <span>{provider.enabled ? '已启用' : '未启用'}</span>
                  </div>
                  <div className="provider-list-item-meta">
                    <span>{provider.protocolLabel}</span>
                    <span>{provider.secretConfigured ? '密钥已配置' : '密钥未配置'}</span>
                    <span>{provider.supported ? '已支持' : '暂未支持'}</span>
                  </div>
                </button>
              ))
            ) : (
              <div className="provider-empty">
                <p>还没有自定义供应商。先新增一个 OpenAI Chat 兼容配置，再测试连接。</p>
              </div>
            )}
          </div>
        </div>
        <div className="provider-right-col">
          <div className="provider-form-actions">
            <button
              className="primary-action"
              disabled={!canSave}
              onClick={state.saveProvider}
              type="button"
            >
              {state.saving ? '保存中' : '保存供应商'}
            </button>
            <button
              className="secondary-action"
              disabled={!canTest}
              onClick={state.testProvider}
              type="button"
            >
              {state.testing ? '测试中' : '测试连接'}
            </button>
            <button
              className="secondary-action"
              disabled={!state.draft.providerId || state.deleting}
              onClick={state.deleteProvider}
              type="button"
            >
              {state.deleting ? '删除中' : '删除供应商'}
            </button>
          </div>
          <div className="config-form">
            {state.testResult ? (
              <div className="provider-test-result" role="status">
                <strong>{state.testResult.message}</strong>
                <small>返回预览：{state.testResult.assistantPreview}</small>
              </div>
            ) : null}
            <label className="config-field">
              <span>名称</span>
              <input
                onChange={(event) => state.updateDraft({ name: event.currentTarget.value })}
                placeholder="例如：OpenAI 兼容供应商"
                value={state.draft.name}
              />
            </label>
            <div className="config-field-grid">
              <label className="config-field">
                <span>协议</span>
                <select
                  onChange={(event) =>
                    state.updateDraft({
                      protocol: event.currentTarget.value as CustomProviderProtocol,
                    })
                  }
                  value={state.draft.protocol}
                >
                  {protocolOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.supported ? option.label : `${option.label}（暂未支持）`}
                    </option>
                  ))}
                </select>
              </label>
              <label className="switch-row">
                <span>
                  <strong>启用该供应商</strong>
                  <small>
                    {!protocolSupported && state.draft.enabled
                      ? '当前协议暂未支持，不能启用'
                      : '后续功能只会使用已启用的供应商'}
                  </small>
                </span>
                <input
                  checked={state.draft.enabled}
                  onChange={(event) =>
                    state.updateDraft({ enabled: event.currentTarget.checked })
                  }
                  type="checkbox"
                />
              </label>
            </div>
            <label className="config-field">
              <span>接口地址</span>
              <small>填写兼容 OpenAI Chat 的基础地址或完整 `/chat/completions` 地址。</small>
              <input
                onChange={(event) =>
                  state.updateDraft({ baseUrl: event.currentTarget.value })
                }
                placeholder="https://example.com/v1"
                value={state.draft.baseUrl}
              />
            </label>
            <div className="config-field-grid">
              <label className="config-field">
                <span>API 密钥</span>
                <small>
                  {state.draft.secretConfigured
                    ? '已配置密钥；留空表示保持当前密钥不变。'
                    : '保存到本地配置，仅测试连接和后续增强能力会使用。'}
                </small>
                <input
                  onChange={(event) =>
                    state.updateDraft({ apiKey: event.currentTarget.value })
                  }
                  placeholder={state.draft.secretConfigured ? '留空则保持不变' : '输入 API 密钥'}
                  type="password"
                  value={state.draft.apiKey}
                />
              </label>
              <label className="config-field">
                <span>默认模型</span>
                <small>测试连接和后续提示词增强默认使用这个模型。</small>
                <input
                  onChange={(event) =>
                    state.updateDraft({ defaultModel: event.currentTarget.value })
                  }
                  placeholder="输入默认模型 ID"
                  value={state.draft.defaultModel}
                />
              </label>
            </div>
            {!protocolSupported ? (
              <p className="provider-inline-note">
                当前协议仅作为配置占位展示，暂不支持测试连接或真实请求。
              </p>
            ) : null}
          </div>
        </div>
      </div>
    </section>
  );
}
