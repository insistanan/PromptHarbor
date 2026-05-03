import { useEffect, useRef, useState } from 'react';
import type { ClipboardEvent as ReactClipboardEvent } from 'react';
import { Editor, defaultValueCtx, rootCtx } from '@milkdown/kit/core';
import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
import { history } from '@milkdown/kit/plugin/history';
import { commonmark } from '@milkdown/kit/preset/commonmark';
import { Milkdown, MilkdownProvider, useEditor } from '@milkdown/react';
import * as api from './api';
import {
  PromptHistoryList,
  type PromptAttachment,
  type PromptAttachmentDataUrl,
  type PromptHistoryItem,
} from './features/history/PromptHistoryList';
import { PromptSearch } from './features/search/PromptSearch';
import type { PromptSearchResultItem } from './features/search/PromptSearch';
import { RuntimeSettings } from './features/settings/RuntimeSettings';

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

type SessionListItem = {
  provider: string;
  providerLabel: string;
  sessionId: string;
  shortSessionId: string;
  status: string;
  cwd: string | null;
  projectName: string;
  title: string;
  lastHookAt: string | null;
  updatedAt: string;
  promptCount: number;
  hasNonEmptyDraft: boolean;
};

type SessionList = {
  active: SessionListItem[];
  maybeClosed: SessionListItem[];
  archived: SessionListItem[];
};

type ArchiveSessionOutcome = {
  archived: boolean;
  requiresConfirmation: boolean;
  message: string;
};

type DeleteSessionOutcome = {
  deleted: boolean;
  provider: string;
  sessionId: string;
  promptEventsDeleted: number;
  draftsDeleted: number;
  attachmentsDeleted: number;
  filesDeleted: number;
  message: string;
};

type DraftState = {
  id: number;
  provider: string;
  sessionId: string;
  contentMd: string;
  contentHash: string;
  status: string;
  copyState: string;
  copiedAt: string | null;
  lastCopiedHash: string | null;
  sentAt: string | null;
  matchedPromptEventId: number | null;
  updatedAt: string;
  isEmpty: boolean;
};

type DraftListItem = {
  id: number;
  provider: string;
  sessionId: string;
  contentMd: string;
  contentHash: string;
  status: string;
  copyState: string;
  copiedAt: string | null;
  lastCopiedHash: string | null;
  sentAt: string | null;
  matchedPromptEventId: number | null;
  updatedAt: string;
  isEmpty: boolean;
  preview: string;
};

type DraftList = {
  provider: string;
  sessionId: string;
  items: DraftListItem[];
};

type PromptHistory = {
  provider: string;
  sessionId: string;
  items: PromptHistoryItem[];
};

type DraftImageAttachment = {
  id: string;
  name: string;
  mimeType: string;
  size: number;
  objectUrl: string;
  blob: Blob;
};

type ImagePreviewState = {
  src: string;
  alt: string;
  caption: string;
};

type DraftContextMenuState = {
  x: number;
  y: number;
  item: DraftListItem;
};

type MainView = 'sessions' | 'drafts' | 'search' | 'settings';

