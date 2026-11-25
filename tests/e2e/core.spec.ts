import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const WORKSPACE = path.join(ROOT, 'artifacts/e2e-workspace');
const CODE_LOG = path.join(ROOT, 'artifacts/fake-code.log');
const SHOTS = path.join(ROOT, 'artifacts/e2e');

async function dismissToasts(page: import('@playwright/test').Page) {
  const closers = page.locator('.toast-close');
  while ((await closers.count()) > 0) await closers.first().click();
}
const MOCK_URL = 'http://127.0.0.1:4391/v1';

function projectDir(dirName: string): string {
  return path.join(WORKSPACE, 'projects', dirName);
}

function walk(root: string): string[] {
  if (!fs.existsSync(root)) return [];
  const out: string[] = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const full = path.join(root, entry.name);
    if (entry.isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
}

test('creator workflow: capture → project → create → generate → VSCode → upload → skills → search', async ({
  page,
  request,
}) => {
  const consoleErrors: string[] = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') consoleErrors.push(msg.text());
  });
  page.on('pageerror', (err) => consoleErrors.push(err.message));

  await page.goto('/');
  await expect(page.locator('.dashboard-page')).toBeVisible();
  await expect(page.getByRole('heading', { level: 1 })).toContainText(/好|夜深了/);
  await page.screenshot({ path: path.join(SHOTS, '01-dashboard.png'), fullPage: true });

  // 1. Quick capture
  await page.locator('.quick-capture-trigger').click();
  const capture = page.getByRole('dialog');
  await capture.getByPlaceholder('给这个想法一个名字').fill('E2E 灵感：构建个人素材系统');
  await capture
    .getByPlaceholder(/此刻在想什么/)
    .fill('我总是在不同平台之间丢失素材。\n\n也许需要一个本地优先、能被 AI 读取的素材系统。');
  const tagInput = capture.getByPlaceholder('输入标签后按回车');
  await tagInput.fill('E2E');
  await tagInput.press('Enter');
  await capture.getByRole('button', { name: '收进灵感箱' }).click();
  await expect(page).toHaveURL(/\/ideas$/);
  await expect(page.locator('.idea-card')).toContainText('E2E 灵感：构建个人素材系统');
  await page.screenshot({ path: path.join(SHOTS, '02-ideas.png'), fullPage: true });

  // 2. Promote idea to project
  const ideaCard = page.locator('.idea-card').filter({ hasText: 'E2E 灵感：构建个人素材系统' });
  await ideaCard.getByRole('button', { name: /立项/ }).click();
  const createProject = page.getByRole('dialog');
  await createProject
    .getByPlaceholder('例如：AI 时代的个人创作方法论')
    .fill('E2E 项目：长期创作系统');
  await createProject
    .getByPlaceholder('这条内容想解决什么问题？给谁看？')
    .fill('验证从灵感、素材到外部 AI 协作的完整工作流。');
  await createProject.getByRole('button', { name: '建立项目' }).click();
  await expect(page).toHaveURL(/\/projects\/[A-Z0-9]+/);
  await expect(page.getByRole('heading', { name: 'E2E 项目：长期创作系统', level: 1 })).toBeVisible();
  const projectId = page.url().split('/').pop()!;
  const projectResponse = await request.get(`/api/projects/${projectId}`);
  expect(projectResponse.ok()).toBeTruthy();
  const project = await projectResponse.json();
  const dirName = project.dirName as string;
  await page.screenshot({ path: path.join(SHOTS, '03-project.png'), fullPage: true });

  // 3. Create a manual article asset
  await page.locator('.project-tab').filter({ hasText: '文章' }).click();
  await expect(page.getByRole('button', { name: '新建文章' })).toBeVisible();
  await page.getByRole('button', { name: '新建文章' }).click();
  const assetEditor = page.getByRole('dialog');
  await assetEditor.locator('.asset-side input.input').first().fill('E2E 手动文章');
  await assetEditor
    .locator('.asset-textarea')
    .fill('# E2E 手动文章\n\n这是手动创建的正文。\n\n- 第一点\n- 第二点');
  await assetEditor.getByRole('button', { name: '保存', exact: true }).click();
  await expect(assetEditor).not.toBeVisible();
  await expect(page.locator('.asset-card').filter({ hasText: 'E2E 手动文章' })).toBeVisible();

  // 4. Configure the mock LLM through the real settings UI
  await page.goto('/settings');
  const llmCard = page.locator('.settings-card').filter({ hasText: '大模型接入' });
  const llmInputs = llmCard.locator('.settings-form-grid').first().locator('input.input');
  await llmInputs.nth(0).fill(MOCK_URL);
  await llmInputs.nth(1).fill('mock-e2e-model');
  await llmCard.getByPlaceholder('sk-…').fill('good-key');
  await llmCard.getByRole('button', { name: '保存设置' }).click();
  await expect(page.locator('.toast-host')).toContainText('大模型设置已保存');
  await llmCard.getByRole('button', { name: '测试连接' }).click();
  await expect(llmCard.locator('.test-result')).toContainText('连接成功');
  const config = await (await request.get('/api/config')).json();
  expect(config.llm.hasApiKey).toBe(true);
  expect(config.llm.apiKey).toBeUndefined();
  const diskConfig = JSON.parse(fs.readFileSync(path.join(WORKSPACE, '.mediadesk/config.json'), 'utf8'));
  expect(diskConfig.llm.apiKey).toBe('good-key');

  // 5. Generate with the mock model, preview prompt first, then save
  await page.goto(`/projects/${projectId}`);
  await page.getByRole('button', { name: '生成素材' }).click();
  const generate = page.getByRole('dialog');
  await generate.locator('textarea.generate-instruction').fill('写一篇关于个人素材系统的短文');
  await generate.locator('.advanced-toggle').click();
  await generate.getByRole('button', { name: '预览', exact: true }).click();
  await expect(generate.locator('.prompt-preview pre')).toContainText('本次任务');
  await generate.getByRole('button', { name: '生成', exact: true }).click();
  await expect(generate.locator('.result-title-input')).toHaveValue('E2E Mock 生成文章', {
    timeout: 30_000,
  });
  await expect(generate.locator('.result-body')).toContainText('Playwright 端到端测试');
  await page.screenshot({ path: path.join(SHOTS, '04-generate-result.png'), fullPage: true });
  await generate.getByRole('button', { name: '保存为素材' }).click();
  await expect(page.getByRole('dialog').locator('.asset-editor')).toBeVisible();
  await page.getByRole('dialog').getByRole('button', { name: '取消' }).click();
  await expect(page.locator('.asset-card').filter({ hasText: 'E2E Mock 生成文章' })).toBeVisible();

  const snapshots = walk(path.join(projectDir(dirName), 'prompts')).filter((file) =>
    file.endsWith('.md'),
  );
  expect(snapshots.some((file) => !file.endsWith('README.md'))).toBeTruthy();

  // 6. Open in VSCode (fake CLI) and verify generated context on disk
  await page.getByRole('button', { name: '在 VSCode 打开' }).click();
  await expect(page.locator('.toast-host')).toContainText('已生成 AI 上下文文件并打开 VS Code');
  for (const rel of [
    'AGENTS.md',
    'CLAUDE.md',
    'README.md',
    '.vscode/settings.json',
    '.vscode/tasks.json',
    '.mediadesk/context.json',
  ]) {
    expect(fs.existsSync(path.join(projectDir(dirName), rel)), rel).toBeTruthy();
  }
  expect(walk(path.join(projectDir(dirName), 'skills')).length).toBeGreaterThan(0);
  expect(walk(path.join(projectDir(dirName), 'presets')).length).toBe(5);
  await expect
    .poll(() => fs.readFileSync(CODE_LOG, 'utf8'), { timeout: 5000 })
    .toContain(dirName);

  // 7. Upload an illustration and edit its metadata
  await page.locator('.project-tab').filter({ hasText: '插图' }).click();
  const [chooser] = await Promise.all([
    page.waitForEvent('filechooser'),
    page.getByRole('button', { name: '上传图片' }).click(),
  ]);
  await chooser.setFiles(path.join(ROOT, 'core/tests/fixtures/test.png'));
  await expect(page.locator('.toast-host')).toContainText('已上传 1 个插图素材');
  const imageCard = page.locator('.asset-card').filter({ hasText: 'test' });
  await expect(imageCard).toBeVisible();
  await imageCard.click();
  const imageEditor = page.getByRole('dialog');
  await imageEditor.locator('.asset-side input.input').first().fill('E2E 插图');
  await imageEditor.getByRole('button', { name: '保存', exact: true }).click();
  await expect(imageEditor).not.toBeVisible();
  const editedCard = page.locator('.asset-card').filter({ hasText: 'E2E 插图' });
  await expect(editedCard).toBeVisible();
  await editedCard.hover();
  await editedCard.locator('.asset-zoom').click();
  await expect(page.locator('.lightbox')).toBeVisible();
  await page.screenshot({ path: path.join(SHOTS, '05-lightbox.png') });
  await page.locator('.lightbox-close').click();
  await expect(page.locator('.lightbox')).not.toBeVisible();

  // 8. Project settings: status + tag
  await page.getByRole('button', { name: '项目设置' }).click();
  const settings = page.getByRole('dialog');
  await settings.locator('select.select').first().selectOption('done');
  const projectTagInput = settings.locator('.tag-input input');
  await projectTagInput.fill('E2E');
  await projectTagInput.press('Enter');
  await settings.getByRole('button', { name: '保存设置' }).click();
  await expect(page.locator('.project-title-row .badge')).toHaveText('已完成');

  // 10. Skill library: create a skill through the UI
  await page.goto('/library');
  await page.getByRole('button', { name: '新建技能' }).click();
  const skillEditor = page.getByRole('dialog');
  await skillEditor.locator('.form-stack input.input').nth(0).fill('E2E 技能：标题实验');
  await skillEditor.locator('.form-stack input.input').nth(1).fill('快速生成 6 个不同动机的标题');
  await skillEditor.locator('.chip-check').filter({ hasText: '文章' }).click();
  await skillEditor
    .locator('.editor-textarea')
    .fill('# 标题实验\n\n一次给出 6 个不同动机的标题，不要同义改写。');
  await skillEditor.getByRole('button', { name: '保存技能' }).click();
  await expect(page.locator('.skill-card').filter({ hasText: 'E2E 技能：标题实验' })).toBeVisible();
  await page.screenshot({ path: path.join(SHOTS, '07-library.png'), fullPage: true });

  // 11. Command palette global search
  await page.goto(`/projects/${projectId}`);
  await page.locator('.command-trigger').click();
  const palette = page.getByRole('dialog');
  await palette.getByPlaceholder(/搜索想法、项目、素材/).fill('E2E');
  const paletteProject = palette.locator('.palette-item').filter({ hasText: 'E2E 项目：长期创作系统' }).first();
  await expect(paletteProject).toBeVisible();
  await page.screenshot({ path: path.join(SHOTS, '08-command-palette.png') });
  await paletteProject.click();
  await expect(page).toHaveURL(new RegExp(`/projects/${projectId}$`));

  // 12. Delete an article asset and confirm trash
  await page.locator('.project-tab').filter({ hasText: '文章' }).click();
  await page.locator('.asset-card').filter({ hasText: 'E2E 手动文章' }).click();
  const cardEditor = page.getByRole('dialog');
  await cardEditor.getByRole('button', { name: '删除', exact: true }).click();
  const confirm = page.getByRole('dialog').filter({ hasText: '删除这个素材？' });
  await expect(confirm).toBeVisible();
  // Escape closes only the top-most dialog, leaving the editor open.
  await page.keyboard.press('Escape');
  await expect(confirm).not.toBeVisible();
  await expect(cardEditor).toBeVisible();
  await cardEditor.getByRole('button', { name: '删除', exact: true }).click();
  await confirm.getByRole('button', { name: '删除', exact: true }).click();
  await expect(page.locator('.asset-card').filter({ hasText: 'E2E 手动文章' })).toHaveCount(0);
  expect(walk(path.join(WORKSPACE, '.mediadesk/trash')).some((f) => f.endsWith('.md'))).toBeTruthy();

  // 13. Theme persists after reload
  await page.goto('/settings');
  await page.locator('.appearance-row .segmented').getByRole('tab', { name: '深色' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.screenshot({ path: path.join(SHOTS, '09-dark-settings.png'), fullPage: true });

  expect(consoleErrors).toEqual([]);
});

test('mobile layout: drawer navigation and quick capture entry', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await expect(page.locator('.mobile-topbar')).toBeVisible();
  await page.getByRole('button', { name: '打开菜单' }).click();
  await expect(page.locator('.sidebar.is-open')).toBeVisible();
  await page.locator('.sidebar').getByRole('button', { name: /灵感/ }).click();
  await expect(page).toHaveURL(/\/ideas$/);
  await expect(page.locator('.sidebar.is-open')).toHaveCount(0);
  await page.screenshot({ path: path.join(SHOTS, '10-mobile-ideas.png'), fullPage: true });
  await page.getByRole('button', { name: '打开菜单' }).click();
  await page.locator('.sidebar .quick-capture-trigger').click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await dialog.getByPlaceholder(/此刻在想什么/).fill('移动端速记测试：想法应该随时可以写下。');
  await dialog.getByRole('button', { name: '收进灵感箱' }).click();
  await expect(page.getByText('移动端速记测试：想法应该随时可以写下。').first()).toBeVisible();
});

