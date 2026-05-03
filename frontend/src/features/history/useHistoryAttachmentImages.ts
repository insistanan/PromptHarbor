import { useEffect, useMemo, useState } from 'react';
import * as api from '../../api';
import type { PromptAttachment, PromptAttachmentDataUrl } from './historyTypes';

export function useHistoryAttachmentImages(attachments: PromptAttachment[]) {
  const imageAttachments = useMemo(
    () => attachments.filter((attachment) => attachment.kind === 'image'),
    [attachments],
  );
  const [dataUrls, setDataUrls] = useState<Record<number, string>>({});

  useEffect(() => {
    let disposed = false;

    setDataUrls({});
    imageAttachments.forEach((attachment) => {
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
  }, [imageAttachments]);

  return {
    dataUrls,
    imageAttachments,
  };
}