const menuItems: Array<{ id: MainView; label: string }> = [
  { id: 'sessions', label: '会话' },
  { id: 'drafts', label: '草稿' },
  { id: 'search', label: '搜索' },
  { id: 'settings', label: '设置' },
];

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [sessions, setSessions] = useState<SessionList>({
    active: [],
    maybeClosed: [],
    archived: [],
  });
  const [activeView, setActiveView] = useState<MainView>('sessions');
  const [selectedSession, setSelectedSession] = useState<SessionListItem | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [deletingSession, setDeletingSession] = useState(false);
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
  const copyNoticeTimerRef = useRef<number | null>(null);
  const draftCacheRef = useRef<Record<string, string>>({});
  const imageCacheRef = useRef<Record<string, DraftImageAttachment[]>>({});
  const pendingCopiedDraftIdRef = useRef<number | null>(null);
  const [draftImages, setDraftImages] = useState<DraftImageAttachment[]>([]);
  const [imagePreview, setImagePreview] = useState<ImagePreviewState | null>(null);
  const [draftContextMenu, setDraftContextMenu] = useState<DraftContextMenuState | null>(null);
  const [hideLowInfo, setHideLowInfo] = useState(false);
  const [sessionHistoryQuery, setSessionHistoryQuery] = useState('');
  const [promptHistory, setPromptHistory] = useState<PromptHistory | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [searchResultCount, setSearchResultCount] = useState(0);

  useEffect(() => {
    let disposed = false;
    const loadStatus = () => {
      api.getAppStatus<AppStatus>()
        .then((nextStatus) => {
          if (!disposed) {
            setStatus(nextStatus);
            setError(null);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };
    const loadSessions = () => {
      api.listSessions<SessionList>()
        .then((nextSessions) => {
          if (!disposed) {
            setSessions(nextSessions);
            setSelectedSession((current) => {
              if (!current) {
                return nextSessions.active[0] ?? nextSessions.maybeClosed[0] ?? null;
              }

              return findSession(nextSessions, current.provider, current.sessionId);
            });
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };

    loadStatus();
    loadSessions();
    const timer = window.setInterval(() => {
      loadStatus();
      loadSessions();
    }, 1000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, []);

  const selectedSessionKey = selectedSession
    ? sessionKey(selectedSession.provider, selectedSession.sessionId)
    : null;
  const selectedDraftKey =
    selectedSession && selectedDraftId !== null
      ? draftKey(selectedSession.provider, selectedSession.sessionId, selectedDraftId)
      : null;
  const allSessions = [
    ...sessions.active,
    ...sessions.maybeClosed,
    ...sessions.archived,
  ];
  const selectedSessionIsActive = selectedSession?.status === 'active';
  const draftHasUnsavedChanges = draftContent !== lastSavedDraftContent;
  const includeLowInfo = !hideLowInfo;
  const filteredHistoryItems = filterHistoryItems(
    promptHistory?.items ?? [],
    sessionHistoryQuery,
  );

  useEffect(() => {
    if (activeView !== 'drafts') {
      return;
    }

    if (selectedSession?.status === 'active') {
      return;
    }

    setSelectedSession(sessions.active[0] ?? null);
  }, [
    activeView,
    selectedSession?.provider,
    selectedSession?.sessionId,
    selectedSession?.status,
    sessions.active,
  ]);

  useEffect(() => {
    let disposed = false;

    if (!selectedSession || !selectedSessionIsActive || !selectedSessionKey) {
      setDraftList(null);
      setSelectedDraftId(null);
      setDraft(null);
      setDraftContent('');
      setDraftImages([]);
      setDraftStateKey(null);
      setLastSavedDraftContent('');
      setDraftLoading(false);
      setDraftSaving(false);
      setDraftMessage(null);
      setEditorVersion((version) => version + 1);
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
          setDraftImages([]);
          setDraftStateKey(null);
          setLastSavedDraftContent('');
          setDraftMessage(null);
          setEditorVersion((version) => version + 1);
          return;
        }

        const key = draftKey(nextItem.provider, nextItem.sessionId, nextItem.id);
        const cachedContent = draftCacheRef.current[key];
        const content = cachedContent ?? nextItem.contentMd;
        const cachedImages = imageCacheRef.current[key] ?? [];
        draftCacheRef.current[key] = content;
        setSelectedDraftId(nextItem.id);
        setDraft(draftStateFromListItem(nextItem));
        setDraftContent(content);
        setDraftImages(cachedImages);
        setLastSavedDraftContent(nextItem.contentMd);
        setDraftStateKey(key);
        setDraftMessage(null);
        setEditorVersion((version) => version + 1);
      })
      .catch((reason) => {
        if (!disposed) {
          setError(String(reason));
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
          setError(null);
        })
        .catch((reason) => setError(String(reason)))
        .finally(() => setDraftSaving(false));
    }, 500);

    return () => window.clearTimeout(timer);
  }, [
    draft,
    draftContent,
    draftStateKey,
    lastSavedDraftContent,
    selectedSession,
    selectedDraftKey,
    selectedSessionIsActive,
  ]);

  useEffect(() => {
    let disposed = false;

    if (!selectedSession) {
      setPromptHistory(null);
      setHistoryLoading(false);
      return () => {
        disposed = true;
      };
    }

    setHistoryLoading(true);
    api.listPromptHistory<PromptHistory>({
      provider: selectedSession.provider,
      sessionId: selectedSession.sessionId,
      includeLowInfo,
    })
      .then((nextHistory) => {
        if (!disposed) {
          setPromptHistory(nextHistory);
          setError(null);
        }
      })
      .catch((reason) => {
        if (!disposed) {
          setError(String(reason));
        }
      })
      .finally(() => {
        if (!disposed) {
          setHistoryLoading(false);
        }
      });

    return () => {
      disposed = true;
    };
  }, [
    includeLowInfo,
    selectedSession?.provider,
    selectedSession?.sessionId,
    selectedSession?.promptCount,
  ]);

  useEffect(() => {
    const imageCache = imageCacheRef.current;
    return () => {
      Object.values(imageCache).forEach((attachments) => {
        attachments.forEach((attachment) => URL.revokeObjectURL(attachment.objectUrl));
      });
      if (copyNoticeTimerRef.current !== null) {
        window.clearTimeout(copyNoticeTimerRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!imagePreview) {
      return;
    }

    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setImagePreview(null);
      }
    };

    window.addEventListener('keydown', closeOnEscape);
    return () => window.removeEventListener('keydown', closeOnEscape);
  }, [imagePreview]);

  useEffect(() => {
    if (!draftContextMenu) {
      return;
    }

    const closeMenu = () => setDraftContextMenu(null);
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closeMenu();
      }
    };

    window.addEventListener('click', closeMenu);
    window.addEventListener('keydown', closeOnEscape);
    return () => {
      window.removeEventListener('click', closeMenu);
      window.removeEventListener('keydown', closeOnEscape);
    };
  }, [draftContextMenu]);

  const showCopyNotice = (message: string) => {
    setCopyNotice(message);
    if (copyNoticeTimerRef.current !== null) {
      window.clearTimeout(copyNoticeTimerRef.current);
    }
    copyNoticeTimerRef.current = window.setTimeout(() => {
      setCopyNotice(null);
      copyNoticeTimerRef.current = null;
    }, 1800);
  };

  const copyCurrentDraft = () => {
    if (!selectedSession || !selectedSessionIsActive || !draft || !draftContent.trim()) {
      return;
    }

    if (draft.status === 'sent') {
      navigator.clipboard
        .writeText(draftContent)
        .then(() => {
          showCopyNotice('历史草稿已复制');
          setError(null);
        })
        .catch((reason) => {
          setError(String(reason));
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
        showCopyNotice('文本已复制');
        setError(null);
      })
      .catch((reason) => {
        setError(String(reason));
      });
  };
  const updateDraftContent = (markdown: string) => {
    setDraftContent(markdown);
    if (selectedDraftKey) {
      draftCacheRef.current[selectedDraftKey] = markdown;
    }
  };
  const updateDraftImages = (images: DraftImageAttachment[]) => {
    setDraftImages(images);
    if (selectedDraftKey) {
      imageCacheRef.current[selectedDraftKey] = images;
    }
  };
  const selectDraftItem = (item: DraftListItem) => {
    setSelectedDraftId(item.id);
    setDraftContextMenu(null);
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
        imageCacheRef.current[key] = [];
        setDraftList((current) => insertDraftListItem(current, nextDraft));
        setSelectedDraftId(nextDraft.id);
        setDraft(nextDraft);
        setDraftContent(nextDraft.contentMd);
        setDraftImages([]);
        setLastSavedDraftContent(nextDraft.contentMd);
        setDraftStateKey(key);
        setDraftMessage(null);
        setEditorVersion((version) => version + 1);
        setError(null);
      })
      .catch((reason) => setError(String(reason)));
  };
  const deleteDraftItem = (item: DraftListItem | null) => {
    if (!selectedSession || !selectedSessionIsActive || !item) {
      return;
    }

    const confirmed = window.confirm(`删除草稿 #${item.id}？\n\n删除后无法恢复。`);
    if (!confirmed) {
      return;
    }

    setDraftContextMenu(null);
    api.deleteDraft<DraftList>({
      provider: selectedSession.provider,
      sessionId: selectedSession.sessionId,
      draftId: item.id,
    })
      .then((nextList) => {
        setDraftList(nextList);
        const deletedKey = draftKey(item.provider, item.sessionId, item.id);
        delete draftCacheRef.current[deletedKey];
        const deletedImages = imageCacheRef.current[deletedKey] ?? [];
        deletedImages.forEach((image) => URL.revokeObjectURL(image.objectUrl));
        delete imageCacheRef.current[deletedKey];

        const nextItem =
          nextList.items.find((draftItem) => draftItem.status !== 'sent') ??
          nextList.items[0] ??
          null;
        if (!nextItem) {
          setSelectedDraftId(null);
          setDraft(null);
          setDraftContent('');
          setDraftImages([]);
          setLastSavedDraftContent('');
          setDraftStateKey(null);
          setEditorVersion((version) => version + 1);
          return;
        }

        const nextKey = draftKey(nextItem.provider, nextItem.sessionId, nextItem.id);
        const cachedContent = draftCacheRef.current[nextKey] ?? nextItem.contentMd;
        const cachedImages = imageCacheRef.current[nextKey] ?? [];
        draftCacheRef.current[nextKey] = cachedContent;
        setSelectedDraftId(nextItem.id);
        setDraft(draftStateFromListItem(nextItem));
        setDraftContent(cachedContent);
        setDraftImages(cachedImages);
        setLastSavedDraftContent(nextItem.contentMd);
        setDraftStateKey(nextKey);
        setEditorVersion((version) => version + 1);
        setDraftMessage(null);
        showCopyNotice('草稿已删除');
        setError(null);
      })
      .catch((reason) => setError(String(reason)));
  };
  const openDraftContextMenu = (item: DraftListItem, x: number, y: number) => {
    setDraftContextMenu({
      item,
      x: Math.min(x, window.innerWidth - 180),
      y: Math.min(y, window.innerHeight - 90),
    });
  };
  const addDraftImages = (files: File[]) => {
    if (!files.length) {
      return;
    }

    const nextImages = files.map((file) => ({
      id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
      name: file.name || 'clipboard-image',
      mimeType: file.type || 'image/png',
      size: file.size,
      objectUrl: URL.createObjectURL(file),
      blob: file,
    }));

    updateDraftImages([...draftImages, ...nextImages]);
    setDraftMessage(`${nextImages.length} 张图片已作为附件暂存`);
  };
  const removeDraftImage = (imageId: string) => {
    const image = draftImages.find((item) => item.id === imageId);
    if (image) {
      URL.revokeObjectURL(image.objectUrl);
    }
    updateDraftImages(draftImages.filter((item) => item.id !== imageId));
  };
  const copyDraftImage = (image: DraftImageAttachment) => {
    if (!navigator.clipboard || typeof ClipboardItem === 'undefined') {
      setError('当前 WebView 不支持直接复制图片到剪切板');
      return;
    }

    navigator.clipboard
      .write([
        new ClipboardItem({
          [image.mimeType]: image.blob,
        }),
      ])
      .then(() => {
        setDraftMessage('图片已复制到剪切板');
        showCopyNotice('图片已复制');
        setError(null);
      })
      .catch((reason) => setError(String(reason)));
  };
  const previewDraftImage = (image: DraftImageAttachment) => {
    setImagePreview({
      src: image.objectUrl,
      alt: image.name,
      caption: `${image.name} · ${formatFileSize(image.size)}`,
    });
  };
  const copyPromptHistoryItem = (item: PromptHistoryItem) => {
    const copyText = historyPromptCopyText(item);
    const copiedWithImagePaths = item.attachments.some(
      (attachment) => attachment.kind === 'image' && attachment.filePath,
    );
    navigator.clipboard
      .writeText(copyText)
      .then(() => {
        showCopyNotice(
          copiedWithImagePaths ? '历史 prompt 已复制，已带图片路径' : '历史 prompt 已复制',
        );
        setError(null);
      })
      .catch((reason) => setError(String(reason)));
  };
  const previewPromptHistoryAttachment = (attachment: PromptAttachment, dataUrl: string) => {
    setImagePreview({
      src: dataUrl,
      alt: attachment.placeholder ?? attachment.fileName,
      caption: `${attachment.placeholder ?? attachment.fileName} · ${formatFileSize(
        attachment.fileSize,
      )}`,
    });
  };
  const copyPromptHistoryAttachment = (attachment: PromptAttachment) => {
    if (!navigator.clipboard || typeof ClipboardItem === 'undefined') {
      setError('当前 WebView 不支持直接复制图片到剪切板');
      return;
    }

    api.readPromptAttachmentDataUrl<PromptAttachmentDataUrl>({
      attachmentId: attachment.id,
    })
      .then((image) => dataUrlToBlob(image.dataUrl).then((blob) => ({ image, blob })))
      .then(({ image, blob }) =>
        navigator.clipboard.write([
          new ClipboardItem({
            [image.mimeType]: blob,
          }),
        ]),
      )
      .then(() => {
        showCopyNotice('历史图片已复制');
        setError(null);
      })
      .catch((reason) => setError(String(reason)));
  };
  const copySelectedSessionResumeCommand = () => {
    if (!selectedSession) {
      return;
    }

    navigator.clipboard
      .writeText(sessionResumeCommand(selectedSession))
      .then(() => {
        showCopyNotice('恢复命令已复制');
        setError(null);
      })
      .catch((reason) => setError(String(reason)));
  };
  const openSelectedSessionFolder = () => {
    if (!selectedSession?.cwd) {
      return;
    }

    api.openProjectPath({ path: selectedSession.cwd }).catch((reason) =>
      setError(String(reason)),
    );
  };
  const archiveSelectedSession = () => {
    if (!selectedSession || selectedSession.status === 'archived') {
      return;
    }

    archiveSession(selectedSession, false).then((outcome) => {
      if (outcome.requiresConfirmation) {
        const confirmed = window.confirm(outcome.message);
        if (confirmed) {
          archiveSession(selectedSession, true);
        }
      }
    });
  };
  const archiveSession = (session: SessionListItem, force: boolean) =>
    api.archiveSession<ArchiveSessionOutcome>({
      provider: session.provider,
      sessionId: session.sessionId,
      force,
    })
      .then((outcome) => {
        if (outcome.archived) {
          return api.listSessions<SessionList>().then((nextSessions) => {
            setSessions(nextSessions);
            setSelectedSession(findSession(nextSessions, session.provider, session.sessionId));
            return outcome;
          });
        }

        return outcome;
      })
      .catch((reason) => {
        setError(String(reason));
        return {
          archived: false,
          requiresConfirmation: false,
          message: String(reason),
        };
      });
  const deleteSelectedSession = () => {
    if (!selectedSession || deletingSession) {
      return;
    }

    const confirmed = window.confirm(
      `删除会话 ${selectedSession.sessionId}？\n\n这只会删除 PromptHarbor 本地记录、草稿、历史 prompt 和本地图片附件，不会删除 Claude Code/Codex CLI 的原始会话文件。`,
    );
    if (!confirmed) {
      return;
    }

    const session = selectedSession;
    setDeletingSession(true);
    api.deleteSession<DeleteSessionOutcome>({
      provider: session.provider,
      sessionId: session.sessionId,
    })
      .then((outcome) =>
        api.listSessions<SessionList>().then((nextSessions) => {
          setSessions(nextSessions);
          const nextSelected =
            nextSessions.active[0] ?? nextSessions.maybeClosed[0] ?? nextSessions.archived[0] ?? null;
          setSelectedSession(nextSelected);
          setPromptHistory(null);
          setDraftList(null);
          setSelectedDraftId(null);
          setDraft(null);
          setDraftContent('');
          setDraftImages([]);
          setDraftStateKey(null);
          setLastSavedDraftContent('');
          showCopyNotice(
            `会话已删除：${outcome.promptEventsDeleted} 条 prompt，${outcome.draftsDeleted} 条草稿`,
          );
          setError(null);
          return outcome;
        }),
      )
      .catch((reason) => setError(String(reason)))
      .finally(() => setDeletingSession(false));
  };
  const setSelectedSessionFromSearch = (result: PromptSearchResultItem) => {
    const nextSession = findSession(sessions, result.provider, result.sessionId);
    if (nextSession) {
      setSelectedSession(nextSession);
      setActiveView('sessions');
    }
  };

  return (
    <main className="app-shell" aria-label="PromptHarbor 工作区">
      <aside className="left-rail" aria-label="主导航">
        <header className="brand-block">
          <p className="eyebrow">提示港</p>
          <h1>PromptHarbor</h1>
          <p className="status-dot">{status?.recordingPaused ? '记录暂停' : '本地记录中'}</p>
        </header>

        <nav className="menu-list" aria-label="主菜单">
          {menuItems.map((item) => (
            <button
              className={activeView === item.id ? 'menu-item active' : 'menu-item'}
              key={item.id}
              onClick={() => setActiveView(item.id)}
              type="button"
            >
              <span>{item.label}</span>
              <small>{menuBadge(item.id, sessions, status, searchResultCount)}</small>
            </button>
          ))}
        </nav>

        <footer className="rail-footer" aria-label="采集概览">
          <span>{status?.collectorReady ? '采集就绪' : '采集等待中'}</span>
          <strong>{status?.promptEventCount ?? 0} 条 prompt</strong>
        </footer>
      </aside>

      <section className="workspace-pane" aria-label="会话工作区">
        <header
          className={
            activeView === 'sessions' || activeView === 'drafts'
              ? 'detail-header compact'
              : 'detail-header'
          }
        >
          <div className="detail-title">
            <p className="eyebrow">{viewEyebrow(activeView)}</p>
            <h2>{viewTitle(activeView)}</h2>
            <p className="workspace-subtitle">{viewSubtitle(activeView, selectedSession)}</p>
          </div>
          <div className="header-side">
            <SessionReferenceCard
              deleting={deletingSession}
              onCopyCommand={copySelectedSessionResumeCommand}
              onDelete={deleteSelectedSession}
              onOpenPath={openSelectedSessionFolder}
              session={selectedSession}
            />
            <div className="status-strip" aria-label="应用状态">
              <span>{status?.version ? `v${status.version}` : '版本读取中'}</span>
              <span>{status?.localEndpoint ?? '采集端点待连接'}</span>
              <span>{status?.recordingPaused ? '记录暂停' : '记录开启'}</span>
              <span>
                {status ? (status.collectorReady ? '采集就绪' : '采集不可用') : '采集状态读取中'}
              </span>
              <span>{status?.hookBinaryReady ? 'hook 就绪' : 'hook 待处理'}</span>
            </div>
          </div>
        </header>

        {error ? <p className="error-banner">IPC 调用失败：{error}</p> : null}
        {copyNotice ? (
          <div className="copy-toast" role="status">
            {copyNotice}
          </div>
        ) : null}
        {imagePreview ? (
          <ImagePreviewDialog image={imagePreview} onClose={() => setImagePreview(null)} />
        ) : null}
        {draftContextMenu ? (
          <DraftContextMenu
            item={draftContextMenu.item}
            onDelete={deleteDraftItem}
            x={draftContextMenu.x}
            y={draftContextMenu.y}
          />
        ) : null}

        {activeView === 'settings' ? (
          <RuntimeSettings
            onError={setError}
            onNotice={showCopyNotice}
            onStatusChange={setStatus}
            status={status}
          />
        ) : null}

        {activeView === 'sessions' ? (
          <>
            <SessionTabs
              emptyDescription="只要 Claude Code 或 Codex CLI 发出第一条 prompt，这里就会出现对应会话。"
              emptyTitle="暂无 Agent 会话"
              items={allSessions}
              onSelect={(session) => {
                setSelectedSession(session);
                setSessionHistoryQuery('');
              }}
              selected={selectedSession}
            />

            <section className="prompt-history" aria-label="prompt 历史">
              <div className="section-heading">
                <h3>prompt 历史</h3>
                <span>
                  {historyLoading
                    ? '读取中'
                    : `${filteredHistoryItems.length}/${promptHistory?.items.length ?? 0} 条`}
                </span>
              </div>
              {selectedSession ? (
                <div className="session-detail">
                  <div className="history-toolbar">
                    <div className="selected-session-meta">
                      <strong>{selectedSession.providerLabel}</strong>
                      <span>
                        {selectedSession.shortSessionId} · {selectedSession.projectName} ·{' '}
                        {sessionStatusLabel(selectedSession.status)}
                      </span>
                    </div>
                    <input
                      aria-label="搜索当前会话 prompt"
                      className="compact-search"
                      onChange={(event) => setSessionHistoryQuery(event.currentTarget.value)}
                      placeholder="搜索当前会话 prompt"
                      type="search"
                      value={sessionHistoryQuery}
                    />
                    <label className="check-control">
                      <input
                        checked={hideLowInfo}
                        onChange={(event) => setHideLowInfo(event.currentTarget.checked)}
                        type="checkbox"
                      />
                      隐藏低信息
                    </label>
                    <button
                      className="secondary-action"
                      disabled={selectedSession.status === 'archived'}
                      onClick={archiveSelectedSession}
                      type="button"
                    >
                      归档
                    </button>
                  </div>
                  <PromptHistoryList
                    items={filteredHistoryItems}
                    onCopy={copyPromptHistoryItem}
                    onCopyAttachment={copyPromptHistoryAttachment}
                    onPreviewAttachment={previewPromptHistoryAttachment}
                  />
                </div>
              ) : (
                <div className="empty-state">
                  <p className="empty-title">等待第一条已发送 prompt</p>
                  <p>只记录用户真实提交的 prompt，模型回复不会进入 PromptHarbor。</p>
                </div>
              )}
            </section>
          </>
        ) : null}

        {activeView === 'search' ? (
          <PromptSearch
            hideLowInfo={hideLowInfo}
            onHideLowInfoChange={setHideLowInfo}
            onResultCountChange={setSearchResultCount}
            onSelect={setSelectedSessionFromSearch}
          />
        ) : null}

        {activeView === 'drafts' ? (
          <>
            <SessionTabs
              emptyDescription="草稿只能绑定当前仍在运行或近期活动的会话。"
              emptyTitle="暂无活动会话"
              items={sessions.active}
              onSelect={setSelectedSession}
              selected={selectedSession}
            />

            <section className="draft-panel" aria-label="当前草稿">
              <div className="section-heading">
                <h3>草稿工作台</h3>
                <span>
                  {draftStatusLabel(draft, draftSaving, draftLoading, draftHasUnsavedChanges)}
                </span>
              </div>
              <div className="draft-split">
                <DraftItemList
                  currentDraftContent={draftContent}
                  draftCache={draftCacheRef.current}
                  items={draftList?.items ?? []}
                  loading={draftLoading}
                  onCreate={createDraftForSelectedSession}
                  onDelete={deleteDraftItem}
                  onOpenContextMenu={openDraftContextMenu}
                  onSelect={selectDraftItem}
                  selectedDraftId={selectedDraftId}
                />
                {selectedSession && selectedSessionIsActive ? (
                  <div className="draft-detail-pane">
                    <div className="draft-detail-header">
                      <div className="selected-session-meta">
                        <strong>{selectedSession.title}</strong>
                        <span>
                          {selectedSession.providerLabel} · {selectedSession.shortSessionId} ·{' '}
                          {selectedSession.projectName}
                          {draft ? ` · 草稿 #${draft.id}` : ''}
                        </span>
                      </div>
                      <span>{draftDetailBadge(draft, draftHasUnsavedChanges)}</span>
                    </div>

                    <div className="draft-workspace">
                      {draftImages.length ? (
                        <ImageAttachmentStrip
                          images={draftImages}
                          onCopy={copyDraftImage}
                          onPreview={previewDraftImage}
                          onRemove={removeDraftImage}
                        />
                      ) : null}
                      <MilkdownProvider>
                        <MilkdownDraftEditor
                          disabled={draftLoading || draft?.status === 'sent'}
                          initialValue={draftContent}
                          key={`${draftStateKey ?? 'none'}:${editorVersion}`}
                          onPasteImages={addDraftImages}
                          onChange={updateDraftContent}
                        />
                      </MilkdownProvider>
                      <div className="draft-actions">
                        <div className="draft-meta">
                          <span>hash {draft?.contentHash.slice(0, 12) ?? '未生成'}</span>
                          <span>
                            {draft?.copiedAt
                              ? `复制于 ${formatDateTime(draft.copiedAt)}`
                              : '未复制'}
                          </span>
                        </div>
                        <button
                          className="primary-action"
                          disabled={
                            draftLoading ||
                            draftSaving ||
                            draftHasUnsavedChanges ||
                            !draft ||
                            !draftContent.trim()
                          }
                          onClick={copyCurrentDraft}
                          type="button"
                        >
                          复制文本
                        </button>
                      </div>
                      {draftMessage ? <p className="draft-message">{draftMessage}</p> : null}
                    </div>
                  </div>
                ) : (
                  <div className="empty-state draft-empty-detail">
                    <p className="empty-title">选择一个活动 Agent 会话</p>
                    <p>当前草稿只绑定活动会话；历史会话不会继续编辑。</p>
                  </div>
                )}
              </div>
            </section>
          </>
        ) : null}
      </section>
    </main>
  );
}

function DraftItemList({
  currentDraftContent,
  draftCache,
  items,
  loading,
  onCreate,
  onDelete,
  onOpenContextMenu,
  onSelect,
  selectedDraftId,
}: {
  currentDraftContent: string;
  draftCache: Record<string, string>;
  items: DraftListItem[];
  loading: boolean;
  onCreate: () => void;
  onDelete: (item: DraftListItem | null) => void;
  onOpenContextMenu: (item: DraftListItem, x: number, y: number) => void;
  onSelect: (item: DraftListItem) => void;
  selectedDraftId: number | null;
}) {
  const selectedItem = items.find((item) => item.id === selectedDraftId) ?? null;

  return (
    <aside className="draft-session-list" aria-label="草稿列表">
      <div className="draft-list-toolbar">
        <span>{loading ? '读取中' : `${items.length} 条草稿`}</span>
        <span className="draft-list-toolbar-actions">
          <button className="tiny-action" onClick={onCreate} type="button">
            新建
          </button>
          <button
            className="tiny-action danger"
            disabled={!selectedItem}
            onClick={() => onDelete(selectedItem)}
            type="button"
          >
            删除
          </button>
        </span>
      </div>
      {!items.length ? (
        <div className="draft-list-empty">
          <p>当前会话还没有草稿</p>
        </div>
      ) : null}
      {items.map((item, index) => {
        const active = selectedDraftId === item.id;
        const key = draftKey(item.provider, item.sessionId, item.id);
        const cachedContent = active ? currentDraftContent : draftCache[key] ?? item.contentMd;
        const preview = draftListPreview(cachedContent, item.preview);

        return (
          <button
            className={active ? 'draft-list-item active' : 'draft-list-item'}
            key={item.id}
            onContextMenu={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onOpenContextMenu(item, event.clientX, event.clientY);
            }}
            onClick={() => onSelect(item)}
            type="button"
          >
            <span className="draft-list-main">
              <strong>{draftListTitle(item, index)}</strong>
              <small>{draftListTimeLabel(item)}</small>
              <em>{preview}</em>
            </span>
            <span className={draftListStateClass(item)}>
              {draftListStateLabel(item)}
            </span>
          </button>
        );
      })}
    </aside>
  );
}

