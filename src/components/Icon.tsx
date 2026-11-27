import type { JSX } from 'preact';

export type IconName =
  | 'spark'
  | 'inbox'
  | 'folder'
  | 'layers'
  | 'settings'
  | 'search'
  | 'plus'
  | 'close'
  | 'arrow-left'
  | 'arrow-right'
  | 'external'
  | 'code'
  | 'wand'
  | 'upload'
  | 'image'
  | 'video'
  | 'file-text'
  | 'cards'
  | 'trash'
  | 'edit'
  | 'check'
  | 'copy'
  | 'download'
  | 'menu'
  | 'sun'
  | 'moon'
  | 'monitor'
  | 'refresh'
  | 'folder-open'
  | 'clock'
  | 'tag'
  | 'more'
  | 'alert'
  | 'bulb'
  | 'eye'
  | 'eye-off'
  | 'play'
  | 'stop'
  | 'book'
  | 'sliders'
  | 'chevron-down'
  | 'chevron-right'
  | 'command'
  | 'archive'
  | 'grid'
  | 'pen';

const PATHS: Record<IconName, JSX.Element> = {
  spark: (
    <>
      <path d="M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9L12 3z" />
      <path d="M19 16l.8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8L19 16z" />
    </>
  ),
  inbox: (
    <>
      <path d="M4 5.5h16v13H4z" />
      <path d="M4 13h4l1.5 2h5L16 13h4" />
    </>
  ),
  folder: (
    <path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" />
  ),
  layers: (
    <>
      <path d="M12 3l9 5-9 5-9-5 9-5z" />
      <path d="M3 12l9 5 9-5" />
      <path d="M3 16l9 5 9-5" />
    </>
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 00.3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 00-1.9-.3 1.7 1.7 0 00-1 1.5V21h-4v-.1a1.7 1.7 0 00-1-1.5 1.7 1.7 0 00-1.9.3l-.1.1-2.8-2.8.1-.1A1.7 1.7 0 004.6 15a1.7 1.7 0 00-1.5-1H3v-4h.1a1.7 1.7 0 001.5-1 1.7 1.7 0 00-.3-1.9l-.1-.1 2.8-2.8.1.1a1.7 1.7 0 001.9.3 1.7 1.7 0 001-1.5V3h4v.1a1.7 1.7 0 001 1.5 1.7 1.7 0 001.9-.3l.1-.1 2.8 2.8-.1.1a1.7 1.7 0 00-.3 1.9 1.7 1.7 0 001.5 1H21v4h-.1a1.7 1.7 0 00-1.5 1z" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-4-4" />
    </>
  ),
  plus: <path d="M12 5v14M5 12h14" />,
  close: <path d="M6 6l12 12M18 6L6 18" />,
  'arrow-left': <path d="M19 12H5M11 18l-6-6 6-6" />,
  'arrow-right': <path d="M5 12h14M13 6l6 6-6 6" />,
  external: (
    <>
      <path d="M14 4h6v6" />
      <path d="M20 4l-9 9" />
      <path d="M19 14v5a1 1 0 01-1 1H5a1 1 0 01-1-1V6a1 1 0 011-1h5" />
    </>
  ),
  code: (
    <>
      <path d="M8 7l-5 5 5 5M16 7l5 5-5 5" />
      <path d="M14 4l-4 16" />
    </>
  ),
  wand: (
    <>
      <path d="M4 20L16 8" />
      <path d="M15 4l1 2.2L18 7l-2 .8L15 10l-1-2.2L12 7l2-.8L15 4z" />
      <path d="M19 12l.8 1.7L21.5 14l-1.7.7L19 16l-.8-1.3L16.5 14l1.7-.3L19 12z" />
      <path d="M7 4l.6 1.4L9 6l-1.4.6L7 8l-.6-1.4L5 6l1.4-.6L7 4z" />
    </>
  ),
  upload: (
    <>
      <path d="M12 16V4" />
      <path d="M7 9l5-5 5 5" />
      <path d="M4 15v4a2 2 0 002 2h12a2 2 0 002-2v-4" />
    </>
  ),
  image: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <circle cx="8.5" cy="9" r="1.5" />
      <path d="M21 15l-5-5L5 20" />
    </>
  ),
  video: (
    <>
      <rect x="3" y="5" width="13" height="14" rx="2" />
      <path d="M16 10l5-3v10l-5-3" />
    </>
  ),
  'file-text': (
    <>
      <path d="M14 3H6a1 1 0 00-1 1v16a1 1 0 001 1h12a1 1 0 001-1V8z" />
      <path d="M14 3v5h5" />
      <path d="M8 13h8M8 17h6" />
    </>
  ),
  cards: (
    <>
      <rect x="6" y="3" width="12" height="18" rx="1.5" />
      <path d="M9 8h6M9 12h6M9 16h3" />
    </>
  ),
  trash: (
    <>
      <path d="M4 7h16" />
      <path d="M9 7V4h6v3" />
      <path d="M6 7l1 13h10l1-13" />
      <path d="M10 11v5M14 11v5" />
    </>
  ),
  edit: (
    <>
      <path d="M4 20h4l11-11-4-4L4 16v4z" />
      <path d="M13.5 6.5l4 4" />
    </>
  ),
  pen: (
    <>
      <path d="M15.5 4.5l4 4L8 20H4v-4L15.5 4.5z" />
      <path d="M13 7l4 4" />
    </>
  ),
  check: <path d="M4 12l5 5L20 6" />,
  copy: (
    <>
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M5 15H4a1 1 0 01-1-1V4a1 1 0 011-1h10a1 1 0 011 1v1" />
    </>
  ),
  download: (
    <>
      <path d="M12 4v12" />
      <path d="M7 11l5 5 5-5" />
      <path d="M4 20h16" />
    </>
  ),
  menu: <path d="M4 6h16M4 12h16M4 18h16" />,
  sun: (
    <>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
    </>
  ),
  moon: <path d="M20 15.5A8.5 8.5 0 019.5 5 8.5 8.5 0 1020 15.5z" />,
  monitor: (
    <>
      <rect x="3" y="4" width="18" height="12" rx="2" />
      <path d="M8 20h8M12 16v4" />
    </>
  ),
  refresh: (
    <>
      <path d="M20 11a8 8 0 10-2.3 6.3" />
      <path d="M20 4v7h-7" />
    </>
  ),
  'folder-open': (
    <>
      <path d="M3 8V6a1 1 0 011-1h4l2 2h8a1 1 0 011 1v2" />
      <path d="M3 10h18l-2 9H5l-2-9z" />
    </>
  ),
  clock: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3 2" />
    </>
  ),
  tag: (
    <>
      <path d="M20 13l-7 7-9-9V4h7l9 9z" />
      <circle cx="8" cy="8" r="1.2" />
    </>
  ),
  more: (
    <>
      <circle cx="5" cy="12" r="1" fill="currentColor" />
      <circle cx="12" cy="12" r="1" fill="currentColor" />
      <circle cx="19" cy="12" r="1" fill="currentColor" />
    </>
  ),
  alert: (
    <>
      <path d="M12 3l9 17H3L12 3z" />
      <path d="M12 9v5M12 17.5v.01" />
    </>
  ),
  bulb: (
    <>
      <path d="M9 18h6M10 21h4" />
      <path d="M12 3a6 6 0 00-3.5 10.9c.7.5 1 1.2 1 2.1h5c0-.9.3-1.6 1-2.1A6 6 0 0012 3z" />
    </>
  ),
  eye: (
    <>
      <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12z" />
      <circle cx="12" cy="12" r="2.5" />
    </>
  ),
  'eye-off': (
    <>
      <path d="M3 3l18 18" />
      <path d="M10.6 6.2A9.7 9.7 0 0112 6c6.5 0 10 6 10 6a17 17 0 01-3.3 3.8M6.1 6.9A17.6 17.6 0 002 12s3.5 6 10 6a9.5 9.5 0 004-.9" />
      <path d="M9.9 9.9a3 3 0 004.2 4.2" />
    </>
  ),
  play: <path d="M8 5l11 7-11 7V5z" />,
  stop: <rect x="6.5" y="6.5" width="11" height="11" rx="1.5" />,
  book: (
    <>
      <path d="M4 5a2 2 0 012-2h13v18H6a2 2 0 01-2-2V5z" />
      <path d="M4 17h15" />
    </>
  ),
  sliders: (
    <>
      <path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0" />
      <circle cx="16" cy="6" r="2" />
      <circle cx="10" cy="12" r="2" />
      <circle cx="18" cy="18" r="2" />
    </>
  ),
  'chevron-down': <path d="M6 9l6 6 6-6" />,
  'chevron-right': <path d="M9 6l6 6-6 6" />,
  command: <path d="M18 3a3 3 0 00-3 3v12a3 3 0 103-3H6a3 3 0 103 3V6a3 3 0 10-3 3h12a3 3 0 000-6z" />,
  archive: (
    <>
      <rect x="3" y="4" width="18" height="4" rx="1" />
      <path d="M5 8v11a1 1 0 001 1h12a1 1 0 001-1V8M10 12h4" />
    </>
  ),
  grid: (
    <>
      <rect x="4" y="4" width="6" height="6" rx="1" />
      <rect x="14" y="4" width="6" height="6" rx="1" />
      <rect x="4" y="14" width="6" height="6" rx="1" />
      <rect x="14" y="14" width="6" height="6" rx="1" />
    </>
  ),
};

interface IconProps {
  name: IconName;
  size?: number;
  class?: string;
  strokeWidth?: number;
}

export function Icon({ name, size = 18, class: className, strokeWidth = 1.7 }: IconProps) {
  return (
    <svg
      class={`icon ${className ?? ''}`}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width={strokeWidth}
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      {PATHS[name]}
    </svg>
  );
}
