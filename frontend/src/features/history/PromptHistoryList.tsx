import { formatDateTime } from '../../formatters';
import { HistoryAttachmentStrip } from './HistoryAttachmentStrip';
import type { PromptAttachment, PromptHistoryItem } from './historyTypes';
import { Copy, Clock, Hash, Image as ImageIcon } from 'lucide-react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export type {
  PromptAttachment,
  PromptAttachmentDataUrl,
  PromptHistoryItem,
} from './historyTypes';

export function PromptHistoryList({
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
      <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
        <Clock size={40} className="mb-4 opacity-20" />
        <p className="text-sm font-medium">暂无已发送 prompt</p>
      </div>
    );
  }

  return (
    <div className="space-y-4" aria-label="已发送 prompt 列表">
      {items.map((item) => (
        <article
          className={cn(
            "group relative bg-card border border-border/60 rounded-lg p-4 transition-all duration-200 hover:border-primary/40 hover:shadow-lg hover:shadow-primary/5 cursor-pointer",
            item.isLowInfo && "opacity-60 grayscale-[0.5]"
          )}
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
          title="复制 prompt"
        >
          <div className="absolute top-4 right-4 opacity-0 group-hover:opacity-100 transition-opacity bg-primary text-white p-1.5 rounded-lg shadow-sm">
            <Copy size={14} />
          </div>

          <header className="flex items-center justify-between mb-3 text-[11px] font-bold uppercase tracking-wider text-muted-foreground/80">
            <div className="flex items-center gap-2">
               <Clock size={12} />
               <span>{formatDateTime(item.sentAt)}</span>
            </div>
            <div className={cn(
                "px-2 py-0.5 rounded-md",
                item.isLowInfo ? "bg-muted" : item.matchedDraftId ? "bg-emerald-100 text-emerald-700" : "bg-primary/10 text-primary"
            )}>
                {item.isLowInfo ? '低信息' : item.matchedDraftId ? '匹配草稿' : '正式'}
            </div>
          </header>

          <HistoryAttachmentStrip
            attachments={item.attachments}
            onCopy={onCopyAttachment}
            onPreview={onPreviewAttachment}
          />

          {item.hasMissingImages ? (
            <div className="mt-2 p-2 rounded-lg bg-amber-50 border border-amber-100 text-amber-700 text-[11px] font-semibold flex items-center gap-2">
              <AlertTriangle size={12} />
              <span>
                图片未采集到：预期 {item.expectedImageCount} 张，已采集{' '}
                {item.capturedImageCount} 张
              </span>
            </div>
          ) : null}

          <div className="mt-3 bg-secondary/30 rounded-md p-3 border border-border/30">
            <pre className="text-xs font-mono leading-relaxed text-foreground whitespace-pre-wrap break-words">{item.promptMd}</pre>
          </div>

          <footer className="mt-4 flex items-center justify-between text-[10px] font-bold text-muted-foreground/60">
            <div className="flex items-center gap-1">
               <Hash size={10} />
               <span>{item.promptHash.slice(0, 12)}</span>
            </div>
            <div className="flex items-center gap-2">
              {item.attachments.length > 0 && (
                <div className="flex items-center gap-1">
                  <ImageIcon size={10} />
                  <span>{item.attachments.length} 图</span>
                </div>
              )}
            </div>
          </footer>
        </article>
      ))}
    </div>
  );
}

function AlertTriangle({ size, className }: { size?: number, className?: string }) {
    return (
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width={size || 24}
            height={size || 24}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            className={className}
        >
            <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>
        </svg>
    )
}
