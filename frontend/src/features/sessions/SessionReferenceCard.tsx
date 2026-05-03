import type { SessionListItem } from '../../appTypes';
import { sessionResumeCommand } from './sessionHelpers';

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

  const command = sessionResumeCommand(session);

  return (
    <section className="session-reference" aria-label="会话引用信息">
      <span className="session-provider-chip">{session.providerLabel}</span>
      <div className="session-reference-row">
        <span className="session-id-text" title={command}>
          {command}
        </span>
        <button
          aria-label={`复制恢复命令：${command}`}
          className="icon-action"
          onClick={onCopyCommand}
          title={command}
          type="button"
        >
          <CopyIcon />
        </button>
      </div>
      <button
        aria-label="删除 PromptHarbor 本地会话记录"
        className="danger-icon-action"
        disabled={deleting}
        onClick={onDelete}
        title="删除 PromptHarbor 本地会话记录"
        type="button"
      >
        <TrashIcon />
        <span>{deleting ? '删除中' : '删除会话'}</span>
      </button>
      <button
        className="project-path-button"
        disabled={!session.cwd}
        onClick={onOpenPath}
        title={session.cwd ?? '暂无项目路径'}
        type="button"
      >
        <FolderIcon />
        <span>{session.cwd ?? '暂无项目路径'}</span>
      </button>
    </section>
  );
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