function DraftContextMenu({
  item,
  onDelete,
  x,
  y,
}: {
  item: DraftListItem;
  onDelete: (item: DraftListItem) => void;
  x: number;
  y: number;
}) {
  return (
    <div
      className="draft-context-menu"
      onClick={(event) => event.stopPropagation()}
      role="menu"
      style={{ left: x, top: y }}
    >
      <button
        className="draft-context-menu-item danger"
        onClick={() => onDelete(item)}
        role="menuitem"
        type="button"
      >
        <TrashIcon />
        <span>删除草稿 #{item.id}</span>
      </button>
    </div>
  );
}

function MilkdownDraftEditor({
  disabled,
  initialValue,
  onPasteImages,
  onChange,
}: {
  disabled: boolean;
  initialValue: string;
  onPasteImages: (files: File[]) => void;
  onChange: (markdown: string) => void;
}) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const handlePaste = (event: ReactClipboardEvent<HTMLDivElement>) => {
    const files = imageFilesFromClipboard(event);
    if (!files.length) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    onPasteImages(files);
  };

  const { loading } = useEditor((root) =>
    Editor.make()
      .config((ctx) => {
        ctx.set(rootCtx, root);
        ctx.set(defaultValueCtx, initialValue);
        ctx.get(listenerCtx).markdownUpdated((_, markdown) => {
          onChangeRef.current(markdown);
        });
      })
      .use(commonmark)
      .use(history)
      .use(listener),
  );

  return (
    <div
      className={disabled || loading ? 'milkdown-host disabled' : 'milkdown-host'}
      onPasteCapture={handlePaste}
    >
      <Milkdown />
    </div>
  );
}

