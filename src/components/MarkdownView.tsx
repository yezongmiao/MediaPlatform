import { useMemo } from 'preact/hooks';
import { renderMarkdown } from '../lib/markdown';

export function MarkdownView({ source, class: className }: { source: string; class?: string }) {
  const html = useMemo(() => renderMarkdown(source), [source]);
  return (
    <div
      class={`markdown-body ${className ?? ''}`}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
