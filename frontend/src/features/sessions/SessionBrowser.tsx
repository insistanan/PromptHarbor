import type { PromptAttachment, PromptHistoryItem } from '../history/PromptHistoryList';
import { PromptHistoryList } from '../history/PromptHistoryList';
import type { PromptHistory, SessionListItem } from '../../appTypes';
import { SessionTabs } from './SessionTabs';
import { Search, Archive, History, Info } from 'lucide-react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export type SessionBrowserProps = {
  allSessions: SessionListItem[];
  filteredHistoryItems: PromptHistoryItem[];
  hideLowInfo: boolean;
  historyLoading: boolean;
  onArchiveSelectedSession: () => void;
  onCopyPromptHistoryAttachment: (attachment: PromptAttachment) => void;
  onCopyPromptHistoryItem: (item: PromptHistoryItem) => void;
  onEditSessionNote: (session: SessionListItem, note: string) => void;
  onHideLowInfoChange: (value: boolean) => void;
  onOpenSessionDrafts: (session: SessionListItem) => void;
  onPreviewPromptHistoryAttachment: (attachment: PromptAttachment, dataUrl: string) => void;
  onSelectSession: (session: SessionListItem) => void;
  onSessionHistoryQueryChange: (value: string) => void;
  promptHistory: PromptHistory | null;
  selectedSession: SessionListItem | null;
  sessionHistoryQuery: string;
};

export function SessionBrowser({
  allSessions,
  filteredHistoryItems,
  hideLowInfo,
  historyLoading,
  onArchiveSelectedSession,
  onCopyPromptHistoryAttachment,
  onCopyPromptHistoryItem,
  onEditSessionNote,
  onHideLowInfoChange,
  onOpenSessionDrafts,
  onPreviewPromptHistoryAttachment,
  onSelectSession,
  onSessionHistoryQueryChange,
  promptHistory,
  selectedSession,
  sessionHistoryQuery,
}: SessionBrowserProps) {
  return (
    <div className="space-y-5">
      <SessionTabs
        contextAction={{
          icon: 'drafts',
          label: '跳转到草稿',
          onSelect: onOpenSessionDrafts,
        }}
        noteAction={{ onSave: onEditSessionNote }}
        emptyTitle="暂无会话"
        items={allSessions}
        onSelect={(session) => {
          onSelectSession(session);
          onSessionHistoryQueryChange('');
        }}
        selected={selectedSession}
      />

      <section className="bg-card border border-border/40 rounded-lg overflow-hidden shadow-sm" aria-label="prompt 历史">
        <header className="px-6 py-4 border-b border-border/40 bg-secondary/20 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <History size={18} className="text-primary" />
            <h3 className="text-sm font-bold tracking-tight">Prompt 历史</h3>
          </div>
          <span className="text-[10px] font-black bg-primary/10 text-primary px-2 py-0.5 rounded-full uppercase tracking-widest">
            {historyLoading
              ? '读取中'
              : `${filteredHistoryItems.length} / ${promptHistory?.items.length ?? 0} 条记录`}
          </span>
        </header>

        {selectedSession ? (
          <div className="flex flex-col">
            <div className="p-4 bg-background border-b border-border/40 flex flex-wrap items-center gap-4">
              <div className="flex-1 min-w-[200px] relative group">
                <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground group-focus-within:text-primary transition-colors" />
                <input
                  aria-label="搜索当前会话 prompt"
                  className="w-full bg-secondary/50 border-none rounded-md py-2 pl-9 pr-4 text-sm focus:ring-2 focus:ring-primary/20 transition-all outline-none"
                  onChange={(event) => onSessionHistoryQueryChange(event.currentTarget.value)}
                  placeholder="搜索历史内容..."
                  type="search"
                  value={sessionHistoryQuery}
                />
              </div>

              <div className="flex items-center gap-4">
                <label className="flex items-center gap-2 cursor-pointer select-none group">
                  <div className="relative">
                    <input
                      checked={hideLowInfo}
                      className="sr-only"
                      onChange={(event) => onHideLowInfoChange(event.currentTarget.checked)}
                      type="checkbox"
                    />
                    <div className={cn(
                        "w-8 h-4 rounded-full transition-colors",
                        hideLowInfo ? "bg-primary" : "bg-muted-foreground/30"
                    )} />
                    <div className={cn(
                        "absolute top-0.5 left-0.5 w-3 h-3 bg-white rounded-full transition-transform",
                        hideLowInfo ? "translate-x-4" : ""
                    )} />
                  </div>
                  <span className="text-xs font-bold text-muted-foreground group-hover:text-foreground transition-colors">隐藏噪音</span>
                </label>

                <button
                  className="flex items-center gap-2 px-4 py-2 rounded-md bg-primary text-white text-xs font-bold shadow-md shadow-primary/10 hover:shadow-lg hover:shadow-primary/20 transition-all disabled:opacity-50 disabled:grayscale"
                  disabled={selectedSession.status === 'archived'}
                  onClick={onArchiveSelectedSession}
                  type="button"
                >
                  <Archive size={14} />
                  <span>归档会话</span>
                </button>
              </div>
            </div>

            <div className="p-6">
              <PromptHistoryList
                items={filteredHistoryItems}
                onCopy={onCopyPromptHistoryItem}
                onCopyAttachment={onCopyPromptHistoryAttachment}
                onPreviewAttachment={onPreviewPromptHistoryAttachment}
              />
            </div>
          </div>
        ) : (
          <div className="py-24 flex flex-col items-center justify-center text-center px-6">
            <div className="w-16 h-16 rounded-full bg-primary/5 flex items-center justify-center text-primary mb-4">
              <Info size={32} />
            </div>
            {/* PromptHarbor 只记录用户提交的 prompt；这条规则不作为空状态说明展示。 */}
            <p className="text-lg font-bold text-foreground mb-0">暂无历史</p>
          </div>
        )}
      </section>
    </div>
  );
}
