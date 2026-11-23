#!/usr/bin/env node
import fs from 'node:fs';

const prompt = fs.readFileSync(0, 'utf8').trim();
const cardPlan = prompt.includes('MEDIADESK_CARD_PROTOCOL');
const assistant = prompt.includes('MEDIADESK_ASSISTANT_PROTOCOL');

if (cardPlan) {
  process.stdout.write(JSON.stringify({
    kicker: '2026 · 11',
    page: '01 / 01',
    title: '把结论放到第一屏',
    body: [
      '读者不需要先看你如何抵达，才决定要不要继续读。',
      '先给出判断，再解释理由；卡片只保留一句值得记住的话。',
    ],
    footnote: '图注｜先写下来，再慢慢变好',
    source: '来源｜E2E 卡片测试',
    imagePrompt: '一张无文字的编辑式桌面照片，纸张、铅笔和晨光，克制的米色与深绿。',
  }));
  process.exit(0);
}

if (assistant) {
  const match = prompt.match(/assetId:\s*([A-Z0-9]+)/);
  const assetId = match?.[1] ?? '';
  const payload = {
    reply: '我读过了项目与素材，建议把这段内容改得更直接，并保留原有信息。',
    edits: assetId
      ? [
          {
            assetId,
            content:
              '# E2E 助手改稿\n\n这是项目助手通过覆盖模式写入的新正文。\n\n## 改动\n\n- 收紧表达\n- 保留原始事实\n\n自检：这是测试占位内容，无外部来源。',
            summary: '收紧正文并补充改动说明',
          },
        ]
      : [],
  };
  if (!assetId && (prompt.includes('文卡图') || prompt.includes('新建'))) {
    payload.reply = 'card/ 目录还是空的。我建议先建 3 张文卡图文案素材，你确认后再写入。';
    payload.newAssets = [
      {
        kind: 'card',
        title: '先写下来，再慢慢变好',
        content: '先写下来，再慢慢变好。\n\n创作不是一次到位，而是让想法先有一个可以修改的形状。',
        summary: '关于创作节奏的短句卡',
      },
      {
        kind: 'card',
        title: '素材不是收藏，是待使用的证据',
        content: '素材不是收藏，是待使用的证据。\n\n只有被引用、被改写、被放进作品，它才真正开始工作。',
        summary: '关于素材使用的短句卡',
      },
      {
        kind: 'card',
        title: '把结论放到第一屏',
        content: '把结论放到第一屏。\n\n读者不需要先看你如何抵达，才决定要不要继续读。',
        summary: '关于写作结构的短句卡',
      },
    ];
  }
  process.stdout.write(JSON.stringify(payload));
} else {
  process.stdout.write(
    '# E2E CLI 生成文章\n\n这是由 mock CLI 内核生成的内容。\n\n## 验证点\n\n- 文本生成走本地 CLI\n- 提示词快照仍会归档\n',
  );
}
