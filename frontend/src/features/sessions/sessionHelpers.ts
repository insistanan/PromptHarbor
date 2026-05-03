import type { SessionList, SessionListItem } from '../../appTypes';

export function findSession(
  sessions: SessionList,
  provider: string,
  sessionId: string,
): SessionListItem | null {
  return (
    [...sessions.active, ...sessions.maybeClosed, ...sessions.archived].find(
      (session) => session.provider === provider && session.sessionId === sessionId,
    ) ?? null
  );
}

export function sessionKey(provider: string, sessionId: string) {
  return `${provider}:${sessionId}`;
}

export function sessionResumeCommand(session: SessionListItem) {
  const provider = session.provider.toLowerCase();
  if (provider.includes('claude')) {
    return `claude --resume ${session.sessionId}`;
  }
  if (provider.includes('codex')) {
    return `codex resume ${session.sessionId}`;
  }
  return `${session.provider} resume ${session.sessionId}`;
}

export function sessionStatusLabel(status: string) {
  if (status === 'active') {
    return '活动';
  }
  if (status === 'maybe_closed') {
    return '可能已关闭';
  }
  if (status === 'archived') {
    return '历史';
  }
  return status;
}
