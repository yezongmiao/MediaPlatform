export type CardRatio = '3:4' | '1:1' | '4:5';

export const CARD_RATIOS: Record<CardRatio, { width: number; height: number; label: string }> = {
  '3:4': { width: 1080, height: 1440, label: '3:4 竖版' },
  '1:1': { width: 1080, height: 1080, label: '1:1 方图' },
  '4:5': { width: 1080, height: 1350, label: '4:5 图文' },
};

export interface CardTheme {
  id: string;
  name: string;
  colors: [string, string];
  ink: string;
  muted: string;
  accent: string;
  frame: string;
  grain: string;
}

export const CARD_THEMES: CardTheme[] = [
  {
    id: 'rice',
    name: '宣纸',
    colors: ['#f3ecdd', '#e9dfcb'],
    ink: '#252019',
    muted: '#746a5b',
    accent: '#b6452d',
    frame: 'rgba(92, 70, 44, 0.22)',
    grain: 'rgba(80, 60, 35, 0.045)',
  },
  {
    id: 'cinnabar',
    name: '朱砂',
    colors: ['#b8492f', '#91331f'],
    ink: '#fbefdd',
    muted: '#eccdb0',
    accent: '#f0c27b',
    frame: 'rgba(251, 239, 221, 0.32)',
    grain: 'rgba(255, 255, 255, 0.035)',
  },
  {
    id: 'pine',
    name: '松烟',
    colors: ['#21322d', '#14231f'],
    ink: '#efe7d6',
    muted: '#a9b8ad',
    accent: '#d3a25e',
    frame: 'rgba(239, 231, 214, 0.22)',
    grain: 'rgba(255, 255, 255, 0.03)',
  },
  {
    id: 'mist',
    name: '雾蓝',
    colors: ['#dfe7e5', '#c8d6d3'],
    ink: '#20312d',
    muted: '#5f716c',
    accent: '#b0543a',
    frame: 'rgba(32, 49, 45, 0.2)',
    grain: 'rgba(30, 60, 50, 0.035)',
  },
  {
    id: 'wheat',
    name: '麦田',
    colors: ['#ead4af', '#d9b980'],
    ink: '#2d2417',
    muted: '#75603f',
    accent: '#7c3d2a',
    frame: 'rgba(64, 45, 20, 0.24)',
    grain: 'rgba(90, 60, 20, 0.04)',
  },
];

export const SERIF_FONT =
  '"Songti SC", "STSong", "Noto Serif CJK SC", "Source Han Serif SC", "SimSun", Georgia, serif';
export const SANS_FONT =
  '-apple-system, BlinkMacSystemFont, "SF Pro Text", "PingFang SC", "Microsoft YaHei", sans-serif';

export interface DrawCardOptions {
  text: string;
  subtext?: string;
  theme: CardTheme;
  ratio: CardRatio;
  topLabel?: string;
}

