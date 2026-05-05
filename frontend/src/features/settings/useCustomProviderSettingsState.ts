import { useEffect, useState } from 'react';
import * as api from '../../api';
import type {
  CustomProviderDraft,
  CustomProviderSaveResult,
  CustomProviderSummary,
  CustomProviderTestResult,
} from '../../appTypes';

export function useCustomProviderSettingsState({
  onError,
  onNotice,
}: {
  onNotice: (message: string) => void;
  onError: (message: string | null) => void;
}) {
  const [providers, setProviders] = useState<CustomProviderSummary[]>([]);
  const [providersLoading, setProvidersLoading] = useState(true);
  const [draft, setDraft] = useState<CustomProviderDraft>(emptyCustomProviderDraft());
  const [draftDirty, setDraftDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<CustomProviderTestResult | null>(null);

  useEffect(() => {
    let disposed = false;

    api
      .listCustomProviders<CustomProviderSummary[]>()
      .then((nextProviders) => {
        if (disposed) {
          return;
        }

        setProviders(nextProviders);
        setDraft(
          nextProviders[0]
            ? customProviderDraftFromSummary(nextProviders[0])
            : emptyCustomProviderDraft(),
        );
        setDraftDirty(false);
        onError(null);
      })
      .catch((reason) => {
        if (!disposed) {
          onError(String(reason));
        }
      })
      .finally(() => {
        if (!disposed) {
          setProvidersLoading(false);
        }
      });

    return () => {
      disposed = true;
    };
  }, [onError]);

  const selectProvider = (providerId: string) => {
    const nextProvider = providers.find((provider) => provider.id === providerId);
    if (!nextProvider) {
      return;
    }

    setDraft(customProviderDraftFromSummary(nextProvider));
    setDraftDirty(false);
    setTestResult(null);
  };

  const createProvider = () => {
    setDraft(emptyCustomProviderDraft());
    setDraftDirty(false);
    setTestResult(null);
  };

  const updateDraft = (patch: Partial<CustomProviderDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));
    setDraftDirty(true);
    setTestResult(null);
  };

  const saveProvider = () => {
    if (!draft.name.trim()) {
      onError('供应商名称不能为空');
      return;
    }

    setSaving(true);
    api
      .saveCustomProvider<CustomProviderSaveResult>(payloadFromDraft(draft))
      .then((result) => {
        const saved =
          result.providers.find((provider) => provider.id === result.savedProviderId) ?? null;
        setProviders(result.providers);
        setDraft(saved ? customProviderDraftFromSummary(saved) : emptyCustomProviderDraft());
        setDraftDirty(false);
        onNotice('自定义供应商已保存');
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setSaving(false));
  };

  const deleteProvider = () => {
    if (!draft.providerId) {
      return;
    }

    const confirmed = window.confirm(`删除供应商 ${draft.name || draft.providerId}？`);
    if (!confirmed) {
      return;
    }

    setDeleting(true);
    api
      .deleteCustomProvider<CustomProviderSummary[]>({
        providerId: draft.providerId,
      })
      .then((nextProviders) => {
        setProviders(nextProviders);
        setDraft(
          nextProviders[0]
            ? customProviderDraftFromSummary(nextProviders[0])
            : emptyCustomProviderDraft(),
        );
        setDraftDirty(false);
        setTestResult(null);
        onNotice('自定义供应商已删除');
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setDeleting(false));
  };

  const testProvider = () => {
    setTesting(true);
    api
      .testCustomProvider<CustomProviderTestResult>(payloadFromDraft(draft))
      .then((result) => {
        setTestResult(result);
        onNotice(result.message);
        onError(null);
      })
      .catch((reason) => {
        setTestResult(null);
        onError(String(reason));
      })
      .finally(() => setTesting(false));
  };

  return {
    createProvider,
    deleteProvider,
    deleting,
    draft,
    draftDirty,
    providers,
    providersLoading,
    saveProvider,
    saving,
    selectProvider,
    testProvider,
    testResult,
    testing,
    updateDraft,
  };
}

function payloadFromDraft(draft: CustomProviderDraft) {
  return {
    providerId: draft.providerId,
    name: draft.name,
    protocol: draft.protocol,
    baseUrl: draft.baseUrl,
    apiKey: draft.apiKey,
    defaultModel: draft.defaultModel,
    enabled: draft.enabled,
  };
}

function customProviderDraftFromSummary(
  summary: CustomProviderSummary,
): CustomProviderDraft {
  return {
    providerId: summary.id,
    name: summary.name,
    protocol: summary.protocol,
    baseUrl: summary.baseUrl,
    apiKey: '',
    defaultModel: summary.defaultModel,
    enabled: summary.enabled,
    secretConfigured: summary.secretConfigured,
  };
}

function emptyCustomProviderDraft(): CustomProviderDraft {
  return {
    providerId: null,
    name: '',
    protocol: 'openai_chat',
    baseUrl: '',
    apiKey: '',
    defaultModel: '',
    enabled: true,
    secretConfigured: false,
  };
}
