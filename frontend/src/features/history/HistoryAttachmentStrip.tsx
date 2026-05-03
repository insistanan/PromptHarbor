import { formatFileSize } from '../../formatters';
import type { PromptAttachment } from './historyTypes';
import { useHistoryAttachmentImages } from './useHistoryAttachmentImages';

type HistoryAttachmentStripProps = {
  attachments: PromptAttachment[];
  onCopy: (attachment: PromptAttachment) => void;
  onPreview: (attachment: PromptAttachment, dataUrl: string) => void;
};

export function HistoryAttachmentStrip({
  attachments,
  onCopy,
  onPreview,
}: HistoryAttachmentStripProps) {
  const { dataUrls, imageAttachments } = useHistoryAttachmentImages(attachments);

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
