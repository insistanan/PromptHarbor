import { useCallback, useEffect, useRef, useState } from 'react';
import * as api from '../../api';
import type {
  DraftContextMenuState,
  DraftList,
  DraftListItem,
  DraftState,
  ImagePreviewState,
  MainView,
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
  const draftCacheRef = useRef<Record<string, string>>({});
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
    setEditorVersion((version) => version + 1);
  }, [resetDraftImages]);

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
          setEditorVersion((version) => version + 1);
          return;
        }

        const key = draftKey(nextItem.provider, nextItem.sessionId, nextItem.id);
        const cachedContent = draftCacheRef.current[key];
        const content = cachedContent ?? nextItem.contentMd;
        const cachedImages = getDraftImages(key);
        draftCacheRef.current[key] = content;
        setSelectedDraftId(nextItem.id);
        setDraft(draftStateFromListItem(nextItem));
        setDraftContent(content);
        replaceDraftImages(cachedImages);
        setLastSavedDraftContent(nextItem.contentMd);
        setDraftStateKey(key);
        setDraftMessage(null);
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
          setDraftMessage(nextDraft.isEmpty ? null : '草稿已保存');
          onError(null);
        })
        .catch((reason) => onError(String(reason)))
        .finally(() => setDraftSaving(false));
    }, 500);

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
        setDraftMessage('文本已复制，等待 Agent hook 匹配真实提交');
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
      draftCacheRef.current[selectedDraftKey] = markdown;
    }
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
        cacheDraftImages(key, []);
        setDraftList((current) => insertDraftListItem(current, nextDraft));
        setSelectedDraftId(nextDraft.id);
        setDraft(nextDraft);
        setDraftContent(nextDraft.contentMd);
        resetDraftImages();
        setLastSavedDraftContent(nextDraft.contentMd);
        setDraftStateKey(key);
        setDraftMessage(null);
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
          setEditorVersion((version) => version + 1);
          return;
        }

        const nextKey = draftKey(nextItem.provider, nextItem.sessionId, nextItem.id);
        const cachedContent = draftCacheRef.current[nextKey] ?? nextItem.contentMd;
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
        onNotice('草稿已删除');
        onError(null);
      })
      .catch((reason) => onError(String(reason)));
  };

  return {
    draftContextMenu,
    deleteDraftItem,
    resetDraftWorkspace,
    workspaceProps: {
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
      onOpenDraftContextMenu: openDraftContextMenu,
      onPasteImages: addDraftImages,
      onPreviewImage: previewDraftImage,
      onRemoveImage: removeDraftImage,
      onSelectDraft: selectDraftItem,
      onSelectSession,
      selectedDraftId,
      selectedSession,
      sessions: activeSessions,
    },
  };
}