function SessionReferenceCard({
  deleting,
  onCopyCommand,
  onDelete,
  onOpenPath,
  session,
}: {
  deleting: boolean;
  onCopyCommand: () => void;
  onDelete: () => void;
  onOpenPath: () => void;
  session: SessionListItem | null;
}) {
  if (!session) {
    return (
      <section className="session-reference empty" aria-label="会话引用信息">
        <span>暂无会话</span>
      </section>
    );
  }

  const command = sessionResumeCommand(session);

  return (
    <section className="session-reference" aria-label="会话引用信息">
      <span className="session-provider-chip">{session.providerLabel}</span>
      <div className="session-reference-row">
        <span className="session-id-text" title={command}>
          {command}
        </span>
        <button
          aria-label={`复制恢复命令：${command}`}
          className="icon-action"
          onClick={onCopyCommand}
          title={command}
          type="button"
        >
          <CopyIcon />
        </button>
      </div>
      <button
        aria-label="删除 PromptHarbor 本地会话记录"
        className="danger-icon-action"
        disabled={deleting}
        onClick={onDelete}
        title="删除 PromptHarbor 本地会话记录"
        type="button"
      >
        <TrashIcon />
        <span>{deleting ? '删除中' : '删除会话'}</span>
      </button>
      <button
        className="project-path-button"
        disabled={!session.cwd}
        onClick={onOpenPath}
        title={session.cwd ?? '暂无项目路径'}
        type="button"
      >
        <FolderIcon />
        <span>{session.cwd ?? '暂无项目路径'}</span>
      </button>
    </section>
  );
}