test('IME composition is not interrupted by Enter or controlled input resets', async ({ page }) => {
  await page.goto('/');
  await page.locator('.sidebar .quick-capture-trigger').click();
  const dialog = page.getByRole('dialog');
  const title = `IME 输入法测试 ${Date.now()}`;
  const titleInput = dialog.getByPlaceholder('给这个想法一个名字');
  await titleInput.click();
  await titleInput.evaluate((element, value) => {
    const input = element as HTMLInputElement;
    input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
    input.value = value;
    input.dispatchEvent(
      new InputEvent('input', { bubbles: true, isComposing: true, data: value }),
    );
    input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true, data: value }));
  }, title);
  await expect(titleInput).toHaveValue(title);

  const tagInput = dialog.getByPlaceholder('输入标签后按回车');
  await tagInput.fill('IME标签');
  await tagInput.evaluate((element) => {
    element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
    element.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, isComposing: true }),
    );
    element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
  });
  await expect(dialog.locator('.tag').filter({ hasText: 'IME标签' })).toHaveCount(0);
  await tagInput.press('Enter');
  await expect(dialog.locator('.tag').filter({ hasText: 'IME标签' })).toHaveCount(1);

  await dialog.getByPlaceholder(/此刻在想什么/).fill('输入法组合期间不应打断，也不应误触保存。');
  await dialog.getByRole('button', { name: '收进灵感箱' }).click();
  await expect(page.getByText(title).first()).toBeVisible();
});

