import type { PromptAttachment, PromptHistoryItem } from '../history/PromptHistoryList';
import { PromptHistoryList } from '../history/PromptHistoryList';
import type { PromptHistory, SessionListItem } from '../../appTypes';
import { sessionStatusLabel } from './sessionHelpers';
import { SessionTabs } from './SessionTabs';

export type SessionBrowserProps = {
  allSessions: SessionListItem[];
  filteredHistoryItems: PromptHistoryItem[];
  hideLowInfo: boolean;
  historyLoading: boolean;
  onArchiveSelectedSession: () => void;
  onCopyPromptHistoryAttachment: (attachment: PromptAttachment) => void;
  onCopyPromptHistoryItem: (item: PromptHistoryItem) => void;
  onHideLowInfoChange: (value: boolean) => void;
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
  onHideLowInfoChange,
  onPreviewPromptHistoryAttachment,
  onSelectSession,
  onSessionHistoryQueryChange,
  promptHistory,
  selectedSession,
  sessionHistoryQuery,
}: SessionBrowserProps) {
  return (
    <>
      <SessionTabs
        emptyDescription="只要 Claude Code 或 Codex CLI 发出第一条 prompt，这里就会出现对应会话。"
        emptyTitle="暂无 Agent 会话"
        items={allSessions}
        onSelect={(session) => {
          onSelectSession(session);
          onSessionHistoryQueryChange('');
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
                onChange={(event) => onSessionHistoryQueryChange(event.currentTarget.value)}
                placeholder="搜索当前会话 prompt"
                type="search"
                value={sessionHistoryQuery}
              />
              <label className="check-control">
                <input
                  checked={hideLowInfo}
                  onChange={(event) => onHideLowInfoChange(event.currentTarget.checked)}
                  type="checkbox"
                />
                隐藏低信息
              </label>
              <button
                className="secondary-action"
                disabled={selectedSession.status === 'archived'}
                onClick={onArchiveSelectedSession}
                type="button"
              >
                归档
              </button>
            </div>
            <PromptHistoryList
              items={filteredHistoryItems}
              onCopy={onCopyPromptHistoryItem}
              onCopyAttachment={onCopyPromptHistoryAttachment}
              onPreviewAttachment={onPreviewPromptHistoryAttachment}
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
  );
}
