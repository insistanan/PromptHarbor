import type { DraftImageAttachment } from '../../appTypes';
import { formatFileSize } from '../../formatters';

export function ImageAttachmentStrip({
  images,
  onCopy,
  onPreview,
  onRemove,
}: {
  images: DraftImageAttachment[];
  onCopy: (image: DraftImageAttachment) => void;
  onPreview: (image: DraftImageAttachment) => void;
  onRemove: (imageId: string) => void;
}) {
  return (
    <section className="image-attachment-strip" aria-label="图片附件">
      {images.map((image) => (
        <article
          className="image-attachment"
          key={image.id}
          onClick={() => onPreview(image)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onPreview(image);
            }
          }}
          role="button"
          tabIndex={0}
          title="点击放大图片"
        >
          <img alt={image.name} src={image.objectUrl} />
          <span>{formatFileSize(image.size)}</span>
          <div className="image-hover-actions" aria-label="图片操作">
            <button
              aria-label="放大图片"
              className="image-hover-button"
              onClick={(event) => {
                event.stopPropagation();
                onPreview(image);
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
                onCopy(image);
              }}
              title="复制"
              type="button"
            >
              <CopyIcon />
            </button>
          </div>
          <button
            aria-label="移除图片"
            className="image-remove-button"
            onClick={(event) => {
              event.stopPropagation();
              onRemove(image.id);
            }}
            type="button"
          >
            ×
          </button>
        </article>
      ))}
    </section>
  );
}

function ZoomInIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <circle cx="11" cy="11" r="6" />
      <path d="m16 16 4 4" />
      <path d="M11 8v6" />
      <path d="M8 11h6" />
    </svg>
  );
}

function CopyIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <rect height="13" rx="2" width="13" x="8" y="8" />
      <path d="M5 16H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v1" />
    </svg>
  );
}
