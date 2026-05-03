import { useEffect, useRef, useState } from 'react';
import type { ClipboardEvent as ReactClipboardEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Editor, defaultValueCtx, rootCtx } from '@milkdown/kit/core';
import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
import { history } from '@milkdown/kit/plugin/history';
import { commonmark } from '@milkdown/kit/preset/commonmark';
import { Milkdown, MilkdownProvider, useEditor } from '@milkdown/react';

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
  startupErrors: string[];
};

type ClaudeHookStatus = {
  settingsPath: string;
  expectedCommand: string;
  installed: boolean;
  readable: boolean;
  message: string;
  backupPath: string | null;
};

type CodexHookStatus = {
  hooksPath: string;
  configPath: string;
  expectedCommand: string;
  hookInstalled: boolean;
  codexHooksEnabled: boolean;
  ready: boolean;
  message: string;
  hooksBackupPath: string | null;
  configBackupPath: string | null;
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

type PromptAttachment = {
  id: number;
  kind: string;
  mimeType: string;
  filePath: string;
  fileName: string;
  fileSize: number;
  placeholder: string | null;
  createdAt: string;
};

type PromptAttachmentDataUrl = {
  id: number;
  mimeType: string;
  dataUrl: string;
};

type PromptHistoryItem = {
  id: number;
  promptMd: string;
  promptHash: string;
  isLowInfo: boolean;
  matchedDraftId: number | null;
  sentAt: string;
  createdAt: string;
  attachments: PromptAttachment[];
};

type PromptHistory = {
  provider: string;
  sessionId: string;
  items: PromptHistoryItem[];
};

type PromptSearchResultItem = {
  provider: string;
  providerLabel: string;
  sessionId: string;
  shortSessionId: string;
  title: string;
  projectName: string;
  matchKind: string;
  matchLabel: string;
  snippet: string;
  isLowInfo: boolean;
  sentAt: string | null;
  updatedAt: string;
};

type PromptSearchResults = {
  query: string;
  items: PromptSearchResultItem[];
};

type RuntimeConfigDraft = {
  localEndpoint: string;
  recordingPaused: boolean;
  maybeClosedAfterHours: string;
  retainRawHookEvents: boolean;
  rawHookEventsRetentionDays: string;
  autostart: boolean;
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
  const [claudeStatus, setClaudeStatus] = useState<ClaudeHookStatus | null>(null);
  const [codexStatus, setCodexStatus] = useState<CodexHookStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [installingClaude, setInstallingClaude] = useState(false);
  const [installingCodex, setInstallingCodex] = useState(false);
  const [uninstallingClaude, setUninstallingClaude] = useState(false);
  const [uninstallingCodex, setUninstallingCodex] = useState(false);
  const [deletingSession, setDeletingSession] = useState(false);
  const [configDraft, setConfigDraft] = useState<RuntimeConfigDraft | null>(null);
  const [configDirty, setConfigDirty] = useState(false);
  const [configSaving, setConfigSaving] = useState(false);
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
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<PromptSearchResults>({
    query: '',
    items: [],
  });
  const [searchLoading, setSearchLoading] = useState(false);

  useEffect(() => {
    let disposed = false;
    const loadStatus = () => {
      invoke<AppStatus>('app_status')
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
      invoke<SessionList>('list_sessions')
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
    const loadClaudeStatus = () => {
      invoke<ClaudeHookStatus>('claude_hook_status')
        .then((nextStatus) => {
          if (!disposed) {
            setClaudeStatus(nextStatus);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };
    const loadCodexStatus = () => {
      invoke<CodexHookStatus>('codex_hook_status')
        .then((nextStatus) => {
          if (!disposed) {
            setCodexStatus(nextStatus);
          }
        })
        .catch((reason) => {
          if (!disposed) {
            setError(String(reason));
          }
        });
    };

    loadClaudeStatus();
    loadCodexStatus();
    const timer = window.setInterval(() => {
      loadStatus();
      loadSessions();
    }, 1000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, []);

  const installClaudeHook = () => {
    setInstallingClaude(true);
    invoke<ClaudeHookStatus>('install_claude_hook')
      .then((nextStatus) => {
        setClaudeStatus(nextStatus);
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setInstallingClaude(false));
  };
  const uninstallClaudeHook = () => {
    setUninstallingClaude(true);
    invoke<ClaudeHookStatus>('uninstall_claude_hook')
      .then((nextStatus) => {
        setClaudeStatus(nextStatus);
        showCopyNotice('Claude Code hook 已取消');
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setUninstallingClaude(false));
  };
  const installCodexHook = () => {
    setInstallingCodex(true);
    invoke<CodexHookStatus>('install_codex_hook')
      .then((nextStatus) => {
        setCodexStatus(nextStatus);
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setInstallingCodex(false));
  };
  const uninstallCodexHook = () => {
    setUninstallingCodex(true);
    invoke<CodexHookStatus>('uninstall_codex_hook')
      .then((nextStatus) => {
        setCodexStatus(nextStatus);
        showCopyNotice('Codex CLI hook 已取消');
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setUninstallingCodex(false));
  };
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
      setError('可能关闭判定时间必须大于 0 小时');
      return;
    }
    if (!Number.isFinite(rawHookEventsRetentionDays) || rawHookEventsRetentionDays < 0) {
      setError('raw hook 保留天数不能小于 0');
      return;
    }

    setConfigSaving(true);
    invoke<AppStatus>('update_runtime_config', {
      localEndpoint: configDraft.localEndpoint,
      recordingPaused: configDraft.recordingPaused,
      maybeClosedAfterHours,
      retainRawHookEvents: configDraft.retainRawHookEvents,
      rawHookEventsRetentionDays,
      autostart: configDraft.autostart,
    })
      .then((nextStatus) => {
        setStatus(nextStatus);
        setConfigDraft(configDraftFromStatus(nextStatus));
        setConfigDirty(false);
        showCopyNotice('运行配置已保存');
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setConfigSaving(false));
  };
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
    if (!status || configDirty || configSaving) {
      return;
    }

    setConfigDraft(configDraftFromStatus(status));
  }, [configDirty, configSaving, status]);

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
    invoke<DraftList>('list_drafts', {
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
      invoke<DraftState>('save_draft_by_id', {
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
    invoke<PromptHistory>('list_prompt_history', {
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
    let disposed = false;
    const query = searchQuery.trim();

    if (!query) {
      setSearchResults({ query: '', items: [] });
      setSearchLoading(false);
      return () => {
        disposed = true;
      };
    }

    setSearchLoading(true);
    const timer = window.setTimeout(() => {
      invoke<PromptSearchResults>('search_prompts', {
        query,
        includeLowInfo,
      })
        .then((nextResults) => {
          if (!disposed) {
            setSearchResults(nextResults);
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
            setSearchLoading(false);
          }
        });
    }, 250);

    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [includeLowInfo, searchQuery]);

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
        invoke<DraftState>('mark_draft_copied_by_id', {
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

    invoke<DraftState>('create_draft', {
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
    invoke<DraftList>('delete_draft', {
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

    invoke<PromptAttachmentDataUrl>('read_prompt_attachment_data_url', {
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

    invoke<void>('open_project_path', { path: selectedSession.cwd }).catch((reason) =>
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
    invoke<ArchiveSessionOutcome>('archive_session', {
      provider: session.provider,
      sessionId: session.sessionId,
      force,
    })
      .then((outcome) => {
        if (outcome.archived) {
          return invoke<SessionList>('list_sessions').then((nextSessions) => {
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
    invoke<DeleteSessionOutcome>('delete_session', {
      provider: session.provider,
      sessionId: session.sessionId,
    })
      .then((outcome) =>
        invoke<SessionList>('list_sessions').then((nextSessions) => {
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
              <small>{menuBadge(item.id, sessions, status, searchResults.items.length)}</small>
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
          <div className="settings-grid">
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
                    onChange={(event) =>
                      updateConfigDraft({ autostart: event.currentTarget.checked })
                    }
                    type="checkbox"
                  />
                </label>
                <label className="switch-row">
                  <span>
                    <strong>暂停记录</strong>
                    <small>开启后 hook 仍可运行，但不会写入新 prompt</small>
                  </span>
                  <input
                    checked={configDraft?.recordingPaused ?? false}
                    onChange={(event) =>
                      updateConfigDraft({ recordingPaused: event.currentTarget.checked })
                    }
                    type="checkbox"
                  />
                </label>
                <label className="config-field">
                  <span>本地采集端点</span>
                  <input
                    onChange={(event) =>
                      updateConfigDraft({ localEndpoint: event.currentTarget.value })
                    }
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
                        updateConfigDraft({ maybeClosedAfterHours: event.currentTarget.value })
                      }
                      type="number"
                      value={configDraft?.maybeClosedAfterHours ?? '12'}
                    />
                    <small>小时</small>
                  </label>
                  <label className="config-field">
                    <span>raw 保留天数</span>
                    <input
                      min="0"
                      onChange={(event) =>
                        updateConfigDraft({
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
                    <strong>保留 raw hook 事件</strong>
                    <small>仅用于短期诊断，正式历史仍只保存用户 prompt</small>
                  </span>
                  <input
                    checked={configDraft?.retainRawHookEvents ?? true}
                    onChange={(event) =>
                      updateConfigDraft({ retainRawHookEvents: event.currentTarget.checked })
                    }
                    type="checkbox"
                  />
                </label>
              </div>
              <div className="wizard-actions">
                <button
                  className="primary-action"
                  disabled={!configDirty || configSaving || !configDraft}
                  onClick={saveRuntimeConfig}
                  type="button"
                >
                  {configSaving ? '保存中' : '保存配置'}
                </button>
              </div>
            </section>

            <section className="runtime-panel" aria-label="本地运行时状态">
              <div className="section-heading">
                <h3>本地运行时</h3>
                <span>{status?.recordingPaused ? '记录暂停' : '记录中'}</span>
              </div>
              <dl className="runtime-list">
                <div>
                  <dt>PromptBox home</dt>
                  <dd>{status?.promptboxHome ?? '未初始化'}</dd>
                </div>
                <div>
                  <dt>用户配置</dt>
                  <dd>{status?.configPath ?? '未初始化'}</dd>
                </div>
                <div>
                  <dt>hook 可执行文件</dt>
                  <dd>{status?.hookBinaryPath ?? '未初始化'}</dd>
                </div>
                <div>
                  <dt>hook 状态</dt>
                  <dd className={status?.hookBinaryReady ? 'ok-text' : 'warning-text'}>
                    {status?.hookBinaryMessage ?? '等待检测'}
                  </dd>
                </div>
                <div>
                  <dt>数据库</dt>
                  <dd className={status?.databaseReady ? 'ok-text' : 'warning-text'}>
                    {status?.databaseMessage ?? '等待初始化'}
                  </dd>
                </div>
                <div>
                  <dt>采集端点</dt>
                  <dd className={status?.collectorReady ? 'ok-text' : 'warning-text'}>
                    {status?.collectorMessage ?? '等待启动'}
                  </dd>
                </div>
                <div>
                  <dt>记录状态</dt>
                  <dd className={status?.recordingPaused ? 'warning-text' : 'ok-text'}>
                    {status?.recordingPaused ? '已暂停，不写入 prompt' : '记录中'}
                  </dd>
                </div>
                <div>
                  <dt>Agent 会话</dt>
                  <dd>{status ? `${status.sessionCount} 个` : '0 个'}</dd>
                </div>
                <div>
                  <dt>正式 prompt</dt>
                  <dd>{status ? `${status.promptEventCount} 条` : '0 条'}</dd>
                </div>
                <div>
                  <dt>已采集事件</dt>
                  <dd>{status ? `${status.receivedPromptEvents} 条` : '0 条'}</dd>
                </div>
                <div>
                  <dt>spool 导入</dt>
                  <dd>{status ? `${status.importedSpoolEvents} 条` : '0 条'}</dd>
                </div>
              </dl>
              {status?.startupErrors.length ? (
                <div className="runtime-errors">
                  {status.startupErrors.map((item) => (
                    <p key={item}>{item}</p>
                  ))}
                </div>
              ) : null}
            </section>

            <section className="wizard-panel" aria-label="Claude Code 配置向导">
              <div className="section-heading">
                <h3>Claude Code</h3>
                <span className={claudeStatus?.installed ? 'ok-text' : 'warning-text'}>
                  {claudeStatus?.installed ? 'hook 已安装' : 'hook 未安装'}
                </span>
              </div>
              <dl className="runtime-list">
                <div>
                  <dt>配置文件</dt>
                  <dd>{claudeStatus?.settingsPath ?? '读取中'}</dd>
                </div>
                <div>
                  <dt>hook 命令</dt>
                  <dd>{claudeStatus?.expectedCommand ?? '读取中'}</dd>
                </div>
                <div>
                  <dt>检测结果</dt>
                  <dd>{claudeStatus?.message ?? '等待检测'}</dd>
                </div>
                {claudeStatus?.backupPath ? (
                  <div>
                    <dt>备份文件</dt>
                    <dd>{claudeStatus.backupPath}</dd>
                  </div>
                ) : null}
              </dl>
              <div className="wizard-actions">
                <button
                  className="primary-action"
                  disabled={installingClaude || uninstallingClaude || claudeStatus?.installed}
                  onClick={installClaudeHook}
                  type="button"
                >
                  {installingClaude ? '安装中' : '安装用户级 hook'}
                </button>
                <button
                  className="secondary-action"
                  disabled={installingClaude || uninstallingClaude || !claudeStatus?.installed}
                  onClick={uninstallClaudeHook}
                  type="button"
                >
                  {uninstallingClaude ? '取消中' : '取消 hook'}
                </button>
              </div>
            </section>

            <section className="wizard-panel" aria-label="Codex CLI 配置向导">
              <div className="section-heading">
                <h3>Codex CLI</h3>
                <span className={codexStatus?.ready ? 'ok-text' : 'warning-text'}>
                  {codexStatus?.ready ? 'hook 可用' : 'hook 未就绪'}
                </span>
              </div>
              <dl className="runtime-list">
                <div>
                  <dt>hooks.json</dt>
                  <dd>{codexStatus?.hooksPath ?? '读取中'}</dd>
                </div>
                <div>
                  <dt>config.toml</dt>
                  <dd>{codexStatus?.configPath ?? '读取中'}</dd>
                </div>
                <div>
                  <dt>hook 命令</dt>
                  <dd>{codexStatus?.expectedCommand ?? '读取中'}</dd>
                </div>
                <div>
                  <dt>hook 状态</dt>
                  <dd className={codexStatus?.hookInstalled ? 'ok-text' : 'warning-text'}>
                    {codexStatus?.hookInstalled ? '已安装' : '未安装'}
                  </dd>
                </div>
                <div>
                  <dt>feature</dt>
                  <dd className={codexStatus?.codexHooksEnabled ? 'ok-text' : 'warning-text'}>
                    {codexStatus?.codexHooksEnabled ? 'codex_hooks 已开启' : 'codex_hooks 未开启'}
                  </dd>
                </div>
                <div>
                  <dt>检测结果</dt>
                  <dd>{codexStatus?.message ?? '等待检测'}</dd>
                </div>
                {codexStatus?.hooksBackupPath ? (
                  <div>
                    <dt>hooks 备份</dt>
                    <dd>{codexStatus.hooksBackupPath}</dd>
                  </div>
                ) : null}
                {codexStatus?.configBackupPath ? (
                  <div>
                    <dt>config 备份</dt>
                    <dd>{codexStatus.configBackupPath}</dd>
                  </div>
                ) : null}
              </dl>
              <div className="wizard-actions">
                <button
                  className="primary-action"
                  disabled={installingCodex || uninstallingCodex || codexStatus?.ready}
                  onClick={installCodexHook}
                  type="button"
                >
                  {installingCodex ? '安装中' : '安装用户级 hook'}
                </button>
                <button
                  className="secondary-action"
                  disabled={installingCodex || uninstallingCodex || !codexStatus?.hookInstalled}
                  onClick={uninstallCodexHook}
                  type="button"
                >
                  {uninstallingCodex ? '取消中' : '取消 hook'}
                </button>
              </div>
            </section>
          </div>
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
          <section className="search-panel" aria-label="prompt 搜索">
            <div className="section-heading">
              <h3>搜索</h3>
              <span>{searchLoading ? '搜索中' : `${searchResults.items.length} 条`}</span>
            </div>
            <div className="search-body">
              <div className="search-row">
                <input
                  aria-label="搜索会话标题、prompt 和当前草稿"
                  onChange={(event) => setSearchQuery(event.currentTarget.value)}
                  placeholder="搜索会话标题、首条 prompt、已发送 prompt、当前草稿"
                  type="search"
                  value={searchQuery}
                />
                <label className="check-control">
                  <input
                    checked={hideLowInfo}
                    onChange={(event) => setHideLowInfo(event.currentTarget.checked)}
                    type="checkbox"
                  />
                  隐藏低信息
                </label>
              </div>
              <SearchResultsList
                items={searchResults.items}
                onSelect={setSelectedSessionFromSearch}
              />
            </div>
          </section>
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

function PromptHistoryList({
  items,
  onCopy,
  onCopyAttachment,
  onPreviewAttachment,
}: {
  items: PromptHistoryItem[];
  onCopy: (item: PromptHistoryItem) => void;
  onCopyAttachment: (attachment: PromptAttachment) => void;
  onPreviewAttachment: (attachment: PromptAttachment, dataUrl: string) => void;
}) {
  if (!items.length) {
    return (
      <div className="history-empty">
        <p>暂无已发送 prompt</p>
      </div>
    );
  }

  return (
    <div className="history-list" aria-label="已发送 prompt 列表">
      {items.map((item) => (
        <article
          className={item.isLowInfo ? 'prompt-card low-info copyable' : 'prompt-card copyable'}
          key={item.id}
          onClick={() => onCopy(item)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onCopy(item);
            }
          }}
          role="button"
          tabIndex={0}
          title="点击复制这条 prompt"
        >
          <header>
            <span>{formatDateTime(item.sentAt)}</span>
            <span>{item.isLowInfo ? '低信息' : item.matchedDraftId ? '匹配草稿' : '正式'}</span>
          </header>
          <HistoryAttachmentStrip
            attachments={item.attachments}
            onCopy={onCopyAttachment}
            onPreview={onPreviewAttachment}
          />
          <pre>{item.promptMd}</pre>
          <footer>
            <span>hash {item.promptHash.slice(0, 12)}</span>
            <span>
              {item.attachments.length
                ? `${item.attachments.length} 张图 · 点击卡片复制`
                : '点击复制'}
            </span>
          </footer>
        </article>
      ))}
    </div>
  );
}

function HistoryAttachmentStrip({
  attachments,
  onCopy,
  onPreview,
}: {
  attachments: PromptAttachment[];
  onCopy: (attachment: PromptAttachment) => void;
  onPreview: (attachment: PromptAttachment, dataUrl: string) => void;
}) {
  const [dataUrls, setDataUrls] = useState<Record<number, string>>({});

  useEffect(() => {
    let disposed = false;
    attachments
      .filter((attachment) => attachment.kind === 'image')
      .forEach((attachment) => {
        invoke<PromptAttachmentDataUrl>('read_prompt_attachment_data_url', {
          attachmentId: attachment.id,
        })
          .then((image) => {
            if (!disposed) {
              setDataUrls((current) => ({ ...current, [image.id]: image.dataUrl }));
            }
          })
          .catch(() => {
            if (!disposed) {
              setDataUrls((current) => ({ ...current, [attachment.id]: '' }));
            }
          });
      });

    return () => {
      disposed = true;
    };
  }, [attachments]);

  const imageAttachments = attachments.filter((attachment) => attachment.kind === 'image');
  if (!imageAttachments.length) {
    return null;
  }

  return (
    <section className="history-attachment-strip" aria-label="历史 prompt 图片附件">
      {imageAttachments.map((attachment) => (
        <article
          className="image-attachment history-image-attachment"
          key={attachment.id}
          onClick={(event) => {
            event.stopPropagation();
            const dataUrl = dataUrls[attachment.id];
            if (dataUrl) {
              onPreview(attachment, dataUrl);
            }
          }}
          onKeyDown={(event) => {
            event.stopPropagation();
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              const dataUrl = dataUrls[attachment.id];
              if (dataUrl) {
                onPreview(attachment, dataUrl);
              }
            }
          }}
          role="button"
          tabIndex={0}
          title={dataUrls[attachment.id] ? '点击放大图片' : '图片加载中'}
        >
          {dataUrls[attachment.id] ? (
            <img
              alt={attachment.placeholder ?? attachment.fileName}
              src={dataUrls[attachment.id]}
            />
          ) : (
            <div className="history-image-placeholder">图片</div>
          )}
          <span>{formatFileSize(attachment.fileSize)}</span>
          <div className="image-hover-actions" aria-label="图片操作">
            <button
              aria-label="放大图片"
              className="image-hover-button"
              disabled={!dataUrls[attachment.id]}
              onClick={(event) => {
                event.stopPropagation();
                const dataUrl = dataUrls[attachment.id];
                if (dataUrl) {
                  onPreview(attachment, dataUrl);
                }
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
                onCopy(attachment);
              }}
              title="复制"
              type="button"
            >
              <CopyIcon />
            </button>
          </div>
        </article>
      ))}
    </section>
  );
}

function SearchResultsList({
  items,
  onSelect,
}: {
  items: PromptSearchResultItem[];
  onSelect: (item: PromptSearchResultItem) => void;
}) {
  if (!items.length) {
    return (
      <div className="history-empty">
        <p>暂无搜索结果</p>
      </div>
    );
  }

  return (
    <div className="search-results" aria-label="搜索结果列表">
      {items.map((item, index) => (
        <button
          className={item.isLowInfo ? 'search-result low-info' : 'search-result'}
          key={`${item.provider}:${item.sessionId}:${item.matchKind}:${index}`}
          onClick={() => onSelect(item)}
          type="button"
        >
          <span>
            <strong>{item.title}</strong>
            <small>
              {item.matchLabel} · {item.providerLabel} · {item.shortSessionId} ·{' '}
              {item.projectName}
            </small>
          </span>
          <em>{item.snippet}</em>
          <small>{formatDateTime(item.sentAt ?? item.updatedAt)}</small>
        </button>
      ))}
    </div>
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
    return stripImagePlaceholders(item.promptMd);
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
