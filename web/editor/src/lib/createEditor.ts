import type { EditorOptions } from '@tiptap/core';

import { Editor } from './Editor';
import { extensions } from './extensions';

const createEditor = (options: Partial<EditorOptions>): Editor => {
  const finalExtensions = [...(options.extensions || [])];

  for (const extension of extensions) {
    if (!finalExtensions.some((e) => e.name === extension.name)) {
      finalExtensions.push(extension);
    }
  }

  const editor = new Editor({ extensions: finalExtensions, ...options });

  return editor;
};

export default createEditor;