function wrapText(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string[] {
  const lines: string[] = [];
  for (const paragraph of text.split('\n')) {
    if (!paragraph.trim()) {
      if (lines.length) lines.push('');
      continue;
    }
    let line = '';
    for (const char of paragraph.trim()) {
      const next = line + char;
      if (line && ctx.measureText(next).width > maxWidth) {
        lines.push(line);
        line = char === ' ' ? '' : char;
      } else {
        line = next;
      }
    }
    if (line) lines.push(line);
  }
  return lines;
}

function fitMainText(
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
  maxHeight: number,
  maxLines: number,
) {
  let size = 96;
  let lines: string[] = [];
  while (size >= 50) {
    ctx.font = `600 ${size}px ${SERIF_FONT}`;
    lines = wrapText(ctx, text, maxWidth);
    const height = lines.length * size * 1.34;
    if (lines.length <= maxLines && height <= maxHeight) break;
    size -= 4;
  }
  if (lines.length > maxLines) {
    lines = lines.slice(0, maxLines);
    const last = lines[maxLines - 1] ?? '';
    lines[maxLines - 1] = `${last.slice(0, Math.max(1, last.length - 1))}…`;
  }
  return { size, lines };
}

function seededRandom(seed: number) {
  let state = seed || 1;
  return () => {
    state = (state * 1664525 + 1013904223) % 4294967296;
    return state / 4294967296;
  };
}

export function drawCard(canvas: HTMLCanvasElement, options: DrawCardOptions): void {
  const { width, height } = CARD_RATIOS[options.ratio];
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const theme = options.theme;

  const gradient = ctx.createLinearGradient(0, 0, width, height);
  gradient.addColorStop(0, theme.colors[0]);
  gradient.addColorStop(1, theme.colors[1]);
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, width, height);

  const rand = seededRandom(options.text.length * 97 + options.ratio.length * 31 + 7);
  ctx.fillStyle = theme.grain;
  for (let i = 0; i < 900; i += 1) {
    const x = rand() * width;
    const y = rand() * height;
    const r = rand() * 1.6 + 0.3;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();
  }

  const margin = 58;
  ctx.strokeStyle = theme.frame;
  ctx.lineWidth = 1.2;
  ctx.strokeRect(margin, margin, width - margin * 2, height - margin * 2);

  ctx.fillStyle = theme.accent;
  ctx.fillRect(margin + 34, margin + 44, 42, 3);

  ctx.fillStyle = theme.muted;
  ctx.font = `500 21px ${SANS_FONT}`;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'middle';
  ctx.fillText(options.topLabel ?? 'IDEAS INTO WORKS', margin + 34, margin + 72);

  ctx.textAlign = 'right';
  ctx.fillText(theme.name, width - margin - 34, margin + 72);

  const contentWidth = width - (margin + 86) * 2;
  const contentLeft = margin + 86;
  const maxMainHeight = height - 560;
  const mainText = options.text.trim() || '写下一句\n值得被记住的话。';
  const { size, lines } = fitMainText(ctx, mainText, contentWidth, maxMainHeight, 6);
  ctx.textAlign = 'center';
  ctx.fillStyle = theme.ink;
  ctx.font = `600 ${size}px ${SERIF_FONT}`;
  const lineHeight = size * 1.36;
  const blockHeight = lines.length * lineHeight;
  const centerY = height * 0.48;
  let y = centerY - blockHeight / 2 + lineHeight / 2;
  for (const line of lines) {
    ctx.fillText(line, width / 2, y);
    y += lineHeight;
  }

  if (options.subtext?.trim()) {
    ctx.fillStyle = theme.muted;
    ctx.font = `400 30px ${SANS_FONT}`;
    const subLines = wrapText(ctx, options.subtext.trim(), contentWidth - 60).slice(0, 3);
    let subY = y + 38;
    for (const line of subLines) {
      ctx.fillText(line, width / 2, subY);
      subY += 44;
    }
  }

  const footY = height - margin - 62;
  ctx.strokeStyle = theme.frame;
  ctx.beginPath();
  ctx.moveTo(width / 2 - 34, footY - 28);
  ctx.lineTo(width / 2 + 34, footY - 28);
  ctx.stroke();
  ctx.fillStyle = theme.muted;
  ctx.font = `500 20px ${SANS_FONT}`;
  ctx.textAlign = 'center';
  ctx.fillText('媒材工坊 · MEDIA DESK', width / 2, footY + 2);
}

export function extractCardText(input: string): string {
  const cleaned = input
    .replace(/```[\s\S]*?```/g, '\n')
    .split('\n')
    .map((line) =>
      line
        .replace(/^#+\s*/, '')
        .replace(/^[-*>\d.\s]+/, '')
        .replace(/\*\*/g, '')
        .trim(),
    )
    .filter(Boolean);
  const preferred = cleaned.find((line) => /^主张[：:]/.test(line));
  const candidate = preferred?.replace(/^主张[：:]\s*/, '') ?? cleaned[0] ?? '';
  return candidate.slice(0, 80);
}

export function extractCardSubtext(input: string): string {
  const lines = input
    .split('\n')
    .map((line) => line.replace(/\*\*/g, '').trim())
    .filter(Boolean);
  const line = lines.find((item) => /^补充[：:]/.test(item));
  return line?.replace(/^补充[：:]\s*/, '').slice(0, 80) ?? '';
}
