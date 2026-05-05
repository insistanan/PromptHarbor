import { MilkdownProvider } from '@milkdown/react';
import type {
  CustomProviderSummary,
  DraftImageAttachment,
  DraftList,
  DraftListItem,
  DraftState,
  SessionListItem,
} from '../../appTypes';
import { SessionTabs } from '../sessions/SessionTabs';
import { displaySessionPath } from '../sessions/sessionHelpers';
import { DraftItemList } from './DraftItemList';
import { ImageAttachmentStrip } from './ImageAttachmentStrip';
import { MilkdownDraftEditor } from './MilkdownDraftEditor';
import { Layers, CheckCircle2, Copy, Clock, Sparkles, ArrowLeftRight } from 'lucide-react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export type DraftWorkspaceProps = {
  activePromptVariant: 'original' | 'optimized';
  canTogglePromptVariant: boolean;
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
  onOptimizePrompt: () => void;
  onOpenDraftContextMenu: (item: DraftListItem, x: number, y: number) => void;
  onOpenSessionHistory: (session: SessionListItem) => void;
  onPasteImages: (files: File[]) => void;
  onPreviewImage: (image: DraftImageAttachment) => void;
  onRemoveImage: (imageId: string) => void;
  onSelectOptimizationProvider: (providerId: string | null) => void;
  onSelectDraft: (item: DraftListItem) => void;
  onSelectSession: (session: SessionListItem) => void;
  onTogglePromptVariant: () => void;
  optimizationDisabledReason: string | null;
  optimizationProviderOptions: CustomProviderSummary[];
  optimizingPrompt: boolean;
  selectedDraftId: number | null;
  selectedOptimizationProviderId: string | null;
  selectedSession: SessionListItem | null;
  sessions: SessionListItem[];
};

