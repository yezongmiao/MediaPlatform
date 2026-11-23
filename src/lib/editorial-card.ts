import type { CardTheme } from './card-renderer';

export interface EditorialCardPlan {
  kicker: string;
  page: string;
  title: string;
  body: string[];
  footnote: string;
  source: string;
}

const WIDTH = 1080;
const HEIGHT = 1440;
const MARGIN = 84;
const SERIF = '"Songti SC", "STSong", "Noto Serif CJK SC", "Source Han Serif SC", Georgia, serif';
const SANS = '-apple-system, BlinkMacSystemFont, "PingFang SC", "Microsoft YaHei", sans-serif';

function wrapText(
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
): string[] {
  const lines: string[] = [];
  let line = '';
  for (const char of text) {
    const next = line + char;
    if (line && ctx.measureText(next).width > maxWidth) {
      lines.push(line);
      line = char === ' ' ? '' : char;
    } else {
      line = next;
    }
  }
  if (line) lines.push(line);
  return lines;
}

function fitTitle(
  ctx: CanvasRenderingContext2D,
  title: string,
  maxWidth: number,
  maxLines: number,
): { size: number; lines: string[] } {
  let size = 66;
  let lines: string[] = [];
  while (size >= 44) {
    ctx.font = `700 ${size}px ${SERIF}`;
    lines = wrapText(ctx, title, maxWidth);
    if (lines.length <= maxLines) break;
    size -= 3;
  }
  if (lines.length > maxLines) {
    lines = lines.slice(0, maxLines);
    const last = lines[lines.length - 1] ?? '';
    lines[lines.length - 1] = `${last.slice(0, Math.max(1, last.length - 1))}…`;
  }
  return { size, lines };
}

function drawImageCover(
  ctx: CanvasRenderingContext2D,
  image: HTMLImageElement,
  x: number,
  y: number,
  width: number,
  height: number,
): void {
  const scale = Math.max(width / image.width, height / image.height);
  const sourceWidth = width / scale;
  const sourceHeight = height / scale;
  const sx = (image.width - sourceWidth) / 2;
  const sy = (image.height - sourceHeight) / 2;
  ctx.drawImage(image, sx, sy, sourceWidth, sourceHeight, x, y, width, height);
}

