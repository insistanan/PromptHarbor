import type { AppStatus } from '../../appTypes';
import { CustomProviderSettingsPanel } from './CustomProviderSettingsPanel';
import { HookSettingsPanel } from './HookSettingsPanel';
import { RuntimeConfigPanel } from './RuntimeConfigPanel';
import { RuntimeStatusPanel } from './RuntimeStatusPanel';
import { useRuntimeSettingsState } from './useRuntimeSettingsState';

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
  const settings = useRuntimeSettingsState({
    onError,
    onNotice,
    onStatusChange,
    status,
  });

  return (
    <div className="settings-grid">
      <RuntimeConfigPanel
        configDirty={settings.configDirty}
        configDraft={settings.configDraft}
        configSaving={settings.configSaving}
        onChange={settings.updateConfigDraft}
        onSave={settings.saveRuntimeConfig}
      />
      <RuntimeStatusPanel status={status} />
      {settings.hookItems.map((item) => (
        <HookSettingsPanel item={item} key={item.provider} />
      ))}
      <CustomProviderSettingsPanel onError={onError} onNotice={onNotice} />
    </div>
  );
}
