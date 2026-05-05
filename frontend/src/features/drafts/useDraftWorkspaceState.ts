import { useCallback, useEffect, useRef, useState } from 'react';
import * as api from '../../api';
import type {
  CustomProviderSummary,
  DraftContextMenuState,
  DraftList,
  DraftListItem,
  DraftState,
  ImagePreviewState,
  MainView,
  PromptOptimizationResult,
  SessionListItem,
} from '../../appTypes';
import type { DraftWorkspaceProps } from './DraftWorkspace';
import {
  draftKey,
  draftStateFromListItem,
  insertDraftListItem,
  replaceDraftListItem,
} from './draftHelpers';
import { useDraftContextMenu } from './useDraftContextMenu';
import { useDraftImages } from './useDraftImages';

const DRAFT_AUTO_SAVE_DELAY_MS = 1200;

type DraftPromptVariant = 'original' | 'optimized';

type DraftOptimizationCacheEntry = {
  activeVariant: DraftPromptVariant;
  optimizedContent: string | null;
  originalContent: string;
};

type UseDraftWorkspaceStateOptions = {
  activeSessions: SessionListItem[];
  activeView: MainView;
  onError: (message: string | null) => void;
  onNotice: (message: string) => void;
  onPreviewImage: (image: ImagePreviewState) => void;
  onSelectSession: (session: SessionListItem | null) => void;
  selectedSession: SessionListItem | null;
};

type UseDraftWorkspaceStateResult = {
  draftContextMenu: DraftContextMenuState | null;
  deleteDraftItem: (item: DraftListItem | null) => void;
  resetDraftWorkspace: () => void;
  workspaceProps: DraftWorkspaceProps;
};

