import { useEffect, useRef, useState } from 'react';
import * as api from './api';
import {
  MessageSquare,
  FileEdit,
  Sparkles,
  Search,
  Settings,
  CheckCircle2,
  AlertCircle,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
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
import { SkillsWorkspace } from './features/skills/SkillsWorkspace';
import { PromptSearch } from './features/search/PromptSearch';
import type { PromptSearchResultItem } from './features/search/PromptSearch';
import { RuntimeSettings } from './features/settings/RuntimeSettings';
import { ImagePreviewDialog } from './features/shared/ImagePreviewDialog';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const menuItems: Array<{ id: MainView; label: string; icon: any }> = [
  { id: 'sessions', label: '会话', icon: MessageSquare },
  { id: 'drafts', label: '草稿', icon: FileEdit },
  { id: 'skills', label: '技能', icon: Sparkles },
  { id: 'search', label: '搜索', icon: Search },
  { id: 'settings', label: '设置', icon: Settings },
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
  const openSessionDrafts = (session: SessionListItem) => {
    setSelectedSession(session);
    setActiveView('drafts');
  };
  const openSessionHistory = (session: SessionListItem) => {
    setSelectedSession(session);
    setActiveView('sessions');
  };
  return (
    <main className="app-shell" aria-label="PromptHarbor 工作区">
      <aside className="left-rail" aria-label="主导航">
        <header className="mb-4">
          <div className="flex min-w-0 items-center gap-2 mb-1">
            <img
              alt=""
              aria-hidden="true"
              className="h-10 w-10 shrink-0"
              src="/promptharbor-icon.png"
            />
            <h1 className="min-w-0 whitespace-nowrap text-[19px] font-bold tracking-tight">
              PromptHarbor
            </h1>
          </div>
          <div className="flex items-center gap-2 px-1">
             <div className={cn("w-2 h-2 rounded-full", status?.recordingPaused ? "bg-amber-500" : "bg-emerald-500 animate-pulse")} />
             <p className="text-xs text-muted-foreground font-medium">{status?.recordingPaused ? '记录暂停' : '实时监控中'}</p>
          </div>
        </header>

        <nav className="flex-1 space-y-1" aria-label="主菜单">
          {menuItems.map((item) => (
            <button
              className={cn(
                "w-full group flex items-center justify-between px-3 py-2.5 rounded-lg transition-all duration-200",
                activeView === item.id
                  ? "bg-primary text-white shadow-md shadow-primary/10"
                  : "text-muted-foreground hover:bg-secondary hover:text-foreground"
              )}
              key={item.id}
              onClick={() => setActiveView(item.id)}
              type="button"
            >
              <div className="flex min-w-0 items-center gap-3">
                <item.icon size={18} className={cn("transition-colors", activeView === item.id ? "text-white" : "group-hover:text-primary")} />
                <span className="truncate whitespace-nowrap text-sm font-semibold">{item.label}</span>
              </div>
              <span className={cn(
                "shrink-0 whitespace-nowrap text-[10px] px-1.5 py-0.5 rounded-md font-bold uppercase tracking-wider tabular-nums",
                activeView === item.id ? "bg-white/20 text-white" : "bg-secondary text-muted-foreground"
              )}>
                {menuBadge(item.id, sessions, status, searchResultCount)}
              </span>
            </button>
          ))}
        </nav>

        <footer className="mt-auto pt-6 border-t border-border/50" aria-label="采集概览">
          <div className="bg-secondary/50 rounded-lg p-3">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest">采集状态</span>
              {status?.collectorReady ? <CheckCircle2 size={12} className="text-emerald-500" /> : <AlertCircle size={12} className="text-amber-500" />}
            </div>
            <div className="text-lg font-bold tracking-tight leading-none mb-1">{status?.promptEventCount ?? 0}</div>
            <div className="text-[10px] text-muted-foreground font-medium">累计捕获 Prompt</div>
          </div>
        </footer>
      </aside>

      <section className="workspace-pane" aria-label="主工作区">
        {activeView === 'sessions' ? (
          <div className="session-context-bar">
            <SessionReferenceCard
              deleting={deletingSession}
              onCopyCommand={copySelectedSessionResumeCommand}
              onDelete={deleteSelectedSession}
              onOpenPath={openSelectedSessionFolder}
              session={selectedSession}
            />
          </div>
        ) : null}

        <AnimatePresence mode="wait">
          <motion.div
            key={activeView}
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            transition={{ duration: 0.2 }}
            className="flex-1 flex flex-col min-h-0"
          >
            {error ? (
              <div className="mb-4 p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-destructive text-sm flex items-center gap-2">
                <AlertCircle size={16} />
                <span>IPC 调用失败：{error}</span>
              </div>
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
                onOpenSessionDrafts={openSessionDrafts}
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

            {activeView === 'skills' ? (
              <SkillsWorkspace onError={setError} onNotice={showCopyNotice} />
            ) : null}

            {activeView === 'drafts' ? (
              <DraftWorkspace
                {...draftWorkspace.workspaceProps}
                onOpenSessionHistory={openSessionHistory}
              />
            ) : null}
          </motion.div>
        </AnimatePresence>

        {copyNotice ? (
          <motion.div
            initial={{ opacity: 0, y: 20, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            className="fixed bottom-6 right-6 z-50 bg-emerald-600 text-white px-4 py-2.5 rounded-lg shadow-lg shadow-emerald-600/20 flex items-center gap-2 font-bold text-sm"
          >
            <CheckCircle2 size={16} />
            {copyNotice}
          </motion.div>
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
    return String(total);
  }
  if (view === 'drafts') {
    return '编辑';
  }
  if (view === 'skills') {
    return '列表';
  }
  if (view === 'search') {
    return searchResultCount ? String(searchResultCount) : '全局';
  }
  return status?.collectorReady ? '就绪' : '待检';
}
