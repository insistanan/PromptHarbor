import type { DraftListItem } from '../../appTypes';

export function DraftContextMenu({
  item,
  onDelete,
  x,
  y,
}: {
  item: DraftListItem;
  onDelete: (item: DraftListItem) => void;
  x: number;
  y: number;
}) {
  return (
    <div
      className="draft-context-menu"
      onClick={(event) => event.stopPropagation()}
      role="menu"
      style={{ left: x, top: y }}
    >
      <button
        className="draft-context-menu-item danger"
        onClick={() => onDelete(item)}
        role="menuitem"
        type="button"
      >
        <TrashIcon />
        <span>删除草稿 #{item.id}</span>
      </button>
    </div>
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
