import { ImageAttachmentStrip as SharedImageAttachmentStrip } from '../shared/ImageAttachmentStrip';
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
    <SharedImageAttachmentStrip
      ariaLabel="历史 prompt 图片附件"
      className="history-attachment-strip"
      images={imageAttachments.map((attachment) => ({
        data: attachment,
        id: String(attachment.id),
        name: attachment.placeholder ?? attachment.fileName,
        placeholder: <div className="history-image-placeholder">图片</div>,
        size: attachment.fileSize,
        src: dataUrls[attachment.id] ?? null,
      }))}
      itemClassName="history-image-attachment"
      onCopy={onCopy}
      onPreview={(attachment, dataUrl) => onPreview(attachment, dataUrl)}
      stopPropagation
    />
  );
}
