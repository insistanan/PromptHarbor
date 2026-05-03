import type { DraftImageAttachment } from '../../appTypes';
import { ImageAttachmentStrip as SharedImageAttachmentStrip } from '../shared/ImageAttachmentStrip';

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
    <SharedImageAttachmentStrip
      images={images.map((image) => ({
        data: image,
        id: image.id,
        name: image.name,
        size: image.size,
        src: image.objectUrl,
      }))}
      onCopy={onCopy}
      onPreview={(image) => onPreview(image)}
      onRemove={(image) => onRemove(image.id)}
    />
  );
}