function ImagePreviewDialog({
  image,
  onClose,
}: {
  image: ImagePreviewState;
  onClose: () => void;
}) {
  return (
    <div
      aria-label="图片预览"
      aria-modal="true"
      className="image-preview-backdrop"
      onClick={onClose}
      role="dialog"
    >
      <figure className="image-preview-dialog" onClick={(event) => event.stopPropagation()}>
        <button
          aria-label="关闭图片预览"
          className="image-preview-close"
          onClick={onClose}
          type="button"
        >
          ×
        </button>
        <img alt={image.alt} src={image.src} />
        <figcaption>{image.caption}</figcaption>
      </figure>
    </div>
  );
}

function ZoomInIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <circle cx="11" cy="11" r="6" />
      <path d="m16 16 4 4" />
      <path d="M11 8v6" />
      <path d="M8 11h6" />
    </svg>
  );
}

function CopyIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <rect height="13" rx="2" width="13" x="8" y="8" />
      <path d="M5 16H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <path d="M3 6a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <path d="M3 6h18" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
      <path d="M10 11v6" />
      <path d="M14 11v6" />
    </svg>
  );
}

function ImageAttachmentStrip({
  images,
  onCopy,
  onPreview,
  onRemove,
}: {
  images: DraftImageAttachment[];
  onCopy: (image: DraftImageAttachment) => void;
  onPreview: (image: DraftImageAttachment) => void;
  onRemove: (imageId: string) => void;
}) {
  return (
    <section className="image-attachment-strip" aria-label="图片附件">
      {images.map((image) => (
        <article
          className="image-attachment"
          key={image.id}
          onClick={() => onPreview(image)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onPreview(image);
            }
          }}
          role="button"
          tabIndex={0}
          title="点击放大图片"
        >
          <img alt={image.name} src={image.objectUrl} />
          <span>{formatFileSize(image.size)}</span>
          <div className="image-hover-actions" aria-label="图片操作">
            <button
              aria-label="放大图片"
              className="image-hover-button"
              onClick={(event) => {
                event.stopPropagation();
                onPreview(image);
              }}
              title="放大"
              type="button"
            >
              <ZoomInIcon />
            </button>
            <button
              aria-label="复制图片到剪切板"
              className="image-hover-button"
              onClick={(event) => {
                event.stopPropagation();
                onCopy(image);
              }}
              title="复制"
              type="button"
            >
              <CopyIcon />
            </button>
          </div>
          <button
            aria-label="移除图片"
            className="image-remove-button"
            onClick={(event) => {
              event.stopPropagation();
              onRemove(image.id);
            }}
            type="button"
          >
            ×
          </button>
        </article>
      ))}
    </section>
  );
}