export function DraftWorkspace({
  activePromptVariant,
  canTogglePromptVariant,
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
  onOptimizePrompt,
  onOpenDraftContextMenu,
  onOpenSessionHistory,
  onPasteImages,
  onPreviewImage,
  onRemoveImage,
  onSelectOptimizationProvider,
  onSelectDraft,
  onSelectSession,
  onTogglePromptVariant,
  optimizationDisabledReason,
  optimizationProviderOptions,
  optimizingPrompt,
  selectedDraftId,
  selectedOptimizationProviderId,
  selectedSession,
  sessions,
}: DraftWorkspaceProps) {
  const selectedSessionIsActive = selectedSession?.status === 'active';
  const selectedSessionLocation =
    displaySessionPath(selectedSession?.cwd) || selectedSession?.projectName || '暂无项目路径';
  const draftPending = draftLoading || draftSaving || draftHasUnsavedChanges;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden">
      <SessionTabs
        contextAction={{
          icon: 'sessions',
          label: '跳转到会话',
          onSelect: onOpenSessionHistory,
        }}
        emptyTitle="暂无活动会话"
        items={sessions}
        onSelect={onSelectSession}
        selected={selectedSession}
      />

      <section
        className="flex min-h-0 flex-1 overflow-hidden rounded-lg border border-border/40 bg-card shadow-sm"
        aria-label="当前草稿"
      >
        <aside className="w-[224px] shrink-0 border-r border-border/40 bg-muted/10">
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
        </aside>

        <main className="flex-1 flex flex-col min-w-0 bg-background">
          {selectedSession && selectedSessionIsActive ? (
            <div className="flex flex-col flex-1 min-h-0">
              <div className="px-5 py-3 border-b border-border/40 bg-card/80 flex items-center justify-between gap-4">
                <div className="min-w-0">
                  <h4 className="text-sm font-bold truncate text-foreground">{selectedSession.title}</h4>
                  <p
                    className="text-[11px] font-medium text-muted-foreground truncate mt-0.5"
                    title={selectedSessionLocation}
                  >
                    {selectedSession.providerLabel} · {selectedSessionLocation}
                  </p>
                </div>
                <div className={cn(
                    "flex shrink-0 items-center gap-1.5 whitespace-nowrap px-2.5 py-1 rounded-full text-[11px] font-bold border",
                    draft?.status === 'sent' ? "bg-secondary text-muted-foreground border-border" :
                    draftPending ? "bg-amber-50 text-amber-600 border-amber-200" : "bg-emerald-50 text-emerald-600 border-emerald-200"
                )}>
                  {draft?.status === 'sent' ? <Clock size={10} /> : <CheckCircle2 size={10} />}
                  {draftDetailBadge({
                    draft,
                    draftHasUnsavedChanges,
                    draftLoading,
                    draftSaving,
                  })}
                </div>
              </div>

              <div className="flex-1 overflow-y-auto p-5 space-y-5">
                {draftImages.length ? (
                  <div className="pb-4 border-b border-border/40">
                    <ImageAttachmentStrip
                      images={draftImages}
                      onCopy={onCopyImage}
                      onPreview={onPreviewImage}
                      onRemove={onRemoveImage}
                    />
                  </div>
                ) : null}

                <div className="relative">
                  <MilkdownProvider>
                    <MilkdownDraftEditor
                      canTogglePromptVariant={canTogglePromptVariant}
                      disabled={draftLoading || optimizingPrompt || draft?.status === 'sent'}
                      initialValue={currentDraftContent}
                      key={`${draftStateKey ?? 'none'}:${editorVersion}`}
                      onTogglePromptVariant={onTogglePromptVariant}
                      onPasteImages={onPasteImages}
                      onChange={onDraftChange}
                    />
                  </MilkdownProvider>
                </div>
              </div>

              <footer className="px-5 py-3 border-t border-border/40 bg-secondary/10 flex flex-col gap-3">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div className="flex min-w-0 flex-wrap items-center gap-2">
                    {draftMessage && (
                      <span className="text-[11px] font-semibold text-emerald-600 flex items-center gap-1 animate-in fade-in slide-in-from-right-2">
                        <CheckCircle2 size={12} />
                        {draftMessage}
                      </span>
                    )}
                    <div className="flex items-center gap-2">
                      <span className="text-[11px] font-semibold text-muted-foreground">版本</span>
                      <button
                        className={cn(
                          'inline-flex items-center rounded-full border px-2.5 py-1 text-[11px] font-bold transition-colors',
                          activePromptVariant === 'original'
                            ? 'border-primary/30 bg-primary/10 text-primary'
                            : 'border-border bg-background text-muted-foreground',
                        )}
                        disabled={optimizingPrompt}
                        onClick={() => {
                          if (activePromptVariant === 'optimized') {
                            onTogglePromptVariant();
                          }
                        }}
                        type="button"
                      >
                        原文
                      </button>
                      <button
                        className={cn(
                          'inline-flex items-center rounded-full border px-2.5 py-1 text-[11px] font-bold transition-colors',
                          activePromptVariant === 'optimized'
                            ? 'border-primary/30 bg-primary/10 text-primary'
                            : 'border-border bg-background text-muted-foreground',
                        )}
                        disabled={!canTogglePromptVariant || optimizingPrompt}
                        onClick={() => {
                          if (activePromptVariant === 'original') {
                            onTogglePromptVariant();
                          }
                        }}
                        type="button"
                      >
                        优化稿
                      </button>
                      {canTogglePromptVariant ? (
                        <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-muted-foreground">
                          <ArrowLeftRight size={12} />
                          Tab 切换
                        </span>
                      ) : null}
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center justify-end gap-2">
                    <select
                      className="h-10 min-w-[220px] rounded-md border border-border/60 bg-background px-3 text-xs font-semibold text-foreground outline-none transition focus:border-primary/40 focus:ring-2 focus:ring-primary/10 disabled:cursor-default disabled:opacity-60"
                      disabled={!optimizationProviderOptions.length || optimizingPrompt}
                      onChange={(event) =>
                        onSelectOptimizationProvider(event.currentTarget.value || null)
                      }
                      value={selectedOptimizationProviderId ?? ''}
                    >
                      {optimizationProviderOptions.length ? (
                        optimizationProviderOptions.map((provider) => (
                          <option key={provider.id} value={provider.id}>
                            {provider.name} · {provider.defaultModel}
                          </option>
                        ))
                      ) : (
                        <option value="">无可用供应商</option>
                      )}
                    </select>
                    <button
                      className="group flex items-center gap-2 px-4 py-2.5 rounded-md border border-primary/20 bg-primary/10 text-primary text-xs font-black hover:bg-primary/15 hover:-translate-y-0.5 transition-all active:translate-y-0 disabled:opacity-50 disabled:grayscale disabled:translate-y-0"
                      disabled={Boolean(optimizationDisabledReason) || optimizingPrompt}
                      onClick={onOptimizePrompt}
                      type="button"
                    >
                      <Sparkles size={14} className="group-hover:rotate-6 transition-transform" />
                      <span className="whitespace-nowrap">
                        {optimizingPrompt ? '优化中' : '优化提示词'}
                      </span>
                    </button>
                    <button
                      className="group flex items-center gap-2 px-6 py-2.5 rounded-md bg-primary text-white text-xs font-black shadow-lg shadow-primary/20 hover:shadow-primary/30 hover:-translate-y-0.5 transition-all active:translate-y-0 disabled:opacity-50 disabled:grayscale disabled:translate-y-0 disabled:shadow-none"
                      disabled={
                        draftLoading ||
                        optimizingPrompt ||
                        draftSaving ||
                        draftHasUnsavedChanges ||
                        !draft ||
                        !currentDraftContent.trim()
                      }
                      onClick={onCopyDraft}
                      type="button"
                    >
                      <Copy size={14} className="group-hover:rotate-12 transition-transform" />
                      <span className="whitespace-nowrap">复制到剪贴板</span>
                    </button>
                  </div>
                </div>

                {optimizationDisabledReason ? (
                  <p className="text-[11px] font-medium text-amber-700">
                    {optimizationDisabledReason}
                  </p>
                ) : null}
              </footer>
            </div>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center py-24 px-6 text-center">
              <div className="w-16 h-16 rounded-lg bg-primary/5 flex items-center justify-center text-primary mb-6">
                <Layers size={32} />
              </div>
              {/* 草稿只绑定活动会话，避免把绑定规则作为界面说明展示。 */}
              <h4 className="text-lg font-bold text-foreground mb-0">选择会话</h4>
            </div>
          )}
        </main>
      </section>
    </div>
  );
}

function draftDetailBadge({
  draft,
  draftHasUnsavedChanges,
  draftLoading,
  draftSaving,
}: {
  draft: DraftState | null;
  draftHasUnsavedChanges: boolean;
  draftLoading: boolean;
  draftSaving: boolean;
}) {
  if (!draft) {
    return '未选择';
  }
  if (draft.status === 'sent') {
    return '只读 (已发送)';
  }
  if (draftLoading) {
    return '读取中';
  }
  if (draftSaving) {
    return '保存中';
  }
  if (draftHasUnsavedChanges) {
    return '修改待保存';
  }
  return '同步就绪';
}