test('switch workspace from settings and restore it', async ({ page, request }) => {
  const altWorkspace = path.join(ROOT, 'artifacts/e2e-alt-workspace');
  fs.rmSync(altWorkspace, { recursive: true, force: true });
  fs.mkdirSync(altWorkspace, { recursive: true });

  const marker = `工作区切换恢复测试 ${Date.now()}`;
  const created = await request.post('/api/ideas', {
    data: { title: marker, content: '切走再切回后应该仍然存在。' },
  });
  expect(created.ok()).toBeTruthy();
  const before = await (await request.get('/api/workspace')).json();
  expect(before.ideas).toBeGreaterThanOrEqual(1);

  await page.goto('/settings');
  await page.getByRole('button', { name: '更换工作区' }).click();
  const browser = page.getByRole('dialog').filter({ hasText: '选择工作区文件夹' });
  await browser.locator('.folder-path-input').fill(altWorkspace);
  await browser.getByRole('button', { name: '前往' }).click();
  await expect(browser.locator('.folder-path-input')).toHaveValue(altWorkspace);
  await browser.getByRole('button', { name: '使用此文件夹' }).click();
  await expect(page.locator('.toast-host')).toContainText('工作区已切换');
  const switched = await (await request.get('/api/workspace')).json();
  expect(switched.path).toBe(fs.realpathSync(altWorkspace));
  expect(switched.ideas).toBe(0);

  await page.goto('/ideas');
  await expect(page.getByText('灵感箱还是空的')).toBeVisible();

  await page.goto('/settings');
  await page.getByRole('button', { name: '更换工作区' }).click();
  const browserBack = page.getByRole('dialog').filter({ hasText: '选择工作区文件夹' });
  await browserBack.locator('.folder-path-input').fill(WORKSPACE);
  await browserBack.getByRole('button', { name: '前往' }).click();
  await expect(browserBack.locator('.folder-path-input')).toHaveValue(fs.realpathSync(WORKSPACE));
  await browserBack.getByRole('button', { name: '使用此文件夹' }).click();
  await expect(page.locator('.toast-host')).toContainText('工作区已切换');
  const restored = await (await request.get('/api/workspace')).json();
  expect(restored.ideas).toBeGreaterThanOrEqual(before.ideas);
  const search = await (await request.get(`/api/search?q=${encodeURIComponent('工作区切换恢复测试')}`)).json();
  expect(search.ideas.some((idea: { title: string }) => idea.title === marker)).toBe(true);
  await expect(page.getByText('e2e-workspace').first()).toBeVisible();
});

