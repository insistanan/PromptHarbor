import type { SessionListItem } from '../../appTypes';
import { sessionStatusLabel } from './sessionHelpers';

export function SessionTabs({
  emptyDescription,
  emptyTitle,
  items,
  onSelect,
  selected,
}: {
  emptyDescription: string;
  emptyTitle: string;
  items: SessionListItem[];
  onSelect: (session: SessionListItem) => void;
  selected: SessionListItem | null;
}) {
  if (!items.length) {
    return (
      <section className="session-tabs-empty" aria-label={emptyTitle}>
        <p className="empty-title">{emptyTitle}</p>
        <p>{emptyDescription}</p>
      </section>
    );
  }

  return (
    <section
      className="session-tabs"
      aria-label="会话标签"
      onWheel={(event) => {
        const target = event.currentTarget;
        const canScrollHorizontally = target.scrollWidth > target.clientWidth;

        if (!canScrollHorizontally) {
          return;
        }

        const horizontalDelta = event.deltaX !== 0 ? event.deltaX : event.deltaY;
        const maxScrollLeft = target.scrollWidth - target.clientWidth;
        const nextScrollLeft = Math.min(
          maxScrollLeft,
          Math.max(0, target.scrollLeft + horizontalDelta),
        );

        event.preventDefault();
        event.stopPropagation();
        target.scrollLeft = nextScrollLeft;
      }}
    >
      {items.map((session) => {
        const active =
          selected?.provider === session.provider && selected?.sessionId === session.sessionId;
        return (
          <button
            className={active ? 'session-tab active' : 'session-tab'}
            key={`${session.provider}:${session.sessionId}`}
            onClick={() => onSelect(session)}
            type="button"
          >
            <ProviderIcon provider={session.provider} />
            <span className="session-tab-main">
              <strong>{session.title}</strong>
              <small>
                {session.shortSessionId} · {session.projectName}
              </small>
            </span>
            <span className="session-tab-side">
              <small>{sessionStatusLabel(session.status)}</small>
              <em>{session.promptCount}</em>
            </span>
          </button>
        );
      })}
    </section>
  );
}

function ProviderIcon({ provider }: { provider: string }) {
  const normalized = provider.toLowerCase();
  const isCodex = normalized.includes('codex');
  const isClaude = normalized.includes('claude');
  const label = isCodex ? 'Codex CLI' : isClaude ? 'Claude Code' : provider;
  const text = isCodex ? 'Cx' : isClaude ? 'Cl' : 'AI';

  return (
    <span
      aria-label={label}
      className={`provider-icon ${isCodex ? 'codex' : isClaude ? 'claude' : 'generic'}`}
      title={label}
    >
      {text}
    </span>
  );
}
