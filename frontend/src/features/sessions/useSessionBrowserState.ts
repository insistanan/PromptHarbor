import { useEffect, useState } from 'react';
import * as api from '../../api';
import type {
  ImagePreviewState,
  PromptHistory,
  SessionListItem,
} from '../../appTypes';
import { formatFileSize } from '../../formatters';
import type {
  PromptAttachment,
  PromptAttachmentDataUrl,
  PromptHistoryItem,
} from '../history/PromptHistoryList';
import type { SessionBrowserProps } from './SessionBrowser';

type UseSessionBrowserStateOptions = {
  allSessions: SessionListItem[];
  hideLowInfo: boolean;
  onError: (message: string | null) => void;
  onHideLowInfoChange: (value: boolean) => void;
  onNotice: (message: string) => void;
  onPreviewImage: (image: ImagePreviewState) => void;
  onSelectSession: (session: SessionListItem) => void;
  selectedSession: SessionListItem | null;
};

type SessionBrowserStateProps = Omit<
  SessionBrowserProps,
  'onArchiveSelectedSession'
>;

type UseSessionBrowserStateResult = {
  resetSessionHistory: () => void;
  sessionBrowserProps: SessionBrowserStateProps;
};

export function useSessionBrowserState({
  allSessions,
  hideLowInfo,
  onError,
  onHideLowInfoChange,
  onNotice,
  onPreviewImage,
  onSelectSession,
  selectedSession,
}: UseSessionBrowserStateOptions): UseSessionBrowserStateResult {
  const [sessionHistoryQuery, setSessionHistoryQuery] = useState('');
  const [promptHistory, setPromptHistory] = useState<PromptHistory | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const includeLowInfo = !hideLowInfo;
  const filteredHistoryItems = filterHistoryItems(
    promptHistory?.items ?? [],
    sessionHistoryQuery,
  );

  const resetSessionHistory = () => {
    setPromptHistory(null);
    setSessionHistoryQuery('');
    setHistoryLoading(false);
  };

  useEffect(() => {
    let disposed = false;

    if (!selectedSession) {
      setPromptHistory(null);
      setHistoryLoading(false);
      return () => {
        disposed = true;
      };
    }

    setHistoryLoading(true);
    api.listPromptHistory<PromptHistory>({
      provider: selectedSession.provider,
      sessionId: selectedSession.sessionId,
      includeLowInfo,
    })
      .then((nextHistory) => {
        if (!disposed) {
          setPromptHistory(nextHistory);
          onError(null);
        }
      })
      .catch((reason) => {
        if (!disposed) {
          onError(String(reason));
        }
      })
      .finally(() => {
        if (!disposed) {
          setHistoryLoading(false);
        }
      });

    return () => {
      disposed = true;
    };
  }, [
    includeLowInfo,
    onError,
    selectedSession?.provider,
    selectedSession?.sessionId,
    selectedSession?.promptCount,
  ]);

  const copyPromptHistoryItem = (item: PromptHistoryItem) => {
    const copyText = historyPromptCopyText(item);
    const copiedWithImagePaths = item.attachments.some(
      (attachment) => attachment.kind === 'image' && attachment.filePath,
    );
    navigator.clipboard
      .writeText(copyText)
      .then(() => {
        onNotice(
          copiedWithImagePaths ? '历史 prompt 已复制，已带图片路径' : '历史 prompt 已复制',
        );
        onError(null);
      })
      .catch((reason) => onError(String(reason)));
  };

  const previewPromptHistoryAttachment = (attachment: PromptAttachment, dataUrl: string) => {
    onPreviewImage({
      src: dataUrl,
      alt: attachment.placeholder ?? attachment.fileName,
      caption: `${attachment.placeholder ?? attachment.fileName} · ${formatFileSize(
        attachment.fileSize,
      )}`,
    });
  };

  const copyPromptHistoryAttachment = (attachment: PromptAttachment) => {
    if (!navigator.clipboard || typeof ClipboardItem === 'undefined') {
      onError('当前 WebView 不支持直接复制图片到剪切板');
      return;
    }

    api.readPromptAttachmentDataUrl<PromptAttachmentDataUrl>({
      attachmentId: attachment.id,
    })
      .then((image) => dataUrlToBlob(image.dataUrl).then((blob) => ({ image, blob })))
      .then(({ image, blob }) =>
        navigator.clipboard.write([
          new ClipboardItem({
            [image.mimeType]: blob,
          }),
        ]),
      )
      .then(() => {
        onNotice('历史图片已复制');
        onError(null);
      })
      .catch((reason) => onError(String(reason)));
  };

  return {
    resetSessionHistory,
    sessionBrowserProps: {
      allSessions,
      filteredHistoryItems,
      hideLowInfo,
      historyLoading,
      onCopyPromptHistoryAttachment: copyPromptHistoryAttachment,
      onCopyPromptHistoryItem: copyPromptHistoryItem,
      onHideLowInfoChange,
      onPreviewPromptHistoryAttachment: previewPromptHistoryAttachment,
      onSelectSession,
      onSessionHistoryQueryChange: setSessionHistoryQuery,
      promptHistory,
      selectedSession,
      sessionHistoryQuery,
    },
  };
}

function dataUrlToBlob(dataUrl: string) {
  return fetch(dataUrl).then((response) => response.blob());
}

function historyPromptCopyText(item: PromptHistoryItem) {
  const imageAttachments = item.attachments.filter(
    (attachment) => attachment.kind === 'image' && attachment.filePath,
  );
  if (!imageAttachments.length) {
    return item.hasMissingImages ? item.promptMd.trim() : stripImagePlaceholders(item.promptMd);
  }

  let text = item.promptMd;
  const appendedPaths: string[] = [];

  imageAttachments.forEach((attachment) => {
    const pathText = attachment.filePath;
    if (attachment.placeholder && text.includes(attachment.placeholder)) {
      text = text.split(attachment.placeholder).join(`\n${pathText}\n`);
      return;
    }

    appendedPaths.push(pathText);
  });

  text = stripImagePlaceholders(text);
  if (appendedPaths.length) {
    text = `${text.trimEnd()}\n\n图片附件：\n${appendedPaths.join('\n')}`;
  }

  return text.trim();
}

function stripImagePlaceholders(value: string) {
  return value
    .replace(/[ \t]*\[(?:Image|图片) #\d+\][ \t]*/g, ' ')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

function filterHistoryItems(items: PromptHistoryItem[], query: string) {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) {
    return items;
  }

  return items.filter((item) => item.promptMd.toLowerCase().includes(normalizedQuery));
}
