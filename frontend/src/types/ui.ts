import type { DraftListItem } from './drafts';

export type ImagePreviewState = {
  src: string;
  alt: string;
  caption: string;
};

export type DraftContextMenuState = {
  x: number;
  y: number;
  item: DraftListItem;
};

export type MainView = 'sessions' | 'drafts' | 'search' | 'settings';
