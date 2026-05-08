import type { SessionListItem } from '../../appTypes';
import { displaySessionPath, sessionStatusLabel } from './sessionHelpers';
import { FileEdit, MessageSquare, Pencil } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function SessionTabs({
  contextAction,
  emptyTitle,
  items,
  noteAction,
  onSelect,
  selected,
}: {
  contextAction?: {
    icon: 'drafts' | 'sessions';
    label: string;
    onSelect: (session: SessionListItem) => void;
  };
  emptyTitle: string;
  items: SessionListItem[];
  noteAction?: {
    onSave: (session: SessionListItem, note: string) => void;
  };
  onSelect: (session: SessionListItem) => void;
  selected: SessionListItem | null;
}) {
  const [contextMenu, setContextMenu] = useState<{
    session: SessionListItem;
    x: number;
    y: number;
  } | null>(null);
  const [editNote, setEditNote] = useState<{
    session: SessionListItem;
    value: string;
  } | null>(null);
  const noteInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!contextMenu) {
      return;
    }

    const closeMenu = () => setContextMenu(null);
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
  }, [contextMenu]);

  if (!items.length) {
    // 空状态保持短文案，采集规则和绑定规则留在代码与文档中说明。
    return (
      <section className="flex flex-col items-center justify-center py-12 px-6 rounded-lg border border-dashed border-border bg-muted/20 text-center" aria-label={emptyTitle}>
        <p className="text-lg font-bold text-foreground mb-0">{emptyTitle}</p>
      </section>
    );
  }

  return (
    <section
      className="flex min-h-[72px] shrink-0 gap-4 overflow-x-auto px-1 py-1 no-scrollbar select-none overscroll-contain"
      aria-label="会话标签"
      onWheel={(event) => {
        const target = event.currentTarget;
        const maxScrollLeft = target.scrollWidth - target.clientWidth;

        if (maxScrollLeft <= 0) {
          return;
        }

        const horizontalDelta =
          Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;

        event.preventDefault();
        event.stopPropagation();
        target.scrollLeft = Math.max(
          0,
          Math.min(maxScrollLeft, target.scrollLeft + horizontalDelta),
        );
      }}
    >
      {items.map((session) => {
        const active =
          selected?.provider === session.provider && selected?.sessionId === session.sessionId;
        const location = displaySessionPath(session.cwd) || session.projectName;
        const providerTone = sessionProviderTone(session.provider);
        return (
          <button
            className={cn(
              "group flex-none min-h-[62px] w-[260px] flex items-center gap-3 p-3 rounded-lg border text-left transition-all duration-200 ease-out transform-gpu hover:-translate-y-0.5 hover:scale-[1.015] active:scale-[0.995] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-1",
              active ? providerTone.active : providerTone.idle,
            )}
            key={`${session.provider}:${session.sessionId}`}
            onContextMenu={(event) => {
              if (!contextAction && !noteAction) {
                return;
              }
              event.preventDefault();
              event.stopPropagation();
              const itemCount = (contextAction ? 1 : 0) + (noteAction ? 1 : 0);
              setContextMenu({
                session,
                x: Math.min(event.clientX, window.innerWidth - 184),
                y: Math.min(event.clientY, window.innerHeight - (44 + itemCount * 36)),
              });
            }}
            onClick={() => onSelect(session)}
            type="button"
          >
            <div className="flex-1 min-w-0">
              <div className={cn("text-sm font-bold truncate transition-colors", active ? "text-primary" : "text-foreground")}>
                {session.title}
              </div>
              <div
                className="text-[10px] text-muted-foreground font-medium truncate mt-0.5"
                title={location}
              >
                {location}
              </div>
            </div>
            <div className="flex flex-col items-end gap-1">
              <div className={cn(
                "whitespace-nowrap text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded-md",
                session.status === 'active' ? "bg-emerald-100 text-emerald-700" : "bg-muted text-muted-foreground"
              )}>
                {sessionStatusLabel(session.status)}
              </div>
              <div className="text-xs font-black text-muted-foreground/40 group-hover:text-primary/40 transition-colors">
                {session.promptCount}
              </div>
            </div>
          </button>
        );
      })}
      {(contextAction || noteAction) && contextMenu ? (
        <div
          className="session-tab-context-menu"
          onClick={(event) => event.stopPropagation()}
          role="menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          {contextAction ? (
            <button
              className="session-tab-context-menu-item"
              onClick={() => {
                contextAction.onSelect(contextMenu.session);
                setContextMenu(null);
              }}
              role="menuitem"
              type="button"
            >
              {contextAction.icon === 'drafts' ? <FileEdit size={14} /> : <MessageSquare size={14} />}
              <span>{contextAction.label}</span>
            </button>
          ) : null}
          {noteAction ? (
            <button
              className="session-tab-context-menu-item"
              onClick={() => {
                setEditNote({ session: contextMenu.session, value: contextMenu.session.title });
                setContextMenu(null);
                setTimeout(() => noteInputRef.current?.select(), 30);
              }}
              role="menuitem"
              type="button"
            >
              <Pencil size={14} />
              <span>编辑备注</span>
            </button>
          ) : null}
        </div>
      ) : null}

      {noteAction && editNote ? (
        <div
          className="fixed inset-0 z-[90] flex items-center justify-center bg-black/40"
          onClick={() => setEditNote(null)}
        >
          <div
            className="bg-white rounded-xl p-6 shadow-2xl w-[340px] max-w-[90vw]"
            onClick={(event) => event.stopPropagation()}
          >
            <h2 className="text-sm font-bold text-foreground mb-1">编辑会话备注</h2>
            <p className="text-xs text-muted-foreground mb-3">留空则恢复自动标题</p>
            <input
              ref={noteInputRef}
              autoFocus
              className="w-full border border-border rounded-lg px-3 py-2 text-sm outline-none focus:ring-2 focus:ring-primary/30 mb-4"
              maxLength={120}
              onChange={(event) => setEditNote({ ...editNote, value: event.target.value })}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  noteAction.onSave(editNote.session, editNote.value);
                  setEditNote(null);
                }
                if (event.key === 'Escape') {
                  setEditNote(null);
                }
              }}
              placeholder="输入备注名称…"
              type="text"
              value={editNote.value}
            />
            <div className="flex gap-2 justify-end">
              <button
                className="px-3 py-1.5 rounded-lg text-sm font-bold text-muted-foreground hover:bg-muted/60 transition-colors"
                onClick={() => setEditNote(null)}
                type="button"
              >
                取消
              </button>
              <button
                className="px-3 py-1.5 rounded-lg text-sm font-bold bg-primary text-white hover:bg-primary/90 transition-colors"
                onClick={() => {
                  noteAction.onSave(editNote.session, editNote.value);
                  setEditNote(null);
                }}
                type="button"
              >
                保存
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function sessionProviderTone(provider: string) {
  const normalized = provider.toLowerCase();
  if (normalized.includes('codex')) {
    return {
      active:
        'border-sky-500 bg-sky-100/90 shadow-md shadow-sky-200/70 ring-sky-300/70 hover:bg-sky-100 hover:scale-[1.02]',
      idle:
        'border-sky-200 bg-sky-50/80 shadow-sm shadow-sky-100/50 ring-sky-300/60 hover:border-sky-400 hover:bg-sky-100/85 hover:shadow-sky-200/70',
    };
  }
  if (normalized.includes('claude')) {
    return {
      active:
        'border-orange-400 bg-orange-100/90 shadow-md shadow-orange-200/70 ring-orange-300/70 hover:bg-orange-100 hover:scale-[1.02]',
      idle:
        'border-orange-200 bg-orange-50/80 shadow-sm shadow-orange-100/60 ring-orange-300/60 hover:border-orange-400 hover:bg-orange-100/85 hover:shadow-orange-200/70',
    };
  }
  return {
    active:
      'border-primary bg-primary/10 shadow-md shadow-primary/10 ring-primary/30 hover:bg-primary/15 hover:scale-[1.02]',
    idle:
      'border-border bg-muted/30 shadow-sm ring-primary/25 hover:border-muted-foreground/40 hover:bg-muted/50',
  };
}
