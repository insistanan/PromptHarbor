import { useCallback, useEffect, useMemo, useState } from 'react';
import type { DraftContextMenuState, DraftListItem } from '../../appTypes';

export function useDraftContextMenu() {
  const [draftContextMenu, setDraftContextMenu] =
    useState<DraftContextMenuState | null>(null);

  useEffect(() => {
    if (!draftContextMenu) {
      return;
    }

    const closeMenu = () => setDraftContextMenu(null);
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closeMenu();
      }
    };

    window.addEventListener('click', closeMenu);
    window.addEventListener('keydown', closeOnEscape);
    return () => {
      window.removeEventListener('click', closeMenu);
      window.removeEventListener('keydown', closeOnEscape);
    };
  }, [draftContextMenu]);

  const openDraftContextMenu = useCallback((item: DraftListItem, x: number, y: number) => {
    setDraftContextMenu({
      item,
      x: Math.min(x, window.innerWidth - 180),
      y: Math.min(y, window.innerHeight - 90),
    });
  }, []);

  const closeDraftContextMenu = useCallback(() => setDraftContextMenu(null), []);

  return useMemo(() => ({
    closeDraftContextMenu,
    draftContextMenu,
    openDraftContextMenu,
  }), [closeDraftContextMenu, draftContextMenu, openDraftContextMenu]);
}
