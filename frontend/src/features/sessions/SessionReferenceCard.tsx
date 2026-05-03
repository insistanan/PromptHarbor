import type { SessionListItem } from '../../appTypes';
import { displaySessionPath } from './sessionHelpers';

export function SessionReferenceCard({
  deleting,
  onCopyCommand,
  onDelete,
  onOpenPath,
  session,
}: {
  deleting: boolean;
  onCopyCommand: () => void;
  onDelete: () => void;
  onOpenPath: () => void;
  session: SessionListItem | null;
}) {
  if (!session) {
    return (
      <section className="session-reference empty" aria-label="会话引用信息">
        <span>暂无会话</span>
      </section>
    );
  }

  const projectPath = displaySessionPath(session.cwd);
  const projectLabel = session.projectName || projectPath || '暂无项目路径';
  const providerIconSrc = sessionProviderIconSrc(session.provider);

  return (
    <section className="session-reference" aria-label="会话引用信息">
      <span className="session-provider-chip">
        {providerIconSrc ? (
          <img
            alt=""
            aria-hidden="true"
            draggable={false}
            src={providerIconSrc}
          />
        ) : null}
        <span>{session.providerLabel}</span>
      </span>
      <div className="session-reference-row">
        <span className="session-project-text" title={projectPath || projectLabel}>
          {projectLabel}
        </span>
      </div>
      <div className="session-reference-actions">
        <button
          aria-label="复制恢复命令"
          className="icon-action"
          onClick={onCopyCommand}
          title="复制恢复命令"
          type="button"
        >
          <CopyIcon />
        </button>
        <button
          aria-label={deleting ? '删除中' : '删除 PromptHarbor 本地会话记录'}
          className="icon-action danger-icon-action"
          disabled={deleting}
          onClick={onDelete}
          title={deleting ? '删除中' : '删除 PromptHarbor 本地会话记录'}
          type="button"
        >
          <TrashIcon />
        </button>
      </div>
      <button
        className="project-path-button"
        disabled={!session.cwd}
        onClick={onOpenPath}
        title={projectPath || '暂无项目路径'}
        type="button"
      >
        <FolderIcon />
        <span>{projectPath || '暂无项目路径'}</span>
      </button>
    </section>
  );
}

function sessionProviderIconSrc(provider: string) {
  const normalized = provider.toLowerCase();
  if (normalized.includes('codex')) {
    return '/provider-codex.png';
  }
  if (normalized.includes('claude')) {
    return '/provider-claude-code.png';
  }
  return null;
}

function CopyIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <rect height="13" rx="2" width="13" x="8" y="8" />
      <path d="M5 16H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <path d="M3 6a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg aria-hidden="true" className="image-action-icon" viewBox="0 0 24 24">
      <path d="M3 6h18" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
      <path d="M10 11v6" />
      <path d="M14 11v6" />
    </svg>
  );
}