function SessionTabs({
  emptyDescription,
  emptyTitle,
  items,
  onSelect,
  selected,
}: {
  emptyDescription: string;
  emptyTitle: string;
  items: SessionListItem[];
  onSelect: (session: SessionListItem) => void;
  selected: SessionListItem | null;
}) {
  if (!items.length) {
    return (
      <section className="session-tabs-empty" aria-label={emptyTitle}>
        <p className="empty-title">{emptyTitle}</p>
        <p>{emptyDescription}</p>
      </section>
    );
  }

  return (
    <section
      className="session-tabs"
      aria-label="会话标签"
      onWheel={(event) => {
        const target = event.currentTarget;
        const canScrollHorizontally = target.scrollWidth > target.clientWidth;

        if (!canScrollHorizontally) {
          return;
        }

        const horizontalDelta = event.deltaX !== 0 ? event.deltaX : event.deltaY;
        const maxScrollLeft = target.scrollWidth - target.clientWidth;
        const nextScrollLeft = Math.min(
          maxScrollLeft,
          Math.max(0, target.scrollLeft + horizontalDelta),
        );

        event.preventDefault();
        event.stopPropagation();
        target.scrollLeft = nextScrollLeft;
      }}
    >
      {items.map((session) => {
        const active =
          selected?.provider === session.provider && selected?.sessionId === session.sessionId;
        return (
          <button
            className={active ? 'session-tab active' : 'session-tab'}
            key={`${session.provider}:${session.sessionId}`}
            onClick={() => onSelect(session)}
            type="button"
          >
            <ProviderIcon provider={session.provider} />
            <span className="session-tab-main">
              <strong>{session.title}</strong>
              <small>
                {session.shortSessionId} · {session.projectName}
              </small>
            </span>
            <span className="session-tab-side">
              <small>{sessionStatusLabel(session.status)}</small>
              <em>{session.promptCount}</em>
            </span>
          </button>
        );
      })}
    </section>
  );
}

