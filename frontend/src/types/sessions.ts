export type SessionListItem = {
  provider: string;
  providerLabel: string;
  sessionId: string;
  shortSessionId: string;
  status: string;
  cwd: string | null;
  projectName: string;
  title: string;
  lastHookAt: string | null;
  updatedAt: string;
  promptCount: number;
  hasNonEmptyDraft: boolean;
};

export type SessionList = {
  active: SessionListItem[];
  maybeClosed: SessionListItem[];
  archived: SessionListItem[];
};

export type ArchiveSessionOutcome = {
  archived: boolean;
  requiresConfirmation: boolean;
  message: string;
};

export type DeleteSessionOutcome = {
  deleted: boolean;
  provider: string;
  sessionId: string;
  promptEventsDeleted: number;
  draftsDeleted: number;
  attachmentsDeleted: number;
  filesDeleted: number;
  message: string;
};
