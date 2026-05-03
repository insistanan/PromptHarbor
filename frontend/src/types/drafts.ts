export type DraftState = {
  id: number;
  provider: string;
  sessionId: string;
  contentMd: string;
  contentHash: string;
  status: string;
  copyState: string;
  copiedAt: string | null;
  lastCopiedHash: string | null;
  sentAt: string | null;
  matchedPromptEventId: number | null;
  updatedAt: string;
  isEmpty: boolean;
};

export type DraftListItem = {
  id: number;
  provider: string;
  sessionId: string;
  contentMd: string;
  contentHash: string;
  status: string;
  copyState: string;
  copiedAt: string | null;
  lastCopiedHash: string | null;
  sentAt: string | null;
  matchedPromptEventId: number | null;
  updatedAt: string;
  isEmpty: boolean;
  preview: string;
};

export type DraftList = {
  provider: string;
  sessionId: string;
  items: DraftListItem[];
};

export type DraftImageAttachment = {
  id: string;
  name: string;
  mimeType: string;
  size: number;
  objectUrl: string;
  blob: Blob;
};
