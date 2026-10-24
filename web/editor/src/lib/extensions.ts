import StarterKit from '@tiptap/starter-kit';
import FileHandler from '@tiptap/extension-file-handler';
import PlaceholderExtension from '@tiptap/extension-placeholder';
import ImageExtension from '@tiptap/extension-image';
import Link from '@tiptap/extension-link';

export const extensions = [
  StarterKit,
  FileHandler,
  PlaceholderExtension.configure({ showOnlyCurrent: false }),
  ImageExtension.configure({
    HTMLAttributes: { class: 'aspect-video' },
  }),
  Link.configure({
    validate: (href) => /^https?:\/\//.test(href),
    openOnClick: false,
  }),
];
