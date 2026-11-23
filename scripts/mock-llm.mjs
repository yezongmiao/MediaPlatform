import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const FIXTURE_IMAGE = fs.readFileSync(path.join(root, 'core/tests/fixtures/e2e-image.png'));
const FIXTURE_VIDEO = fs.readFileSync(path.join(root, 'core/tests/fixtures/clip.mp4'));

function sendJson(res, status, body) {
  res.writeHead(status, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(body));
}

export function startMockLlm(port = 4391) {
  const server = http.createServer((req, res) => {
    const url = req.url?.split('?')[0] ?? '';

    if (url === '/__health') {
      sendJson(res, 200, { ok: true });
      return;
    }

    if (req.method === 'GET' && url === '/mock-video.mp4') {
      res.writeHead(200, { 'Content-Type': 'video/mp4', 'Content-Length': FIXTURE_VIDEO.length });
      res.end(FIXTURE_VIDEO);
      return;
    }

    if (req.method === 'GET' && url === '/v1/videos/e2e-task-1') {
      sendJson(res, 200, {
        id: 'e2e-task-1',
        status: 'completed',
        video_url: `http://127.0.0.1:${port}/mock-video.mp4`,
      });
      return;
    }

    const chunks = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', () => {
      let payload = {};
      try {
        payload = JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}');
      } catch {
        payload = {};
      }

      if (req.method === 'POST' && url === '/v1/images/generations') {
        sendJson(res, 200, {
          data: [{ b64_json: FIXTURE_IMAGE.toString('base64'), revised_prompt: payload.prompt }],
          model: 'mock-image-e2e',
        });
        return;
      }

      if (req.method === 'POST' && url === '/v1/videos') {
        sendJson(res, 200, { id: 'e2e-task-1', status: 'queued' });
        return;
      }

      if (req.method !== 'POST' || url !== '/v1/chat/completions') {
        sendJson(res, 404, { error: { message: 'not found' } });
        return;
      }

      const auth = req.headers.authorization ?? '';
      if (auth === 'Bearer bad-key') {
        sendJson(res, 401, { error: { message: 'invalid api key' } });
        return;
      }

      const messages = Array.isArray(payload.messages) ? payload.messages : [];
      const user = String(messages.at(-1)?.content ?? '');
      let content =
        '# E2E Mock 生成文章\n\n这是 Playwright 端到端测试中由 mock 模型生成的内容。\n\n## 关键观点\n\n- 素材需要被看见，也需要被再次使用。\n- 创作系统的最小单位是一条可以被找回的想法。\n\n## 结尾\n\n把今天的一个念头写下来。\n\n自检：数据为测试占位，无外部来源。';
      if (user.includes('文卡图')) {
        content =
          '主张：创作是把想法变成可保存的东西\n补充：先写下来，再慢慢变好。\n\n主张：素材不是仓库，是下一次创作的起点\n补充：让文件可以被再次找到。';
      } else if (user.includes('插图')) {
        content =
          '## 方案一\n\n中文画面说明：晨光中的书桌，纸张与笔。\n\nEnglish prompt: warm morning light, editorial illustration, paper texture, muted vermilion, 3:4, delicate linework.\n\nNegative prompt: text, logo, photorealistic face, clutter.\n\n比例：3:4\n\n标题安全区：顶部留白。';
      } else if (user.includes('视频')) {
        content =
          '# E2E Mock 视频脚本\n\n| 镜号 | 时长 | 画面 | 口播 |\n|---|---|---|---|\n| 1 | 5s | 书桌特写 | 你也有过这样的时刻吗？ |\n';
      } else if (user.trim() === 'ping') {
        content = 'pong';
      }
      setTimeout(() => {
        sendJson(res, 200, {
          id: 'mock-e2e',
          model: 'mock-e2e-model',
          choices: [{ index: 0, message: { role: 'assistant', content }, finish_reason: 'stop' }],
          usage: { prompt_tokens: 101, completion_tokens: 202, total_tokens: 303 },
        });
      }, 220);
    });
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', () => resolve(server));
  });
}