test('CLI text kernel and project assistant edits', async ({ page, request }) => {
  const created = await request.post('/api/projects', {
    data: { title: 'E2E CLI 与助手项目', brief: '验证本地 CLI 与项目助手。' },
  });
  expect(created.ok()).toBeTruthy();
  const project = await created.json();
  const projectId = project.id as string;
  const assetResponse = await request.post(`/api/projects/${projectId}/assets`, {
    data: { kind: 'article', title: '助手原文', content: '原始正文，等待助手修改。' },
  });
  expect(assetResponse.ok()).toBeTruthy();
  const originalAsset = await assetResponse.json();

  await page.goto('/settings');
  const kernelCard = page.locator('.kernel-card');
  await kernelCard.getByRole('button', { name: /Claude Code/ }).click();
  await expect(kernelCard.locator('.kernel-status')).toContainText('mock-cli.mjs');
  await kernelCard.getByRole('button', { name: '保存文本内核' }).click();
  await expect(page.locator('.toast-host')).toContainText('文本内核已保存');

  await page.goto(`/projects/${projectId}`);
  await page.getByRole('button', { name: '生成素材' }).click();
  const generate = page.getByRole('dialog');
  await generate.locator('textarea.generate-instruction').fill('用 CLI 写一段测试文字');
  await generate.getByRole('button', { name: '生成', exact: true }).click();
  await expect(generate.locator('.result-body')).toContainText('E2E CLI 生成文章', {
    timeout: 15_000,
  });
  await expect(generate.locator('.generate-kernel-pill')).toContainText('Claude');
  await page.keyboard.press('Escape');
  await expect(generate).not.toBeVisible();

  await page.locator('.project-tab').filter({ hasText: '脚本' }).click();
  const assistantFab = page.getByRole('button', { name: '打开项目助手' });
  await expect(assistantFab).toBeVisible();
  await assistantFab.click();
  const assistant = page.locator('.assistant-sidebar');
  await expect(assistant).toBeVisible();
  await expect(page.locator('.project-content')).toBeVisible();
  // The project remains interactive while the assistant is docked.
  await page.locator('.project-tab').filter({ hasText: '文章' }).click();
  await expect(page.locator('.kind-toolbar')).toBeVisible();
  await assistant.getByRole('button', { name: '建议式' }).click();
  await assistant.getByLabel('给项目助手的消息').fill('把原文改得更紧凑。');
  await assistant.getByRole('button', { name: '发送' }).click();
  await expect(assistant).toContainText('建议修改 1 条素材', { timeout: 15_000 });
  await assistant.getByRole('button', { name: '采纳并写入' }).click();
  const adopt = page.getByRole('dialog');
  await expect(adopt).toContainText('覆盖 1 条已有素材');
  await adopt.getByRole('button', { name: '覆盖素材' }).click();
  await expect(assistant).toContainText('已更新 1 条素材', { timeout: 15_000 });
  await dismissToasts(page);
  await page.screenshot({ path: path.join(SHOTS, '12-assistant.png') });
  await assistant.getByRole('button', { name: '关闭助手' }).click();
  await expect(page.locator('.assistant-sidebar')).toHaveCount(0);

  const updatedResponse = await request.get(
    `/api/projects/${projectId}/assets/${originalAsset.id}`,
  );
  const updated = await updatedResponse.json();
  expect(updated.content).toContain('E2E 助手改稿');
});

