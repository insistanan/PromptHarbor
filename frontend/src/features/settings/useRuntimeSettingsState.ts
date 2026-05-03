import { useEffect, useState } from 'react';
import * as api from '../../api';
import type { HookProvider } from '../../api';
import type { AppStatus, HookAdapterStatus, RuntimeConfigDraft } from '../../appTypes';

const hookProviders: HookProvider[] = ['claude', 'codex'];

const fallbackProviderLabels: Record<HookProvider, string> = {
  claude: 'Claude Code',
  codex: 'Codex CLI',
};

type HookProviderMap<T> = Record<HookProvider, T>;

export type RuntimeHookSettingsItem = {
  provider: HookProvider;
  title: string;
  status: HookAdapterStatus | null;
  installing: boolean;
  uninstalling: boolean;
  onInstall: () => void;
  onUninstall: () => void;
};

export function useRuntimeSettingsState({
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
  const [hookStatuses, setHookStatuses] = useState<HookProviderMap<HookAdapterStatus | null>>({
    claude: null,
    codex: null,
  });
  const [installing, setInstalling] = useState<HookProviderMap<boolean>>({
    claude: false,
    codex: false,
  });
  const [uninstalling, setUninstalling] = useState<HookProviderMap<boolean>>({
    claude: false,
    codex: false,
  });
  const [configDraft, setConfigDraft] = useState<RuntimeConfigDraft | null>(null);
  const [configDirty, setConfigDirty] = useState(false);
  const [configSaving, setConfigSaving] = useState(false);

  useEffect(() => {
    let disposed = false;

    hookProviders.forEach((provider) => {
      api
        .getHookStatus<HookAdapterStatus>(provider)
        .then((nextStatus) => {
          if (!disposed) {
            setHookStatuses((current) => ({ ...current, [provider]: nextStatus }));
          }
        })
        .catch((reason) => {
          if (!disposed) {
            onError(String(reason));
          }
        });
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

  const installHook = (provider: HookProvider) => {
    setInstalling((current) => ({ ...current, [provider]: true }));
    api
      .installHook<HookAdapterStatus>(provider)
      .then((nextStatus) => {
        setHookStatuses((current) => ({ ...current, [provider]: nextStatus }));
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => {
        setInstalling((current) => ({ ...current, [provider]: false }));
      });
  };

  const uninstallHook = (provider: HookProvider) => {
    setUninstalling((current) => ({ ...current, [provider]: true }));
    api
      .uninstallHook<HookAdapterStatus>(provider)
      .then((nextStatus) => {
        setHookStatuses((current) => ({ ...current, [provider]: nextStatus }));
        onNotice(`${providerLabel(provider, nextStatus)} 钩子已取消`);
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => {
        setUninstalling((current) => ({ ...current, [provider]: false }));
      });
  };

  return {
    configDirty,
    configDraft,
    configSaving,
    hookItems: hookProviders.map((provider) => ({
      provider,
      title: providerLabel(provider, hookStatuses[provider]),
      status: hookStatuses[provider],
      installing: installing[provider],
      uninstalling: uninstalling[provider],
      onInstall: () => installHook(provider),
      onUninstall: () => uninstallHook(provider),
    })),
    saveRuntimeConfig,
    updateConfigDraft,
  };
}

function providerLabel(provider: HookProvider, status: HookAdapterStatus | null) {
  return status?.providerLabel ?? fallbackProviderLabels[provider];
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
