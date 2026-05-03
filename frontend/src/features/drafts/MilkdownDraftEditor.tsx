import { useRef } from 'react';
import type { ClipboardEvent as ReactClipboardEvent } from 'react';
import { Editor, defaultValueCtx, rootCtx } from '@milkdown/kit/core';
import { listener, listenerCtx } from '@milkdown/kit/plugin/listener';
import { history } from '@milkdown/kit/plugin/history';
import { commonmark } from '@milkdown/kit/preset/commonmark';
import { Milkdown, useEditor } from '@milkdown/react';

export function MilkdownDraftEditor({
  disabled,
  initialValue,
  onPasteImages,
  onChange,
}: {
  disabled: boolean;
  initialValue: string;
  onPasteImages: (files: File[]) => void;
  onChange: (markdown: string) => void;
}) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const handlePaste = (event: ReactClipboardEvent<HTMLDivElement>) => {
    const files = imageFilesFromClipboard(event);
    if (!files.length) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    onPasteImages(files);
  };

  const { loading } = useEditor((root) =>
    Editor.make()
      .config((ctx) => {
        ctx.set(rootCtx, root);
        ctx.set(defaultValueCtx, initialValue);
        ctx.get(listenerCtx).markdownUpdated((_, markdown) => {
          onChangeRef.current(markdown);
        });
      })
      .use(commonmark)
      .use(history)
      .use(listener),
  );

  return (
    <div
      className={disabled || loading ? 'milkdown-host disabled' : 'milkdown-host'}
      onPasteCapture={handlePaste}
    >
      <Milkdown />
    </div>
  );
}

export function imageFilesFromClipboard(event: ReactClipboardEvent<HTMLDivElement>) {
  const data = event.clipboardData;
  const itemFiles = Array.from(data.items)
    .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
    .map((item) => item.getAsFile())
    .filter((file): file is File => Boolean(file));

  if (itemFiles.length) {
    return itemFiles;
  }

  return Array.from(data.files).filter((file) => file.type.startsWith('image/'));
}