test('assistant creates new card assets in an empty directory', async ({ page, request }) => {
  const created = await request.post('/api/projects', {
    data: { title: 'E2E 助手新建文卡图', brief: '验证空 card 目录下的新建能力。' },
  });
  expect(created.ok()).toBeTruthy();
  const project = await created.json();
  const kernel = await request.put('/api/config/kernel', {
    data: { kernel: 'claude', binary: null, args: [], timeoutSecs: 60 },
  });
  expect(kernel.ok()).toBeTruthy();

  const cardDir = path.join(projectDir(project.dirName as string), 'card');
  expect(fs.existsSync(cardDir)).toBeTruthy();
  expect(fs.readdirSync(cardDir).filter((file) => file.endsWith('.md'))).toHaveLength(0);

  await page.goto(`/projects/${project.id}`);
  await page.getByRole('button', { name: '助手', exact: true }).click();
  const assistant = page.locator('.assistant-sidebar');
  await assistant.getByRole('button', { name: '建议式' }).click();
  await assistant.getByLabel('给项目助手的消息').fill('card 目录是空的，请新建 3 张文卡图。');
  await assistant.getByRole('button', { name: '发送' }).click();
  await expect(assistant).toContainText('建议新建 3 条素材', { timeout: 15_000 });
  await expect(assistant).toContainText('未创建');
  expect(fs.readdirSync(cardDir).filter((file) => file.endsWith('.md'))).toHaveLength(0);
  await page.screenshot({ path: path.join(SHOTS, '15-assistant-create.png') });

  await assistant.getByRole('button', { name: '采纳并写入' }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('新建 3 条素材');
  await dialog.getByRole('button', { name: '创建素材' }).click();
  await expect(assistant).toContainText('已新建 3 条素材', { timeout: 15_000 });

  const projectAfter = await (await request.get(`/api/projects/${project.id}`)).json();
  expect(projectAfter.assets.card).toHaveLength(3);
  expect(fs.readdirSync(cardDir).filter((file) => file.endsWith('.md'))).toHaveLength(3);
  await assistant.getByRole('button', { name: '关闭助手' }).click();
});

test('asset management deletes one item and a batch', async ({ page, request }) => {
  const created = await request.post('/api/projects', {
    data: { title: 'E2E 批量删除项目', brief: '验证单个和批量删除。' },
  });
  expect(created.ok()).toBeTruthy();
  const project = await created.json();
  for (const title of ['待删除 A', '待删除 B', '待删除 C']) {
    const asset = await request.post(`/api/projects/${project.id}/assets`, {
      data: { kind: 'article', title, content: `# ${title}\n\n这是用于删除测试的正文。` },
    });
    expect(asset.ok()).toBeTruthy();
  }

  await page.goto(`/projects/${project.id}`);
  await page.locator('.kind-overview-title').filter({ hasText: '文章' }).click();
  await expect(page.locator('.project-tab.is-active')).toContainText('文章');
  await expect(page.locator('.asset-card')).toHaveCount(3);

  const firstCard = page.locator('.asset-card').filter({ hasText: '待删除 A' });
  await firstCard.hover();
  await firstCard.getByRole('button', { name: /删除素材/ }).click();
  const singleDialog = page.getByRole('dialog');
  await expect(singleDialog).toContainText('删除这个素材');
  await singleDialog.getByRole('button', { name: '移入回收站' }).click();
  await expect(page.locator('.toast-host')).toContainText('已删除 1 个素材');
  await expect(page.locator('.asset-card')).toHaveCount(2);

  await page.getByRole('button', { name: '批量选择' }).click();
  const checkboxes = page.locator('.asset-card .asset-select input');
  await expect(checkboxes).toHaveCount(2);
  await checkboxes.nth(0).check();
  await checkboxes.nth(1).check();
  await page.screenshot({ path: path.join(SHOTS, '17-asset-batch-delete.png') });
  await page.getByRole('button', { name: '删除所选' }).click();
  const batchDialog = page.getByRole('dialog');
  await expect(batchDialog).toContainText('删除 2 个素材');
  await batchDialog.getByRole('button', { name: '移入回收站' }).click();
  await expect(page.locator('.toast-host')).toContainText('已删除 2 个素材');
  await expect(page.locator('.asset-card')).toHaveCount(0);

  const after = await (await request.get(`/api/projects/${project.id}`)).json();
  expect(after.assets.article).toHaveLength(0);
});

test('AI card studio generates an editorial PNG and saves it', async ({ page, request }) => {
  const created = await request.post('/api/projects', {
    data: { title: 'E2E AI 文卡图', brief: '把文章提炼成一张编辑式卡片。' },
  });
  expect(created.ok()).toBeTruthy();
  const project = await created.json();
  const article = await request.post(`/api/projects/${project.id}/assets`, {
    data: {
      kind: 'article',
      title: '把结论放到第一屏',
      content:
        '# 写作结构\n\n' +
        '读者不需要先看你如何抵达，才决定要不要继续读。先给出判断，再解释理由；卡片只保留一句值得记住的话。\n\n'.repeat(40),
    },
  });
  expect(article.ok()).toBeTruthy();
  const kernel = await request.put('/api/config/kernel', {
    data: { kernel: 'claude', binary: null, args: [], timeoutSecs: 60 },
  });
  expect(kernel.ok()).toBeTruthy();
  const media = await request.put('/api/config/media', {
    data: {
      illustration: {
        baseUrl: MOCK_URL,
        apiKey: 'media-key',
        model: 'mock-image-e2e',
        protocol: 'openai',
        createPath: '',
        statusPath: '',
        style: '暖纸色编辑插画',
      },
      video: {
        baseUrl: MOCK_URL,
        apiKey: 'media-key',
        model: 'mock-video-e2e',
        protocol: 'openai',
        createPath: '/videos',
        statusPath: '',
        style: '纪实慢镜头',
      },
      timeoutSecs: 30,
      pollIntervalSecs: 1,
      videoTimeoutSecs: 60,
    },
  });
  expect(media.ok()).toBeTruthy();

  await page.goto(`/projects/${project.id}`);
  await page.locator('.project-tab').filter({ hasText: '文卡图' }).click();
  await page.getByRole('button', { name: 'AI 文卡图' }).click();
  const studio = page.locator('.modal-panel.ai-card-modal');
  await expect(studio).toBeVisible();
  await studio.getByRole('button', { name: '生成文卡图' }).click();
  await expect(studio.getByRole('button', { name: '重新生成' })).toBeVisible({ timeout: 15_000 });
  await expect(studio).toContainText('已带入 AI 配图', { timeout: 15_000 });
  await expect(studio).toContainText('实际生成 3 张');
  await expect(studio.locator('.ai-card-canvas')).toHaveAttribute('width', '1080');
  await expect(studio.locator('.ai-card-thumb')).toHaveCount(3);
  await page.screenshot({ path: path.join(SHOTS, '16-ai-card.png') });
  await studio.getByRole('button', { name: '保存全部 3 张' }).click();
  await expect(page.locator('.toast-host')).toContainText('已存入 3 张文卡图 PNG', { timeout: 15_000 });
  await expect(page.getByRole('dialog').locator('.asset-editor')).toBeVisible();
  await page.getByRole('dialog').getByRole('button', { name: '取消' }).click();
  await expect(page.locator('.card-group-card')).toHaveCount(1);
  await expect(page.locator('.card-group-count')).toContainText('3 张');
  await page.screenshot({ path: path.join(SHOTS, '18-card-group-grid.png') });
  await page.locator('.card-group-card').click();
  await expect(page.locator('.card-group-modal')).toBeVisible();
  await expect(page.locator('.card-group-deck-item')).toHaveCount(3);
  await page.waitForTimeout(350);
  await page.screenshot({ path: path.join(SHOTS, '18-card-group.png') });
  await page.getByRole('contentinfo').getByRole('button', { name: '关闭' }).click();

  await page.locator('.project-tab').filter({ hasText: '插图' }).click();
  await expect(page.locator('.card-group-card')).toHaveCount(1);
  await page.locator('.card-group-card').click();
  await expect(page.locator('.card-group-deck-item')).toHaveCount(3);
  await expect(page.locator('.card-group-index')).toContainText('01 / 03');
  await page.keyboard.press('ArrowRight');
  await expect(page.locator('.card-group-index')).toContainText('02 / 03');
  await page.keyboard.press('ArrowLeft');
  await expect(page.locator('.card-group-index')).toContainText('01 / 03');
  await page.waitForTimeout(350);
  await page.screenshot({ path: path.join(SHOTS, '20-illustration-group.png') });
  await page.getByRole('contentinfo').getByRole('button', { name: '关闭' }).click();

  const after = await (await request.get(`/api/projects/${project.id}`)).json();
  expect(after.assets.card).toHaveLength(3);
  expect(after.assets.card.every((asset: { fileUrl?: string }) => Boolean(asset.fileUrl))).toBeTruthy();
  expect(after.assets.illustration).toHaveLength(3);
  const imageGroups = new Set(after.assets.illustration.map((asset: { cardGroupId?: string }) => asset.cardGroupId));
  expect(imageGroups.size).toBe(1);
  expect(after.assets.illustration[0].cardGroupTotal).toBe(3);
});

test('media generation saves an illustration into the project', async ({ page, request }) => {
  const created = await request.post('/api/projects', {
    data: { title: 'E2E 媒体生成项目', brief: '验证图像生成落盘。' },
  });
  expect(created.ok()).toBeTruthy();
  const project = await created.json();
  const projectId = project.id as string;

  const configured = await request.put('/api/config/media', {
    data: {
      illustration: {
        baseUrl: MOCK_URL,
        apiKey: 'media-key',
        model: 'mock-image-e2e',
        protocol: 'openai',
        createPath: '',
        statusPath: '',
        style: '暖纸色编辑插画',
      },
      video: {
        baseUrl: MOCK_URL,
        apiKey: 'media-key',
        model: 'mock-video-e2e',
        protocol: 'openai',
        createPath: '/videos',
        statusPath: '',
        style: '纪实慢镜头',
      },
      timeoutSecs: 30,
      pollIntervalSecs: 1,
      videoTimeoutSecs: 60,
    },
  });
  expect(configured.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}`);
  await page.locator('.project-tab').filter({ hasText: '插图' }).click();
  await page.locator('.kind-toolbar').getByRole('button', { name: '生成', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.locator('.generate-kind.is-active')).toContainText('插图');
  await dialog.locator('textarea.generate-instruction').fill('晨光下的书桌');
  await dialog.getByRole('button', { name: '生成并保存' }).click();
  await expect(dialog.locator('.result-media img')).toBeVisible({ timeout: 15_000 });
  await expect(dialog).toContainText('已写入项目');
  await page.screenshot({ path: path.join(SHOTS, '13-media-result.png'), fullPage: true });

  const projectResponse = await request.get(`/api/projects/${projectId}`);
  const refreshed = await projectResponse.json();
  expect(refreshed.assets.illustration.length).toBe(1);
  expect(refreshed.assets.illustration[0].file).toMatch(/\.png$/);

  await dialog.getByRole('button', { name: '查看并编辑' }).click();
  await expect(page.getByRole('dialog').locator('.asset-editor')).toBeVisible();
});


test('library assistant creates and deletes skills with isolated history', async ({ page, request }) => {
  const kernel = await request.put('/api/config/kernel', {
    data: { kernel: 'claude', binary: null, args: [], timeoutSecs: 60 },
  });
  expect(kernel.ok()).toBeTruthy();

  await page.goto('/library');
  const fab = page.getByRole('button', { name: '打开技能与预设助手' });
  await expect(fab).toBeVisible();
  await fab.click();
  const panel = page.locator('.library-assistant-sidebar');
  await expect(panel).toBeVisible();
  await panel.locator('textarea').fill('为文卡图创建一套 E2E 技能和预设。');
  await panel.getByRole('button', { name: '发送' }).click();
  await expect(panel).toContainText('已执行 2 项', { timeout: 15_000 });
  await page.screenshot({ path: path.join(SHOTS, '19-library-assistant.png') });

  await panel.getByRole('button', { name: '关闭技能助手' }).click();
  await expect(page.locator('.skill-card').filter({ hasText: 'E2E 文卡方法' })).toBeVisible();
  await page.locator('.library-tab').filter({ hasText: '创作预设' }).click();
  await expect(page.locator('.preset-card').filter({ hasText: 'E2E 文卡预设' })).toBeVisible();

  await page.getByRole('button', { name: '打开技能与预设助手' }).click();
  await panel.locator('textarea').fill('删除 E2E 文卡方法 和 E2E 文卡预设。');
  await panel.getByRole('button', { name: '发送' }).click();
  await expect(panel).toContainText('已执行 2 项', { timeout: 15_000 });
  await panel.getByRole('button', { name: '关闭技能助手' }).click();
  await expect(page.locator('.preset-card').filter({ hasText: 'E2E 文卡预设' })).toHaveCount(0);
  await page.locator('.library-tab').filter({ hasText: '创作技能' }).click();
  await expect(page.locator('.skill-card').filter({ hasText: 'E2E 文卡方法' })).toHaveCount(0);

  const skills = await (await request.get('/api/skills')).json();
  expect(skills.some((skill: { name: string }) => skill.name === 'E2E 文卡方法')).toBeFalsy();
});

test('provider catalog: accordion and model list', async ({ page }) => {
  await page.goto('/settings');
  const card = page.locator('.settings-card').filter({ hasText: '大模型接入' });
  await expect(card.locator('.provider-row')).toHaveCount(12);
  await expect(card.locator('.provider-panel')).toHaveCount(1);

  await card.locator('.provider-row').filter({ hasText: 'OpenRouter' }).click();
  await expect(card.locator('.provider-panel')).toHaveCount(1);
  await expect(card.locator('.provider-entry.is-open')).toContainText('OpenRouter');

  const openEntry = card.locator('.provider-entry.is-open');
  const inputs = openEntry.locator('.settings-form-grid').first().locator('input.input');
  await inputs.nth(0).fill(MOCK_URL);
  await openEntry.getByRole('button', { name: '获取列表' }).click();
  await expect(openEntry.locator('.model-option')).toHaveCount(2);
  await page.waitForTimeout(350);
  await page.screenshot({ path: path.join(SHOTS, '14-providers.png'), fullPage: false });
  await openEntry.locator('.model-option').filter({ hasText: 'Mock Model 2' }).click();
  await expect(inputs.nth(1)).toHaveValue('mock-model-2');

  await card.locator('.provider-row').filter({ hasText: 'OpenRouter' }).click();
  await expect(card.locator('.provider-panel')).toHaveCount(0);

  await card.locator('.provider-row').filter({ hasText: 'Kimi' }).click();
  await expect(card.locator('.provider-panel')).toHaveCount(1);
  await expect(card.locator('.provider-entry.is-open')).toContainText('Kimi');
  await card.getByLabel('搜索服务商').fill('Claude');
  await expect(card.locator('.provider-row')).toHaveCount(1);
  await expect(card.locator('.provider-row')).toContainText('Claude');
});
