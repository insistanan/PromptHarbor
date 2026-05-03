import type { DraftList, DraftListItem, DraftState } from '../../appTypes';

export function draftKey(provider: string, sessionId: string, draftId: number) {
  return `${provider}:${sessionId}:${draftId}`;
}

export function draftStateFromListItem(item: DraftListItem): DraftState {
  return {
    id: item.id,
    provider: item.provider,
    sessionId: item.sessionId,
    contentMd: item.contentMd,
    contentHash: item.contentHash,
    status: item.status,
    copyState: item.copyState,
    copiedAt: item.copiedAt,
    lastCopiedHash: item.lastCopiedHash,
    sentAt: item.sentAt,
    matchedPromptEventId: item.matchedPromptEventId,
    updatedAt: item.updatedAt,
    isEmpty: item.isEmpty,
  };
}

export function draftListItemFromState(draft: DraftState): DraftListItem {
  return {
    id: draft.id,
    provider: draft.provider,
    sessionId: draft.sessionId,
    contentMd: draft.contentMd,
    contentHash: draft.contentHash,
    status: draft.status,
    copyState: draft.copyState,
    copiedAt: draft.copiedAt,
    lastCopiedHash: draft.lastCopiedHash,
    sentAt: draft.sentAt,
    matchedPromptEventId: draft.matchedPromptEventId,
    updatedAt: draft.updatedAt,
    isEmpty: draft.isEmpty,
    preview: draftListPreview(draft.contentMd, '空草稿'),
  };
}

export function insertDraftListItem(current: DraftList | null, draft: DraftState): DraftList | null {
  if (!current || current.provider !== draft.provider || current.sessionId !== draft.sessionId) {
    return current;
  }

  const item = draftListItemFromState(draft);
  return {
    ...current,
    items: [item, ...current.items.filter((existing) => existing.id !== draft.id)],
  };
}

export function replaceDraftListItem(
  current: DraftList | null,
  draft: DraftState,
): DraftList | null {
  if (!current || current.provider !== draft.provider || current.sessionId !== draft.sessionId) {
    return current;
  }

  const item = draftListItemFromState(draft);
  return {
    ...current,
    items: current.items.map((existing) => (existing.id === draft.id ? item : existing)),
  };
}

export function draftListPreview(content: string, fallbackPreview: string) {
  const preview = content.replace(/\s+/g, ' ').trim();
  if (preview) {
    return preview;
  }
  return fallbackPreview || '空草稿';
}
