import DOMPurify from 'dompurify';
import { marked } from 'marked';

marked.setOptions({
  gfm: true,
  breaks: true,
});

const PURIFY_CONFIG = {
  USE_PROFILES: { html: true },
  FORBID_TAGS: ['script', 'style', 'iframe', 'object', 'embed', 'form'],
  FORBID_ATTR: ['style', 'onerror', 'onclick', 'onload'],
};

export function renderMarkdown(source: string): string {
  const raw = marked.parse(source ?? '', { async: false }) as string;
  return DOMPurify.sanitize(raw, PURIFY_CONFIG);
}

export function renderInlineMarkdown(source: string): string {
  const raw = marked.parseInline(source ?? '', { async: false }) as string;
  return DOMPurify.sanitize(raw, PURIFY_CONFIG);
}