export function drawEditorialCard(
  canvas: HTMLCanvasElement,
  options: { plan: EditorialCardPlan; theme: CardTheme; image?: HTMLImageElement | null },
): void {
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const { plan, theme, image } = options;

  const gradient = ctx.createLinearGradient(0, 0, WIDTH, HEIGHT);
  gradient.addColorStop(0, theme.colors[0]);
  gradient.addColorStop(1, theme.colors[1]);
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, WIDTH, HEIGHT);

  // Light paper grain, deterministic so the same plan exports consistently.
  ctx.fillStyle = theme.grain;
  let seed = 20261123;
  const random = () => {
    seed = (seed * 1664525 + 1013904223) % 4294967296;
    return seed / 4294967296;
  };
  for (let i = 0; i < 900; i += 1) {
    ctx.beginPath();
    ctx.arc(random() * WIDTH, random() * HEIGHT, random() * 1.5 + 0.3, 0, Math.PI * 2);
    ctx.fill();
  }

  const contentWidth = WIDTH - MARGIN * 2;

  // Kicker + page
  ctx.fillStyle = theme.ink;
  const kicker = plan.kicker.trim();
  let kickerSize = 92;
  ctx.font = `700 ${kickerSize}px ${SERIF}`;
  while (kickerSize > 52 && ctx.measureText(kicker).width > contentWidth - 180) {
    kickerSize -= 4;
    ctx.font = `700 ${kickerSize}px ${SERIF}`;
  }
  ctx.textAlign = 'left';
  ctx.textBaseline = 'alphabetic';
  if (kicker) ctx.fillText(kicker, MARGIN, 142);

  if (plan.page.trim()) {
    ctx.fillStyle = theme.muted;
    ctx.font = `500 24px ${SANS}`;
    ctx.textAlign = 'right';
    ctx.fillText(plan.page.trim(), WIDTH - MARGIN, 120);
  }

  // Red rule + thin editorial line
  ctx.fillStyle = theme.accent;
  ctx.fillRect(MARGIN, 184, 116, 5);
  ctx.fillStyle = theme.frame;
  ctx.fillRect(MARGIN + 140, 186, contentWidth - 140, 1);

  // Title
  const title = fitTitle(ctx, plan.title, contentWidth, 2);
  ctx.fillStyle = theme.ink;
  ctx.textAlign = 'left';
  ctx.font = `700 ${title.size}px ${SERIF}`;
  const titleTop = 260;
  title.lines.forEach((line, index) => {
    ctx.fillText(line, MARGIN, titleTop + index * Math.round(title.size * 1.32));
  });
  const titleBottom = titleTop + (title.lines.length - 1) * Math.round(title.size * 1.32);

  // Image area or a larger text field when there is no illustration.
  let bodyTop = titleBottom + 70;
  if (image) {
    const imageTop = titleBottom + 58;
    const imageHeight = 600;
    ctx.save();
    ctx.fillStyle = 'rgba(0, 0, 0, 0.035)';
    ctx.fillRect(MARGIN + 4, imageTop + 5, contentWidth, imageHeight);
    drawImageCover(ctx, image, MARGIN, imageTop, contentWidth, imageHeight);
    ctx.strokeStyle = theme.frame;
    ctx.lineWidth = 1.2;
    ctx.strokeRect(MARGIN, imageTop, contentWidth, imageHeight);
    ctx.restore();
    bodyTop = imageTop + imageHeight + 66;
  }

  // Body — shrink until it fits above the footer. In text-only mode, use a
  // larger measure and center the block so the page does not look like an
  // unfinished image layout.
  const body = plan.body.filter((line) => line.trim());
  const maxBodyBottom = 1270;
  let bodySize = image ? 33 : 38;
  let bodyLines: string[] = [];
  let bodyLineHeight = Math.round(bodySize * 1.72);
  let bodyGap = Math.round(bodySize * 0.56);
  while (bodySize >= 23) {
    ctx.font = `400 ${bodySize}px ${SANS}`;
    bodyLineHeight = Math.round(bodySize * 1.72);
    bodyGap = Math.round(bodySize * 0.56);
    bodyLines = [];
    for (const paragraph of body) {
      bodyLines.push(...wrapText(ctx, paragraph, contentWidth));
    }
    const blockHeight =
      bodyLines.length * bodyLineHeight + Math.max(0, body.length - 1) * bodyGap;
    if (bodyTop + blockHeight <= maxBodyBottom) break;
    bodySize -= 2;
  }
  const blockHeight =
    bodyLines.length * bodyLineHeight + Math.max(0, body.length - 1) * bodyGap;
  const available = maxBodyBottom - bodyTop;
  let cursorY =
    bodyTop +
    (image ? 0 : Math.max(0, (available - blockHeight) / 2)) +
    bodySize;
  ctx.fillStyle = theme.ink;
  ctx.textAlign = 'left';
  ctx.font = `400 ${bodySize}px ${SANS}`;
  let lineIndex = 0;
  for (const paragraph of body) {
    const wrapped = wrapText(ctx, paragraph, contentWidth);
    for (const line of wrapped) {
      if (lineIndex >= bodyLines.length) break;
      if (cursorY > maxBodyBottom) break;
      ctx.fillText(line, MARGIN, cursorY);
      cursorY += bodyLineHeight;
      lineIndex += 1;
    }
    cursorY += bodyGap;
  }

  // Footer
  ctx.fillStyle = theme.frame;
  ctx.fillRect(MARGIN, 1304, contentWidth, 1);
  ctx.fillStyle = theme.muted;
  ctx.font = `400 20px ${SANS}`;
  ctx.textAlign = 'left';
  if (plan.footnote.trim()) {
    ctx.fillText(wrapText(ctx, plan.footnote.trim(), contentWidth * 0.58)[0] ?? '', MARGIN, 1352);
  }
  if (plan.source.trim()) {
    ctx.textAlign = 'right';
    ctx.fillText(wrapText(ctx, plan.source.trim(), contentWidth * 0.38)[0] ?? '', WIDTH - MARGIN, 1352);
  }

  // A small editorial mark keeps the bottom corner from feeling unfinished.
  ctx.fillStyle = theme.accent;
  ctx.fillRect(MARGIN, 1378, 34, 3);
  ctx.fillStyle = theme.muted;
  ctx.font = `500 15px ${SANS}`;
  ctx.textAlign = 'left';
  ctx.fillText('IDEAS INTO WORKS', MARGIN + 48, 1383);
}
