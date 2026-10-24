import { Editor } from '@tiptap/core';
import { DOMSerializer } from 'prosemirror-model';

export function getHTMLContentBetween(editor: Editor, from: number, to: number) {
  const { state } = editor;
  const nodesArray: string[] = [];

  state.doc.nodesBetween(from, to, (node, pos, parent) => {
    if (parent === state.doc) {
      const serializer = DOMSerializer.fromSchema(editor.schema);
      const dom = serializer.serializeNode(node);
      const tempDiv = document.createElement('div');
      tempDiv.appendChild(dom);
      nodesArray.push(tempDiv.innerHTML);
    }
  });

  return nodesArray.join('');
}
