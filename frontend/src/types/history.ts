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

export type PromptHistory = {
  provider: string;
  sessionId: string;
  items: PromptHistoryItem[];
};