function ProviderIcon({ provider }: { provider: string }) {
  const normalized = provider.toLowerCase();
  const isCodex = normalized.includes('codex');
  const isClaude = normalized.includes('claude');
  const label = isCodex ? 'Codex CLI' : isClaude ? 'Claude Code' : provider;
  const text = isCodex ? 'Cx' : isClaude ? 'Cl' : 'AI';

  return (
    <span
      aria-label={label}
      className={`provider-icon ${isCodex ? 'codex' : isClaude ? 'claude' : 'generic'}`}
      title={label}
    >
      {text}
    </span>
  );
}

function findSession(
  sessions: SessionList,
  provider: string,
  sessionId: string,
): SessionListItem | null {
  return (
    [...sessions.active, ...sessions.maybeClosed, ...sessions.archived].find(
      (session) => session.provider === provider && session.sessionId === sessionId,
    ) ?? null
  );
}

function sessionKey(provider: string, sessionId: string) {
  return `${provider}:${sessionId}`;
}

function draftKey(provider: string, sessionId: string, draftId: number) {
  return `${provider}:${sessionId}:${draftId}`;
}

function draftStateFromListItem(item: DraftListItem): DraftState {
  return {
    id: item.id,
    provider: item.provider,
    sessionId: item.sessionId,
    contentMd: item.contentMd,
    contentHash: item.contentHash,
    status: item.status,
    copyState: item.copyState,
    copiedAt: item.copiedAt,
    lastCopiedHash: item.lastCopiedHash,
    sentAt: item.sentAt,
    matchedPromptEventId: item.matchedPromptEventId,
    updatedAt: item.updatedAt,
    isEmpty: item.isEmpty,
  };
}

function draftListItemFromState(draft: DraftState): DraftListItem {
  return {
    id: draft.id,
    provider: draft.provider,
    sessionId: draft.sessionId,
    contentMd: draft.contentMd,
    contentHash: draft.contentHash,
    status: draft.status,
    copyState: draft.copyState,
    copiedAt: draft.copiedAt,
    lastCopiedHash: draft.lastCopiedHash,
    sentAt: draft.sentAt,
    matchedPromptEventId: draft.matchedPromptEventId,
    updatedAt: draft.updatedAt,
    isEmpty: draft.isEmpty,
    preview: draftListPreview(draft.contentMd, '空草稿'),
  };
}

function insertDraftListItem(current: DraftList | null, draft: DraftState): DraftList | null {
  if (!current || current.provider !== draft.provider || current.sessionId !== draft.sessionId) {
    return current;
  }

  const item = draftListItemFromState(draft);
  return {
    ...current,
    items: [item, ...current.items.filter((existing) => existing.id !== draft.id)],
  };
}

function replaceDraftListItem(current: DraftList | null, draft: DraftState): DraftList | null {
  if (!current || current.provider !== draft.provider || current.sessionId !== draft.sessionId) {
    return current;
  }

  const item = draftListItemFromState(draft);
  return {
    ...current,
    items: current.items.map((existing) => (existing.id === draft.id ? item : existing)),
  };
}

function sessionResumeCommand(session: SessionListItem) {
  const provider = session.provider.toLowerCase();
  if (provider.includes('claude')) {
    return `claude --resume ${session.sessionId}`;
  }
  if (provider.includes('codex')) {
    return `codex resume ${session.sessionId}`;
  }
  return `${session.provider} resume ${session.sessionId}`;
}

