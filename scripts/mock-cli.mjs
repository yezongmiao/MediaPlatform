#!/usr/bin/env node
import fs from 'node:fs';

const prompt = fs.readFileSync(0, 'utf8').trim();
const assistant = prompt.includes('MEDIADESK_ASSISTANT_PROTOCOL');

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
  process.stdout.write(JSON.stringify(payload));
} else {
  process.stdout.write(
    '# E2E CLI 生成文章\n\n这是由 mock CLI 内核生成的内容。\n\n## 验证点\n\n- 文本生成走本地 CLI\n- 提示词快照仍会归档\n',
  );
}
