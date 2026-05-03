import { useEffect, useRef, useState } from 'react';
import * as api from './api';
import type {
  AppStatus,
  ArchiveSessionOutcome,
  DeleteSessionOutcome,
  ImagePreviewState,
  MainView,
  SessionList,
  SessionListItem,
} from './appTypes';
import { useAppPollingState } from './features/app/useAppPollingState';
import { DraftContextMenu } from './features/drafts/DraftContextMenu';
import { DraftWorkspace } from './features/drafts/DraftWorkspace';
import { useDraftWorkspaceState } from './features/drafts/useDraftWorkspaceState';
import { SessionBrowser } from './features/sessions/SessionBrowser';
import { SessionReferenceCard } from './features/sessions/SessionReferenceCard';
import {
  findSession,
  sessionResumeCommand,
} from './features/sessions/sessionHelpers';
import { useSessionBrowserState } from './features/sessions/useSessionBrowserState';
import { PromptSearch } from './features/search/PromptSearch';
import type { PromptSearchResultItem } from './features/search/PromptSearch';
import { RuntimeSettings } from './features/settings/RuntimeSettings';
import { ImagePreviewDialog } from './features/shared/ImagePreviewDialog';

const menuItems: Array<{ id: MainView; label: string }> = [
  { id: 'sessions', label: '会话' },
  { id: 'drafts', label: '草稿' },
  { id: 'search', label: '搜索' },
  { id: 'settings', label: '设置' },
];

export function App() {
  const [activeView, setActiveView] = useState<MainView>('sessions');
  const [copyNotice, setCopyNotice] = useState<string | null>(null);
  const [deletingSession, setDeletingSession] = useState(false);
  const copyNoticeTimerRef = useRef<number | null>(null);
  const [imagePreview, setImagePreview] = useState<ImagePreviewState | null>(null);
  const [hideLowInfo, setHideLowInfo] = useState(false);
  const [searchResultCount, setSearchResultCount] = useState(0);
  const {
    allSessions,
    error,
    selectedSession,
    setError,
    setSelectedSession,
    setSessions,
    setStatus,
    sessions,
    status,
  } = useAppPollingState();

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

  const draftWorkspace = useDraftWorkspaceState({
    activeSessions: sessions.active,
    activeView,
    onError: setError,
    onNotice: showCopyNotice,
    onPreviewImage: setImagePreview,
    onSelectSession: setSelectedSession,
    selectedSession,
  });
  const sessionBrowser = useSessionBrowserState({
    allSessions,
    hideLowInfo,
    onError: setError,
    onHideLowInfoChange: setHideLowInfo,
    onNotice: showCopyNotice,
    onPreviewImage: setImagePreview,
    onSelectSession: setSelectedSession,
    selectedSession,
  });

  useEffect(() => {
    return () => {
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
          sessionBrowser.resetSessionHistory();
          draftWorkspace.resetDraftWorkspace();
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
        {draftWorkspace.draftContextMenu ? (
          <DraftContextMenu
            item={draftWorkspace.draftContextMenu.item}
            onDelete={draftWorkspace.deleteDraftItem}
            x={draftWorkspace.draftContextMenu.x}
            y={draftWorkspace.draftContextMenu.y}
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
          <SessionBrowser
            {...sessionBrowser.sessionBrowserProps}
            onArchiveSelectedSession={archiveSelectedSession}
          />
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
          <DraftWorkspace {...draftWorkspace.workspaceProps} />
        ) : null}
      </section>
    </main>
  );
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
