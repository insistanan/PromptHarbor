import { invoke } from '@tauri-apps/api/core';

export function getAppStatus<T>() {
  return invoke<T>('app_status');
}

export function listSessions<T>() {
  return invoke<T>('list_sessions');
}

export function archiveSession<T>(payload: {
  provider: string;
  sessionId: string;
  force: boolean;
}) {
  return invoke<T>('archive_session', payload);
}

export function deleteSession<T>(payload: { provider: string; sessionId: string }) {
  return invoke<T>('delete_session', payload);
}

export function listDrafts<T>(payload: { provider: string; sessionId: string }) {
  return invoke<T>('list_drafts', payload);
}

export function createDraft<T>(payload: { provider: string; sessionId: string }) {
  return invoke<T>('create_draft', payload);
}

export function deleteDraft<T>(payload: {
  provider: string;
  sessionId: string;
  draftId: number;
}) {
  return invoke<T>('delete_draft', payload);
}

export function saveDraftById<T>(payload: {
  provider: string;
  sessionId: string;
  draftId: number;
  contentMd: string;
}) {
  return invoke<T>('save_draft_by_id', payload);
}

export function markDraftCopiedById<T>(payload: {
  provider: string;
  sessionId: string;
  draftId: number;
  contentMd: string;
}) {
  return invoke<T>('mark_draft_copied_by_id', payload);
}

export function listPromptHistory<T>(payload: {
  provider: string;
  sessionId: string;
  includeLowInfo: boolean;
}) {
  return invoke<T>('list_prompt_history', payload);
}

export function readPromptAttachmentDataUrl<T>(payload: { attachmentId: number }) {
  return invoke<T>('read_prompt_attachment_data_url', payload);
}

export function searchPrompts<T>(payload: { query: string; includeLowInfo: boolean }) {
  return invoke<T>('search_prompts', payload);
}

export function updateRuntimeConfig<T>(payload: {
  localEndpoint: string;
  recordingPaused: boolean;
  maybeClosedAfterHours: number;
  retainRawHookEvents: boolean;
  rawHookEventsRetentionDays: number;
  autostart: boolean;
}) {
  return invoke<T>('update_runtime_config', payload);
}

export function getClaudeHookStatus<T>() {
  return invoke<T>('claude_hook_status');
}

export function installClaudeHook<T>() {
  return invoke<T>('install_claude_hook');
}

export function uninstallClaudeHook<T>() {
  return invoke<T>('uninstall_claude_hook');
}

export function getCodexHookStatus<T>() {
  return invoke<T>('codex_hook_status');
}

export function installCodexHook<T>() {
  return invoke<T>('install_codex_hook');
}

export function uninstallCodexHook<T>() {
  return invoke<T>('uninstall_codex_hook');
}

export function openProjectPath(payload: { path: string }) {
  return invoke<void>('open_project_path', payload);
}
