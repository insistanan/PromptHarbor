import type { ImagePreviewState } from '../../appTypes';

export function ImagePreviewDialog({
  image,
  onClose,
}: {
  image: ImagePreviewState;
  onClose: () => void;
}) {
  return (
    <div
      aria-label="图片预览"
      aria-modal="true"
      className="image-preview-backdrop"
      onClick={onClose}
      role="dialog"
    >
      <figure className="image-preview-dialog" onClick={(event) => event.stopPropagation()}>
        <button
          aria-label="关闭图片预览"
          className="image-preview-close"
          onClick={onClose}
          type="button"
        >
          ×
        </button>
        <img alt={image.alt} src={image.src} />
        <figcaption>{image.caption}</figcaption>
      </figure>
    </div>
  );
}
