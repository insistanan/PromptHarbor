import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { DraftImageAttachment, ImagePreviewState } from '../../appTypes';
import { formatFileSize } from '../../formatters';

type UseDraftImagesOptions = {
  onError: (message: string | null) => void;
  onMessage: (message: string | null) => void;
  onNotice: (message: string) => void;
  onPreviewImage: (image: ImagePreviewState) => void;
  selectedDraftKey: string | null;
};

export function useDraftImages({
  onError,
  onMessage,
  onNotice,
  onPreviewImage,
  selectedDraftKey,
}: UseDraftImagesOptions) {
  const [draftImages, setDraftImages] = useState<DraftImageAttachment[]>([]);
  const imageCacheRef = useRef<Record<string, DraftImageAttachment[]>>({});

  useEffect(() => {
    const imageCache = imageCacheRef.current;
    return () => {
      Object.values(imageCache).forEach((attachments) => {
        attachments.forEach((attachment) => URL.revokeObjectURL(attachment.objectUrl));
      });
    };
  }, []);

  const getDraftImages = useCallback((key: string) => imageCacheRef.current[key] ?? [], []);

  const cacheDraftImages = useCallback((key: string, images: DraftImageAttachment[]) => {
    imageCacheRef.current[key] = images;
  }, []);

  const updateDraftImages = useCallback((images: DraftImageAttachment[]) => {
    setDraftImages(images);
    if (selectedDraftKey) {
      imageCacheRef.current[selectedDraftKey] = images;
    }
  }, [selectedDraftKey]);

  const resetDraftImages = useCallback(() => setDraftImages([]), []);

  const replaceDraftImages = useCallback((images: DraftImageAttachment[]) => {
    setDraftImages(images);
  }, []);

  const deleteDraftImages = useCallback((key: string) => {
    const deletedImages = imageCacheRef.current[key] ?? [];
    deletedImages.forEach((image) => URL.revokeObjectURL(image.objectUrl));
    delete imageCacheRef.current[key];
  }, []);

  const addDraftImages = useCallback((files: File[]) => {
    if (!files.length) {
      return;
    }

    const nextImages = files.map((file) => ({
      id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
      name: file.name || 'clipboard-image',
      mimeType: file.type || 'image/png',
      size: file.size,
      objectUrl: URL.createObjectURL(file),
      blob: file,
    }));

    updateDraftImages([...draftImages, ...nextImages]);
    onMessage(`${nextImages.length} 张图片已作为附件暂存`);
  }, [draftImages, onMessage, updateDraftImages]);

  const removeDraftImage = useCallback((imageId: string) => {
    const image = draftImages.find((item) => item.id === imageId);
    if (image) {
      URL.revokeObjectURL(image.objectUrl);
    }
    updateDraftImages(draftImages.filter((item) => item.id !== imageId));
  }, [draftImages, updateDraftImages]);

  const copyDraftImage = useCallback((image: DraftImageAttachment) => {
    if (!navigator.clipboard || typeof ClipboardItem === 'undefined') {
      onError('当前 WebView 不支持直接复制图片到剪切板');
      return;
    }

    navigator.clipboard
      .write([
        new ClipboardItem({
          [image.mimeType]: image.blob,
        }),
      ])
      .then(() => {
        onMessage('图片已复制到剪切板');
        onNotice('图片已复制');
        onError(null);
      })
      .catch((reason) => onError(String(reason)));
  }, [onError, onMessage, onNotice]);

  const previewDraftImage = useCallback((image: DraftImageAttachment) => {
    onPreviewImage({
      src: image.objectUrl,
      alt: image.name,
      caption: `${image.name} · ${formatFileSize(image.size)}`,
    });
  }, [onPreviewImage]);

  return useMemo(() => ({
    addDraftImages,
    cacheDraftImages,
    copyDraftImage,
    deleteDraftImages,
    draftImages,
    getDraftImages,
    previewDraftImage,
    replaceDraftImages,
    removeDraftImage,
    resetDraftImages,
  }), [
    addDraftImages,
    cacheDraftImages,
    copyDraftImage,
    deleteDraftImages,
    draftImages,
    getDraftImages,
    previewDraftImage,
    replaceDraftImages,
    removeDraftImage,
    resetDraftImages,
  ]);
}
