#!/usr/bin/env node
import fs from 'node:fs';

const prompt = fs.readFileSync(0, 'utf8').trim();
const cardPlan = prompt.includes('MEDIADESK_CARD_PROTOCOL');
const libraryAssistant = prompt.includes('MEDIADESK_LIBRARY_PROTOCOL');
const assistant = prompt.includes('MEDIADESK_ASSISTANT_PROTOCOL');

if (libraryAssistant) {
  const libraryMessage = prompt.split('# 新消息').pop() || '';
  if (libraryMessage.includes('删除')) {
    const skillId = (prompt.match(/### 《E2E 文卡方法》[\s\S]{0,240}?id: ([^\n]+)/) || [])[1];
    const presetId = (prompt.match(/### 《E2E 文卡预设》[\s\S]{0,240}?id: ([^\n]+)/) || [])[1];
    const actions = [];
    if (skillId) actions.push({ type: 'deleteSkill', id: skillId.trim() });
    if (presetId) actions.push({ type: 'deletePreset', id: presetId.trim() });
    process.stdout.write(JSON.stringify({
      reply: actions.length ? '已删除这组技能与预设。' : '没有找到可删除的测试技能。',
      actions,
    }));
    process.exit(0);
  }
  process.stdout.write(JSON.stringify({
    reply: '已创建一套用于文卡图的技能与预设，并直接写入技能库。',
    actions: [
      {
        type: 'createSkill',
        name: 'E2E 文卡方法',
        description: '把文章拆成短卡',
        appliesTo: ['article', 'card'],
        content: '# E2E 文卡方法\n\n一张卡片只保留一个观点。\n\n- 先给结论\n- 再解释理由',
      },
      {
        type: 'createPreset',
        name: 'E2E 文卡预设',
        description: '测试用文卡规范',
        kind: 'card',
        skillIds: [],
        content: '# E2E 文卡预设\n\n标题克制，正文不超过三段，来源可核查。',
      },
    ],
  }));
  process.exit(0);
}

if (cardPlan) {
  process.stdout.write(JSON.stringify({
    cards: [
      {
        kicker: '2026 · 11', page: '01 / 03', title: '把结论放到第一屏',
        body: ['读者不需要先看你如何抵达，才决定要不要继续读。', '先给出判断，再解释理由；卡片只保留一句值得记住的话。'],
        footnote: '图注｜先写下来，再慢慢变好', source: '来源｜E2E 卡片测试',
        imagePrompt: '一张无文字的编辑式桌面照片，纸张、铅笔和晨光，克制的米色与深绿。',
      },
      {
        kicker: '2026 · 11', page: '02 / 03', title: '素材不是收藏',
        body: ['素材不是收藏，是待使用的证据。', '只有被引用、被改写、被放进作品，它才真正开始工作。'],
        footnote: '图注｜让素材进入作品', source: '来源｜E2E 卡片测试',
        imagePrompt: '一张无文字的编辑式资料柜照片，暖米色与深绿，留白。',
      },
      {
        kicker: '2026 · 11', page: '03 / 03', title: '先完成，再完美',
        body: ['第一版不需要完美，它只需要存在。', '让想法先有一个可以修改的形状，再慢慢变好。'],
        footnote: '图注｜创作是长期主义', source: '来源｜E2E 卡片测试',
        imagePrompt: '一张无文字的编辑式工作台照片，散落纸张与铅笔，晨光。',
      },
    ],
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
