import { formatDateTime } from '../../formatters';
import { HistoryAttachmentStrip } from './HistoryAttachmentStrip';
import type { PromptAttachment, PromptHistoryItem } from './historyTypes';

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
          {item.hasMissingImages ? (
            <p className="missing-image-notice">
              图片未采集到：预期 {item.expectedImageCount} 张，已采集{' '}
              {item.capturedImageCount} 张
            </p>
          ) : null}
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