export function useDraftWorkspaceState({
  activeSessions,
  activeView,
  onError,
  onNotice,
  onPreviewImage,
  onSelectSession,
  selectedSession,
}: UseDraftWorkspaceStateOptions): UseDraftWorkspaceStateResult {
  const [draftList, setDraftList] = useState<DraftList | null>(null);
  const [selectedDraftId, setSelectedDraftId] = useState<number | null>(null);
  const [draft, setDraft] = useState<DraftState | null>(null);
  const [draftContent, setDraftContent] = useState('');
  const [draftStateKey, setDraftStateKey] = useState<string | null>(null);
  const [lastSavedDraftContent, setLastSavedDraftContent] = useState('');
  const [draftLoading, setDraftLoading] = useState(false);
  const [draftSaving, setDraftSaving] = useState(false);
  const [draftMessage, setDraftMessage] = useState<string | null>(null);
  const [editorVersion, setEditorVersion] = useState(0);
  const [optimizationProviders, setOptimizationProviders] = useState<CustomProviderSummary[]>([]);
  const [optimizationProvidersLoading, setOptimizationProvidersLoading] = useState(false);
  const [selectedOptimizationProviderId, setSelectedOptimizationProviderId] = useState<string | null>(null);
  const [optimizingPrompt, setOptimizingPrompt] = useState(false);
  const [activePromptVariant, setActivePromptVariant] = useState<DraftPromptVariant>('original');
  const draftCacheRef = useRef<Record<string, string>>({});
  const draftOptimizationCacheRef = useRef<Record<string, DraftOptimizationCacheEntry>>({});
  const currentSelectedDraftKeyRef = useRef<string | null>(null);
  const pendingCopiedDraftIdRef = useRef<number | null>(null);

  const selectedSessionKey = selectedSession
    ? `${selectedSession.provider}:${selectedSession.sessionId}`
    : null;
  const selectedDraftKey =
    selectedSession && selectedDraftId !== null
      ? draftKey(selectedSession.provider, selectedSession.sessionId, selectedDraftId)
      : null;
  const selectedSessionIsActive = selectedSession?.status === 'active';
  const draftHasUnsavedChanges = draftContent !== lastSavedDraftContent;
  currentSelectedDraftKeyRef.current = selectedDraftKey;
  const {
    closeDraftContextMenu,
    draftContextMenu,
    openDraftContextMenu,
  } = useDraftContextMenu();
  const {
    addDraftImages,
    cacheDraftImages,
    copyDraftImage,
    deleteDraftImages,
    draftImages,
    getDraftImages,
    previewDraftImage,
    removeDraftImage,
    replaceDraftImages,
    resetDraftImages,
  } = useDraftImages({
    onError,
    onMessage: setDraftMessage,
    onNotice,
    onPreviewImage,
    selectedDraftKey,
  });

  const resetDraftWorkspace = useCallback(() => {
    setDraftList(null);
    setSelectedDraftId(null);
    setDraft(null);
    setDraftContent('');
    resetDraftImages();
    setDraftStateKey(null);
    setLastSavedDraftContent('');
    setDraftLoading(false);
    setDraftSaving(false);
    setDraftMessage(null);
    setOptimizingPrompt(false);
    setActivePromptVariant('original');
    setEditorVersion((version) => version + 1);
  }, [resetDraftImages]);

  useEffect(() => {
    if (activeView !== 'drafts') {
      return;
    }

    let disposed = false;
    setOptimizationProvidersLoading(true);
    api
      .listCustomProviders<CustomProviderSummary[]>()
      .then((providers) => {
        if (disposed) {
          return;
        }

        const availableProviders = providers.filter(
          (provider) => provider.enabled && provider.supported && provider.secretConfigured,
        );
        setOptimizationProviders(availableProviders);
        setSelectedOptimizationProviderId((current) =>
          availableProviders.some((provider) => provider.id === current)
            ? current
            : (availableProviders[0]?.id ?? null),
        );
      })
      .catch((reason) => {
        if (!disposed) {
          onError(String(reason));
        }
      })
      .finally(() => {
        if (!disposed) {
          setOptimizationProvidersLoading(false);
        }
      });

    return () => {
      disposed = true;
    };
  }, [activeView, onError]);

  useEffect(() => {
    if (activeView !== 'drafts') {
      return;
    }

    if (selectedSession?.status === 'active') {
      return;
    }

    onSelectSession(activeSessions[0] ?? null);
  }, [
    activeSessions,
    activeView,
    onSelectSession,
    selectedSession?.provider,
    selectedSession?.sessionId,
    selectedSession?.status,
  ]);

  useEffect(() => {
    let disposed = false;

    if (!selectedSession || !selectedSessionIsActive || !selectedSessionKey) {
      resetDraftWorkspace();
      return () => {
        disposed = true;
      };
    }

    setDraftLoading(true);
    api.listDrafts<DraftList>({
      provider: selectedSession.provider,
      sessionId: selectedSession.sessionId,
    })
      .then((nextList) => {
        if (disposed) {
          return;
        }
        const currentItem =
          selectedDraftId === null
            ? null
            : nextList.items.find((item) => item.id === selectedDraftId) ?? null;
        const copiedDraftWasSent =
          currentItem?.status === 'sent' && pendingCopiedDraftIdRef.current === currentItem.id;
        const nextItem =
          !currentItem || copiedDraftWasSent
            ? nextList.items.find((item) => item.status !== 'sent') ?? nextList.items[0] ?? null
            : currentItem;

        if (copiedDraftWasSent) {
          pendingCopiedDraftIdRef.current = null;
        }
        setDraftList(nextList);

        if (!nextItem) {
          setSelectedDraftId(null);
          setDraft(null);
          setDraftContent('');
          resetDraftImages();
          setDraftStateKey(null);
          setLastSavedDraftContent('');
          setDraftMessage(null);
          setActivePromptVariant('original');
          setEditorVersion((version) => version + 1);
          return;
        }

        const key = draftKey(nextItem.provider, nextItem.sessionId, nextItem.id);
        const content = draftCacheRef.current[key] ?? nextItem.contentMd;
        const optimizationEntry = getOrCreateDraftOptimizationEntry(
          draftOptimizationCacheRef.current,
          key,
          content,
        );
        const visibleContent = activeContentFromOptimizationEntry(optimizationEntry);
        const cachedImages = getDraftImages(key);
        draftCacheRef.current[key] = visibleContent;
        setSelectedDraftId(nextItem.id);
        setDraft(draftStateFromListItem(nextItem));
        setDraftContent(visibleContent);
        replaceDraftImages(cachedImages);
        setLastSavedDraftContent(nextItem.contentMd);
        setDraftStateKey(key);
        setDraftMessage(null);
        setActivePromptVariant(optimizationEntry.activeVariant);
        setEditorVersion((version) => version + 1);
      })
      .catch((reason) => {
        if (!disposed) {
          onError(String(reason));
        }
      })
      .finally(() => {
        if (!disposed) {
          setDraftLoading(false);
        }
      });

    return () => {
      disposed = true;
    };
  }, [
    onError,
    getDraftImages,
    replaceDraftImages,
    resetDraftImages,
    resetDraftWorkspace,
    selectedSession?.provider,
    selectedSession?.sessionId,
    selectedSession?.status,
    selectedSession?.promptCount,
    selectedDraftId,
    selectedSessionIsActive,
    selectedSessionKey,
  ]);

  useEffect(() => {
    if (
      !selectedSession ||
      !selectedSessionIsActive ||
      !selectedDraftKey ||
      !draft ||
      draft.status === 'sent' ||
      draftStateKey !== selectedDraftKey ||
      draftContent === lastSavedDraftContent
    ) {
      return;
    }

    const timer = window.setTimeout(() => {
      setDraftSaving(true);
      api.saveDraftById<DraftState>({
        provider: selectedSession.provider,
        sessionId: selectedSession.sessionId,
        draftId: draft.id,
        contentMd: draftContent,
      })
        .then((nextDraft) => {
          setDraft(nextDraft);
          setLastSavedDraftContent(nextDraft.contentMd);
          draftCacheRef.current[selectedDraftKey] = nextDraft.contentMd;
          setDraftList((current) => replaceDraftListItem(current, nextDraft));
          setDraftMessage(null);
          onError(null);
        })
        .catch((reason) => onError(String(reason)))
        .finally(() => setDraftSaving(false));
    }, DRAFT_AUTO_SAVE_DELAY_MS);

    return () => window.clearTimeout(timer);
  }, [
    draft,
    draftContent,
    draftStateKey,
    lastSavedDraftContent,
    onError,
    selectedSession,
    selectedDraftKey,
    selectedSessionIsActive,
  ]);

  const copyCurrentDraft = () => {
    if (!selectedSession || !selectedSessionIsActive || !draft || !draftContent.trim()) {
      return;
    }

    if (draft.status === 'sent') {
      navigator.clipboard
        .writeText(draftContent)
        .then(() => {
          onNotice('历史草稿已复制');
          onError(null);
        })
        .catch((reason) => {
          onError(String(reason));
        });
      return;
    }

    navigator.clipboard
      .writeText(draftContent)
      .then(() =>
        api.markDraftCopiedById<DraftState>({
          provider: selectedSession.provider,
          sessionId: selectedSession.sessionId,
          draftId: draft.id,
          contentMd: draftContent,
        }),
      )
      .then((nextDraft) => {
        pendingCopiedDraftIdRef.current = nextDraft.id;
        setDraft(nextDraft);
        setLastSavedDraftContent(nextDraft.contentMd);
        if (selectedDraftKey) {
          draftCacheRef.current[selectedDraftKey] = nextDraft.contentMd;
        }
        setDraftList((current) => replaceDraftListItem(current, nextDraft));
        setDraftMessage('文本已复制');
        onNotice('文本已复制');
        onError(null);
      })
      .catch((reason) => {
        onError(String(reason));
      });
  };

  const updateDraftContent = (markdown: string) => {
    setDraftContent(markdown);
    if (selectedDraftKey) {
      const optimizationEntry = getOrCreateDraftOptimizationEntry(
        draftOptimizationCacheRef.current,
        selectedDraftKey,
        markdown,
      );
      if (optimizationEntry.activeVariant === 'optimized' && optimizationEntry.optimizedContent !== null) {
        optimizationEntry.optimizedContent = markdown;
      } else {
        optimizationEntry.originalContent = markdown;
      }
      draftCacheRef.current[selectedDraftKey] = markdown;
    }
  };

  const togglePromptVariant = () => {
    if (!selectedDraftKey) {
      return;
    }

    const entry = draftOptimizationCacheRef.current[selectedDraftKey];
    if (!entry?.optimizedContent) {
      return;
    }

    entry.activeVariant = entry.activeVariant === 'original' ? 'optimized' : 'original';
    const nextContent = activeContentFromOptimizationEntry(entry);
    draftCacheRef.current[selectedDraftKey] = nextContent;
    setActivePromptVariant(entry.activeVariant);
    setDraftContent(nextContent);
    setDraftMessage(entry.activeVariant === 'original' ? '已切换到原文' : '已切换到优化稿');
    setEditorVersion((version) => version + 1);
  };

  const optimizeCurrentDraft = () => {
    if (
      !selectedSession ||
      !selectedSessionIsActive ||
      !draft ||
      draft.status === 'sent' ||
      !selectedDraftKey
    ) {
      return;
    }

    const providerId = selectedOptimizationProviderId;
    if (!providerId) {
      onError('未配置可用供应商，请先在设置中启用带密钥的 OpenAI Chat 供应商');
      return;
    }

    const sourcePrompt = draftContent.trim();
    if (!sourcePrompt) {
      onError('请先输入要优化的草稿内容');
      return;
    }

    setOptimizingPrompt(true);
    const requestedDraftKey = selectedDraftKey;
    const requestedDraftContent = draftContent;
    api
      .optimizePromptWithCustomProvider<PromptOptimizationResult>({
        providerId,
        promptMd: requestedDraftContent,
      })
      .then((result) => {
        const entry = getOrCreateDraftOptimizationEntry(
          draftOptimizationCacheRef.current,
          requestedDraftKey,
          requestedDraftContent,
        );
        entry.originalContent = requestedDraftContent;
        entry.optimizedContent = result.optimizedPromptMd;
        entry.activeVariant = 'optimized';
        draftCacheRef.current[requestedDraftKey] = result.optimizedPromptMd;
        if (currentSelectedDraftKeyRef.current !== requestedDraftKey) {
          onError(null);
          return;
        }
        setActivePromptVariant('optimized');
        setDraftContent(result.optimizedPromptMd);
        setDraftMessage('已生成优化稿');
        setEditorVersion((version) => version + 1);
        onNotice(`已使用 ${result.providerName} 优化提示词`);
        onError(null);
      })
      .catch((reason) => {
        onError(String(reason));
      })
      .finally(() => setOptimizingPrompt(false));
  };

  const selectDraftItem = (item: DraftListItem) => {
    setSelectedDraftId(item.id);
    closeDraftContextMenu();
  };

  const createDraftForSelectedSession = () => {
    if (!selectedSession || !selectedSessionIsActive) {
      return;
    }

    api.createDraft<DraftState>({
      provider: selectedSession.provider,
      sessionId: selectedSession.sessionId,
    })
      .then((nextDraft) => {
        const key = draftKey(nextDraft.provider, nextDraft.sessionId, nextDraft.id);
        draftCacheRef.current[key] = nextDraft.contentMd;
        draftOptimizationCacheRef.current[key] = {
          activeVariant: 'original',
          optimizedContent: null,
          originalContent: nextDraft.contentMd,
        };
        cacheDraftImages(key, []);
        setDraftList((current) => insertDraftListItem(current, nextDraft));
        setSelectedDraftId(nextDraft.id);
        setDraft(nextDraft);
        setDraftContent(nextDraft.contentMd);
        resetDraftImages();
        setLastSavedDraftContent(nextDraft.contentMd);
        setDraftStateKey(key);
        setDraftMessage(null);
        setActivePromptVariant('original');
        setEditorVersion((version) => version + 1);
        onError(null);
      })
      .catch((reason) => onError(String(reason)));
  };

  const deleteDraftItem = (item: DraftListItem | null) => {
    if (!selectedSession || !selectedSessionIsActive || !item) {
      return;
    }

    const confirmed = window.confirm(`删除草稿 #${item.id}？\n\n删除后无法恢复。`);
    if (!confirmed) {
      return;
    }

    closeDraftContextMenu();
    api.deleteDraft<DraftList>({
      provider: selectedSession.provider,
      sessionId: selectedSession.sessionId,
      draftId: item.id,
    })
      .then((nextList) => {
        setDraftList(nextList);
        const deletedKey = draftKey(item.provider, item.sessionId, item.id);
        delete draftCacheRef.current[deletedKey];
        delete draftOptimizationCacheRef.current[deletedKey];
        deleteDraftImages(deletedKey);

        const nextItem =
          nextList.items.find((draftItem) => draftItem.status !== 'sent') ??
          nextList.items[0] ??
          null;
        if (!nextItem) {
          setSelectedDraftId(null);
          setDraft(null);
          setDraftContent('');
          resetDraftImages();
          setLastSavedDraftContent('');
          setDraftStateKey(null);
          setActivePromptVariant('original');
          setEditorVersion((version) => version + 1);
          return;
        }

        const nextKey = draftKey(nextItem.provider, nextItem.sessionId, nextItem.id);
        const initialContent = draftCacheRef.current[nextKey] ?? nextItem.contentMd;
        const optimizationEntry = getOrCreateDraftOptimizationEntry(
          draftOptimizationCacheRef.current,
          nextKey,
          initialContent,
        );
        const cachedContent = activeContentFromOptimizationEntry(optimizationEntry);
        const cachedImages = getDraftImages(nextKey);
        draftCacheRef.current[nextKey] = cachedContent;
        setSelectedDraftId(nextItem.id);
        setDraft(draftStateFromListItem(nextItem));
        setDraftContent(cachedContent);
        replaceDraftImages(cachedImages);
        setLastSavedDraftContent(nextItem.contentMd);
        setDraftStateKey(nextKey);
        setEditorVersion((version) => version + 1);
        setDraftMessage(null);
        setActivePromptVariant(optimizationEntry.activeVariant);
        onNotice('草稿已删除');
        onError(null);
      })
      .catch((reason) => onError(String(reason)));
  };

  const hasOptimizedPromptVariant =
    selectedDraftKey !== null &&
    Boolean(draftOptimizationCacheRef.current[selectedDraftKey]?.optimizedContent);
  const optimizationDisabledReason = getOptimizationDisabledReason({
    draft,
    draftContent,
    draftLoading,
    draftSaving,
    optimizationProviders,
    optimizationProvidersLoading,
    selectedOptimizationProviderId,
    selectedSessionIsActive,
  });

  return {
    draftContextMenu,
    deleteDraftItem,
    resetDraftWorkspace,
    workspaceProps: {
      activePromptVariant,
      canTogglePromptVariant: hasOptimizedPromptVariant,
      currentDraftContent: draftContent,
      draft,
      draftCache: draftCacheRef.current,
      draftHasUnsavedChanges,
      draftImages,
      draftList,
      draftLoading,
      draftMessage,
      draftSaving,
      draftStateKey,
      editorVersion,
      onCopyDraft: copyCurrentDraft,
      onCopyImage: copyDraftImage,
      onCreateDraft: createDraftForSelectedSession,
      onDeleteDraft: deleteDraftItem,
      onDraftChange: updateDraftContent,
      onOptimizePrompt: optimizeCurrentDraft,
      onOpenDraftContextMenu: openDraftContextMenu,
      onOpenSessionHistory: onSelectSession,
      onPasteImages: addDraftImages,
      onPreviewImage: previewDraftImage,
      onRemoveImage: removeDraftImage,
      onSelectOptimizationProvider: setSelectedOptimizationProviderId,
      onSelectDraft: selectDraftItem,
      onSelectSession,
      onTogglePromptVariant: togglePromptVariant,
      optimizationDisabledReason,
      optimizationProviderOptions: optimizationProviders,
      optimizingPrompt,
      selectedDraftId,
      selectedOptimizationProviderId,
      selectedSession,
      sessions: activeSessions,
    },
  };
}

