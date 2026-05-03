import type { ReactNode } from 'react';
import { formatFileSize } from '../../formatters';

export type ImageAttachmentStripItem<T> = {
  id: string;
  name: string;
  size: number;
  src: string | null;
  data: T;
  placeholder?: ReactNode;
};

type ImageAttachmentStripProps<T> = {
  ariaLabel?: string;
  className?: string;
  images: ImageAttachmentStripItem<T>[];
  itemClassName?: string;
  onCopy: (image: T) => void;
  onPreview: (image: T, src: string) => void;
  onRemove?: (image: T) => void;
  stopPropagation?: boolean;
};

export function ImageAttachmentStrip<T>({
  ariaLabel = '图片附件',
  className = 'image-attachment-strip',
  images,
  itemClassName,
  onCopy,
  onPreview,
  onRemove,
  stopPropagation = false,
}: ImageAttachmentStripProps<T>) {
  return (
    <section className={className} aria-label={ariaLabel}>
      {images.map((image) => {
        const canPreview = Boolean(image.src);
        const articleClassName = itemClassName
          ? `image-attachment ${itemClassName}`
          : 'image-attachment';

        return (
          <article
            className={articleClassName}
            key={image.id}
            onClick={(event) => {
              if (stopPropagation) {
                event.stopPropagation();
              }
              if (image.src) {
                onPreview(image.data, image.src);
              }
            }}
            onKeyDown={(event) => {
              if (stopPropagation) {
                event.stopPropagation();
              }
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                if (image.src) {
                  onPreview(image.data, image.src);
                }
              }
            }}
            role="button"
            tabIndex={0}
            title={canPreview ? '放大' : '加载中'}
          >
            {image.src ? (
              <img alt={image.name} src={image.src} />
            ) : (
              image.placeholder ?? <div className="history-image-placeholder">图片</div>
            )}
            <span>{formatFileSize(image.size)}</span>
            <div className="image-hover-actions" aria-label="图片操作">
              <button
                aria-label="放大图片"
                className="image-hover-button"
                disabled={!canPreview}
                onClick={(event) => {
                  event.stopPropagation();
                  if (image.src) {
                    onPreview(image.data, image.src);
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
                  onCopy(image.data);
                }}
                title="复制"
                type="button"
              >
                <CopyIcon />
              </button>
            </div>
            {onRemove ? (
              <button
                aria-label="移除图片"
                className="image-remove-button"
                onClick={(event) => {
                  event.stopPropagation();
                  onRemove(image.data);
                }}
                type="button"
              >
                ×
              </button>
            ) : null}
          </article>
        );
      })}
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
