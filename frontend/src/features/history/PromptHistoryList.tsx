import { useEffect, useState } from 'react';
import * as api from '../../api';

export type PromptAttachment = {
  id: number;
  kind: string;
  mimeType: string;
  filePath: string;
  fileName: string;
  fileSize: number;
  placeholder: string | null;
  createdAt: string;
};

export type PromptAttachmentDataUrl = {
  id: number;
  mimeType: string;
  dataUrl: string;
};

export type PromptHistoryItem = {
  id: number;
  promptMd: string;
  promptHash: string;
  isLowInfo: boolean;
  matchedDraftId: number | null;
  sentAt: string;
  createdAt: string;
  expectedImageCount: number;
  capturedImageCount: number;
  hasMissingImages: boolean;
  attachments: PromptAttachment[];
};

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
        api
          .readPromptAttachmentDataUrl<PromptAttachmentDataUrl>({
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

function ZoomInIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 24 24">
      <circle cx="11" cy="11" r="6" />
      <path d="m16 16 4 4" />
      <path d="M11 8v6" />
      <path d="M8 11h6" />
    </svg>
  );
}

function CopyIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 24 24">
      <rect height="12" rx="2" width="12" x="8" y="8" />
      <path d="M6 16H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
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
