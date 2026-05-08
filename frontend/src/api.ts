import { invoke } from '@tauri-apps/api/core';
import type { CustomProviderProtocol } from './types/providers';

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

export function setSessionNote(payload: { provider: string; sessionId: string; note: string }) {
  return invoke<void>('set_session_note', payload);
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

export function listSkills<T>() {
  return invoke<T>('list_skills');
}

export function readSkillDetail<T>(payload: { skillFile: string }) {
  return invoke<T>('read_skill_detail', payload);
}

export function translateSkill<T>(payload: {
  providerId: string;
  skillId: string;
  skillFile: string;
  contentHash: string;
  force?: boolean;
}) {
  return invoke<T>('translate_skill', payload);
}

export function listImportedSkillPackages<T>() {
  return invoke<T>('list_imported_skill_packages');
}

export function importSkillZip<T>(payload: { zipPath: string }) {
  return invoke<T>('import_skill_zip', payload);
}

export function importSkillZipBytes<T>(payload: {
  originalFileName: string;
  zipBytes: number[];
}) {
  return invoke<T>('import_skill_zip_bytes', payload);
}

export function installImportedSkill<T>(payload: {
  packageId: string;
  targets: string[];
  targetSkillName: string | null;
  overwrite: boolean;
}) {
  return invoke<T>('install_imported_skill', payload);
}

export function exportSkillToLibrary<T>(payload: {
  skillId: string;
  skillFile: string;
  contentHash: string;
}) {
  return invoke<T>('export_skill_to_library', payload);
}

export function transferSkill<T>(payload: {
  skillId: string;
  skillFile: string;
  contentHash: string;
  targetProvider: string;
  targetSkillName: string | null;
  overwrite: boolean;
}) {
  return invoke<T>('transfer_skill', payload);
}

export function deleteSkill<T>(payload: {
  skillId: string;
  skillFile: string;
  contentHash: string;
}) {
  return invoke<T>('delete_skill', payload);
}

export function deleteImportedSkillPackage<T>(payload: { packageId: string }) {
  return invoke<T>('delete_imported_skill_package', payload);
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

export function listCustomProviders<T>() {
  return invoke<T>('list_custom_providers');
}

export function saveCustomProvider<T>(draft: {
  providerId: string | null;
  name: string;
  protocol: CustomProviderProtocol;
  baseUrl: string;
  apiKey: string;
  defaultModel: string;
  enabled: boolean;
}) {
  return invoke<T>('save_custom_provider', { draft });
}

export function deleteCustomProvider<T>(payload: { providerId: string }) {
  return invoke<T>('delete_custom_provider', payload);
}

export function testCustomProvider<T>(draft: {
  providerId: string | null;
  name: string;
  protocol: CustomProviderProtocol;
  baseUrl: string;
  apiKey: string;
  defaultModel: string;
  enabled: boolean;
}) {
  return invoke<T>('test_custom_provider', { draft });
}

export function optimizePromptWithCustomProvider<T>(payload: {
  providerId: string;
  promptMd: string;
}) {
  return invoke<T>('optimize_prompt_with_custom_provider', payload);
}

export type HookProvider = 'claude' | 'codex';

export function getHookStatus<T>(provider: HookProvider) {
  return invoke<T>('hook_status', { provider });
}

export function installHook<T>(provider: HookProvider) {
  return invoke<T>('install_hook', { provider });
}

export function uninstallHook<T>(provider: HookProvider) {
  return invoke<T>('uninstall_hook', { provider });
}

export function openProjectPath(payload: { path: string }) {
  return invoke<void>('open_project_path', payload);
}
