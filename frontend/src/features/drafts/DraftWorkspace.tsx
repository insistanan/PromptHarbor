import { MilkdownProvider } from '@milkdown/react';
import type {
  DraftImageAttachment,
  DraftList,
  DraftListItem,
  DraftState,
  SessionListItem,
} from '../../appTypes';
import { formatDateTime } from '../../formatters';
import { SessionTabs } from '../sessions/SessionTabs';
import { DraftItemList } from './DraftItemList';
import { ImageAttachmentStrip } from './ImageAttachmentStrip';
import { MilkdownDraftEditor } from './MilkdownDraftEditor';

export type DraftWorkspaceProps = {
  currentDraftContent: string;
  draft: DraftState | null;
  draftCache: Record<string, string>;
  draftHasUnsavedChanges: boolean;
  draftImages: DraftImageAttachment[];
  draftList: DraftList | null;
  draftLoading: boolean;
  draftMessage: string | null;
  draftSaving: boolean;
  draftStateKey: string | null;
  editorVersion: number;
  onCopyDraft: () => void;
  onCopyImage: (image: DraftImageAttachment) => void;
  onCreateDraft: () => void;
  onDeleteDraft: (item: DraftListItem | null) => void;
  onDraftChange: (markdown: string) => void;
  onOpenDraftContextMenu: (item: DraftListItem, x: number, y: number) => void;
  onPasteImages: (files: File[]) => void;
  onPreviewImage: (image: DraftImageAttachment) => void;
  onRemoveImage: (imageId: string) => void;
  onSelectDraft: (item: DraftListItem) => void;
  onSelectSession: (session: SessionListItem) => void;
  selectedDraftId: number | null;
  selectedSession: SessionListItem | null;
  sessions: SessionListItem[];
};

export function DraftWorkspace({
  currentDraftContent,
  draft,
  draftCache,
  draftHasUnsavedChanges,
  draftImages,
  draftList,
  draftLoading,
  draftMessage,
  draftSaving,
  draftStateKey,
  editorVersion,
  onCopyDraft,
  onCopyImage,
  onCreateDraft,
  onDeleteDraft,
  onDraftChange,
  onOpenDraftContextMenu,
  onPasteImages,
  onPreviewImage,
  onRemoveImage,
  onSelectDraft,
  onSelectSession,
  selectedDraftId,
  selectedSession,
  sessions,
}: DraftWorkspaceProps) {
  const selectedSessionIsActive = selectedSession?.status === 'active';

  return (
    <>
      <SessionTabs
        emptyDescription="草稿只能绑定当前仍在运行或近期活动的会话。"
        emptyTitle="暂无活动会话"
        items={sessions}
        onSelect={onSelectSession}
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
            currentDraftContent={currentDraftContent}
            draftCache={draftCache}
            items={draftList?.items ?? []}
            loading={draftLoading}
            onCreate={onCreateDraft}
            onDelete={onDeleteDraft}
            onOpenContextMenu={onOpenDraftContextMenu}
            onSelect={onSelectDraft}
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
                    onCopy={onCopyImage}
                    onPreview={onPreviewImage}
                    onRemove={onRemoveImage}
                  />
                ) : null}
                <MilkdownProvider>
                  <MilkdownDraftEditor
                    disabled={draftLoading || draft?.status === 'sent'}
                    initialValue={currentDraftContent}
                    key={`${draftStateKey ?? 'none'}:${editorVersion}`}
                    onPasteImages={onPasteImages}
                    onChange={onDraftChange}
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
                      !currentDraftContent.trim()
                    }
                    onClick={onCopyDraft}
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
  );
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
