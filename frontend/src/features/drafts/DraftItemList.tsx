import type { DraftListItem } from '../../appTypes';
import { formatDateTime } from '../../formatters';
import { draftKey, draftListPreview } from './draftHelpers';

export function DraftItemList({
  currentDraftContent,
  draftCache,
  items,
  loading,
  onCreate,
  onDelete,
  onOpenContextMenu,
  onSelect,
  selectedDraftId,
}: {
  currentDraftContent: string;
  draftCache: Record<string, string>;
  items: DraftListItem[];
  loading: boolean;
  onCreate: () => void;
  onDelete: (item: DraftListItem | null) => void;
  onOpenContextMenu: (item: DraftListItem, x: number, y: number) => void;
  onSelect: (item: DraftListItem) => void;
  selectedDraftId: number | null;
}) {
  const selectedItem = items.find((item) => item.id === selectedDraftId) ?? null;

  return (
    <aside className="draft-session-list" aria-label="草稿列表">
      <div className="draft-list-toolbar">
        <span>{loading ? '读取中' : `${items.length} 条草稿`}</span>
        <span className="draft-list-toolbar-actions">
          <button className="tiny-action" onClick={onCreate} type="button">
            新建
          </button>
          <button
            className="tiny-action danger"
            disabled={!selectedItem}
            onClick={() => onDelete(selectedItem)}
            type="button"
          >
            删除
          </button>
        </span>
      </div>
      {!items.length ? (
        <div className="draft-list-empty">
          <p>当前会话还没有草稿</p>
        </div>
      ) : null}
      {items.map((item, index) => {
        const active = selectedDraftId === item.id;
        const key = draftKey(item.provider, item.sessionId, item.id);
        const cachedContent = active ? currentDraftContent : draftCache[key] ?? item.contentMd;
        const preview = draftListPreview(cachedContent, item.preview);

        return (
          <button
            className={active ? 'draft-list-item active' : 'draft-list-item'}
            key={item.id}
            onContextMenu={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onOpenContextMenu(item, event.clientX, event.clientY);
            }}
            onClick={() => onSelect(item)}
            type="button"
          >
            <span className="draft-list-main">
              <strong>{draftListTitle(item, index)}</strong>
              <small>{draftListTimeLabel(item)}</small>
              <em>{preview}</em>
            </span>
            <span className={draftListStateClass(item)}>
              {draftListStateLabel(item)}
            </span>
          </button>
        );
      })}
    </aside>
  );
}

function draftListTitle(item: DraftListItem, index: number) {
  if (item.status === 'sent') {
    return `已发送草稿 #${item.id}`;
  }
  if (item.copyState === 'copied') {
    return `待确认草稿 #${item.id}`;
  }
  if (item.isEmpty) {
    return index === 0 ? '新草稿' : `空草稿 #${item.id}`;
  }
  return `草稿 #${item.id}`;
}

function draftListTimeLabel(item: DraftListItem) {
  if (item.sentAt) {
    return `发送于 ${formatDateTime(item.sentAt)}`;
  }
  if (item.copiedAt) {
    return `复制于 ${formatDateTime(item.copiedAt)}`;
  }
  return `更新于 ${formatDateTime(item.updatedAt)}`;
}

function draftListStateLabel(item: DraftListItem) {
  if (item.status === 'sent') {
    return '已发送';
  }
  if (item.copyState === 'copied') {
    return '待确认';
  }
  if (item.isEmpty) {
    return '空';
  }
  return '编辑中';
}

function draftListStateClass(item: DraftListItem) {
  if (item.status === 'sent') {
    return 'draft-list-state sent';
  }
  if (item.isEmpty) {
    return 'draft-list-state empty';
  }
  return 'draft-list-state';
}