function getOrCreateDraftOptimizationEntry(
  cache: Record<string, DraftOptimizationCacheEntry>,
  key: string,
  initialContent: string,
) {
  const current = cache[key];
  if (current) {
    if (current.activeVariant === 'optimized' && !current.optimizedContent) {
      current.activeVariant = 'original';
    }
    return current;
  }

  const nextEntry: DraftOptimizationCacheEntry = {
    activeVariant: 'original',
    optimizedContent: null,
    originalContent: initialContent,
  };
  cache[key] = nextEntry;
  return nextEntry;
}

function activeContentFromOptimizationEntry(entry: DraftOptimizationCacheEntry) {
  if (entry.activeVariant === 'optimized' && entry.optimizedContent !== null) {
    return entry.optimizedContent;
  }
  return entry.originalContent;
}

function getOptimizationDisabledReason({
  draft,
  draftContent,
  draftLoading,
  draftSaving,
  optimizationProviders,
  optimizationProvidersLoading,
  selectedOptimizationProviderId,
  selectedSessionIsActive,
}: {
  draft: DraftState | null;
  draftContent: string;
  draftLoading: boolean;
  draftSaving: boolean;
  optimizationProviders: CustomProviderSummary[];
  optimizationProvidersLoading: boolean;
  selectedOptimizationProviderId: string | null;
  selectedSessionIsActive: boolean | undefined;
}) {
  if (optimizationProvidersLoading) {
    return '正在读取可用供应商';
  }
  if (!selectedSessionIsActive || !draft || draft.status === 'sent') {
    return '当前草稿不可优化';
  }
  if (draftLoading) {
    return '正在读取草稿';
  }
  if (draftSaving) {
    return '草稿保存中';
  }
  if (!draftContent.trim()) {
    return '请先输入要优化的草稿内容';
  }
  if (!optimizationProviders.length) {
    return '未配置可用供应商，请先在设置中启用带密钥的 OpenAI Chat 供应商';
  }
  if (!selectedOptimizationProviderId) {
    return '请选择一个供应商';
  }
  return null;
}