function imageFilesFromClipboard(event: ReactClipboardEvent<HTMLDivElement>) {
  const data = event.clipboardData;
  const itemFiles = Array.from(data.items)
    .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
    .map((item) => item.getAsFile())
    .filter((file): file is File => Boolean(file));

  if (itemFiles.length) {
    return itemFiles;
  }

  return Array.from(data.files).filter((file) => file.type.startsWith('image/'));
}

function dataUrlToBlob(dataUrl: string) {
  return fetch(dataUrl).then((response) => response.blob());
}

function historyPromptCopyText(item: PromptHistoryItem) {
  const imageAttachments = item.attachments.filter(
    (attachment) => attachment.kind === 'image' && attachment.filePath,
  );
  if (!imageAttachments.length) {
    return item.hasMissingImages ? item.promptMd.trim() : stripImagePlaceholders(item.promptMd);
  }

  let text = item.promptMd;
  const appendedPaths: string[] = [];

  imageAttachments.forEach((attachment) => {
    const pathText = attachment.filePath;
    if (attachment.placeholder && text.includes(attachment.placeholder)) {
      text = text.split(attachment.placeholder).join(`\n${pathText}\n`);
      return;
    }

    appendedPaths.push(pathText);
  });

  text = stripImagePlaceholders(text);
  if (appendedPaths.length) {
    text = `${text.trimEnd()}\n\n图片附件：\n${appendedPaths.join('\n')}`;
  }

  return text.trim();
}

function stripImagePlaceholders(value: string) {
  return value
    .replace(/[ \t]*\[(?:Image|图片) #\d+\][ \t]*/g, ' ')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function formatFileSize(size: number) {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${Math.round(size / 1024)} KB`;
  }
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

function menuBadge(
  view: MainView,
  sessions: SessionList,
  status: AppStatus | null,
  searchResultCount: number,
) {
  if (view === 'sessions') {
    const total = sessions.active.length + sessions.maybeClosed.length + sessions.archived.length;
    return `${total} 个`;
  }
  if (view === 'drafts') {
    return '编辑';
  }
  if (view === 'search') {
    return searchResultCount ? `${searchResultCount} 条` : '全局';
  }
  return status?.collectorReady ? '就绪' : '待检';
}

function draftListPreview(content: string, fallbackPreview: string) {
  const preview = content.replace(/\s+/g, ' ').trim();
  if (preview) {
    return preview;
  }
  return fallbackPreview || '空草稿';
}

function draftListTitle(item: DraftListItem, index: number) {
  if (item.status === 'sent') {
    return `已发送草稿 #${item.id}`;
  }
  if (item.copyState === 'copied') {
    return `待确认草稿 #${item.id}`;
  }
  if (item.isEmpty) {
    return index === 0 ? '新草稿' : `空草稿 #${item.id}`;
  }
  return `草稿 #${item.id}`;
}

function draftListTimeLabel(item: DraftListItem) {
  if (item.sentAt) {
    return `发送于 ${formatDateTime(item.sentAt)}`;
  }
  if (item.copiedAt) {
    return `复制于 ${formatDateTime(item.copiedAt)}`;
  }
  return `更新于 ${formatDateTime(item.updatedAt)}`;
}

function draftListStateLabel(item: DraftListItem) {
  if (item.status === 'sent') {
    return '已发送';
  }
  if (item.copyState === 'copied') {
    return '待确认';
  }
  if (item.isEmpty) {
    return '空';
  }
  return '编辑中';
}

function draftListStateClass(item: DraftListItem) {
  if (item.status === 'sent') {
    return 'draft-list-state sent';
  }
  if (item.isEmpty) {
    return 'draft-list-state empty';
  }
  return 'draft-list-state';
}

function draftDetailBadge(draft: DraftState | null, hasUnsavedChanges: boolean) {
  if (!draft) {
    return '未选择';
  }
  if (draft.status === 'sent') {
    return '已发送只读';
  }
  if (hasUnsavedChanges) {
    return '未保存';
  }
  return '已保存';
}

function viewEyebrow(view: MainView) {
  if (view === 'sessions') {
    return '会话工作区';
  }
  if (view === 'drafts') {
    return '草稿工作区';
  }
  if (view === 'search') {
    return '全局检索';
  }
  return '运行设置';
}

function viewTitle(view: MainView) {
  if (view === 'sessions') {
    return '会话';
  }
  if (view === 'drafts') {
    return '草稿';
  }
  if (view === 'search') {
    return '搜索 prompt 与草稿';
  }
  return '本地运行时与 hook 注入';
}

function viewSubtitle(view: MainView, selectedSession: SessionListItem | null) {
  if (view === 'sessions') {
    return selectedSession
      ? `${selectedSession.providerLabel} · ${selectedSession.shortSessionId} · ${selectedSession.projectName}`
      : '会话会按 provider + session_id 唯一绑定。';
  }
  if (view === 'drafts') {
    return selectedSession
      ? `${selectedSession.shortSessionId} 的草稿会独立保存，切换标签不会丢失。`
      : '打开一个活动会话后，可以为它单独维护 Markdown 草稿。';
  }
  if (view === 'search') {
    return '从会话标题、历史 prompt、当前草稿中定位内容。';
  }
  return '本地端口、运行状态、Claude Code 和 Codex CLI hook 都集中在这里。';
}

function filterHistoryItems(items: PromptHistoryItem[], query: string) {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) {
    return items;
  }

  return items.filter((item) => item.promptMd.toLowerCase().includes(normalizedQuery));
}

function draftStatusLabel(
  draft: DraftState | null,
  saving: boolean,
  loading: boolean,
  hasUnsavedChanges: boolean,
) {
  if (loading) {
    return '读取中';
  }
  if (saving || hasUnsavedChanges) {
    return '保存中';
  }
  if (!draft || draft.isEmpty) {
    return '空草稿';
  }
  if (draft.status === 'sent') {
    return '已发送';
  }
  if (draft.copyState === 'copied') {
    return '已复制';
  }
  if (draft.copyState === 'cleared_after_send') {
    return '已发送';
  }
  return '已编辑';
}

function sessionStatusLabel(status: string) {
  if (status === 'active') {
    return '活动';
  }
  if (status === 'maybe_closed') {
    return '可能已关闭';
  }
  if (status === 'archived') {
    return '历史';
  }
  return status;
}

function formatDateTime(value: string | null) {
  if (!value) {
    return '暂无';
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}
