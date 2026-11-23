import { useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import { FolderBrowser } from '../components/FolderBrowser';
import { MediaSettings } from '../components/MediaSettings';
import { ProviderSettings } from '../components/ProviderSettings';
import { TextKernelSettings } from '../components/TextKernelSettings';
import { Icon } from '../components/Icon';
import { Button, Modal, Segmented } from '../components/ui';
import { copyText, formatBytes } from '../lib/format';
import type { ThemeMode } from '../lib/types';

export function SettingsPage() {
  const app = useApp();
  const [folderOpen, setFolderOpen] = useState(false);
  const [switching, setSwitching] = useState(false);

  const switchWorkspace = async (path: string) => {
    setSwitching(true);
    try {
      await api.switchWorkspace(path);
      await app.refreshAll();
      app.toast({ message: '工作区已切换', tone: 'success' });
      setFolderOpen(false);
    } finally {
      setSwitching(false);
    }
  };

  const theme = app.theme;

  return (
    <div class="page settings-page">
      <header class="page-heading">
        <div>
          <p class="eyebrow">SETTINGS · 设置</p>
          <h1>把工具调成顺手的样子。</h1>
          <p class="page-subtitle">工作区、文本内核、服务商、媒体模型与外观。所有配置只保存在本机。</p>
        </div>
      </header>

      <div class="settings-layout">
        <section class="panel settings-card">
          <div class="settings-card-head">
            <div>
              <p class="eyebrow">WORKSPACE</p>
              <h2>工作区</h2>
            </div>
            <span class="settings-badge"><Icon name="folder" size={14} /> 本地文件</span>
          </div>
          <div class="workspace-path-card">
            <span class="workspace-path-icon"><Icon name="folder-open" size={20} /></span>
            <div>
              <strong>{app.data.workspace.path}</strong>
              <small>
                {app.data.workspace.ideas} 条想法 · {app.data.workspace.projects} 个项目 ·{' '}
                {app.data.workspace.assets} 个素材 · {formatBytes(app.data.workspace.bytes)}
              </small>
            </div>
          </div>
          <div class="settings-actions-row">
            <Button variant="secondary" icon="folder" onClick={() => setFolderOpen(true)}>
              更换工作区
            </Button>
            <Button
              variant="ghost"
              icon="folder-open"
              onClick={async () => {
                try {
                  await api.revealWorkspace();
                } catch (err) {
                  app.toast({ message: err instanceof ApiError ? err.message : '无法打开文件夹', tone: 'error' });
                }
              }}
            >
              在 Finder / 资源管理器中打开
            </Button>
            <Button
              variant="ghost"
              icon="copy"
              onClick={async () => {
                await copyText(app.data.workspace.path);
                app.toast({ message: '路径已复制', tone: 'success' });
              }}
            >
              复制路径
            </Button>
          </div>
          <p class="settings-note">
            工作区是可移植的普通目录：<code>ideas/</code>、<code>projects/</code> 与{' '}
            <code>.mediadesk/</code>。可以放进 Git、iCloud 或移动硬盘。
          </p>
        </section>

        <TextKernelSettings />

        <ProviderSettings />

        <MediaSettings />

        <section class="panel settings-card">
          <div class="settings-card-head">
            <div>
              <p class="eyebrow">APPEARANCE</p>
              <h2>外观</h2>
            </div>
          </div>
          <div class="appearance-row">
            <Segmented<ThemeMode>
              options={[
                { value: 'light', label: '浅色', icon: 'sun' },
                { value: 'dark', label: '深色', icon: 'moon' },
                { value: 'system', label: '跟随系统', icon: 'monitor' },
              ]}
              value={theme}
              onChange={(value) => app.setTheme(value)}
            />
            <p class="muted-text">主题只影响界面，不写入工作区。</p>
          </div>
        </section>

        <section class="panel settings-card about-card">
          <div class="about-mark">媒</div>
          <div>
            <h2>媒材工坊 MediaDesk <span class="version-pill">v{app.data.workspace.version}</span></h2>
            <p>本地优先的自媒体素材工作台。想法、项目与五类素材都以普通文件保存，可被 VS Code 与任意 AI 读取。</p>
            <div class="about-points">
              <span><Icon name="check" size={13} /> 无账号、无遥测</span>
              <span><Icon name="check" size={13} /> 无外部字体与 CDN</span>
              <span><Icon name="check" size={13} /> 数据可随时迁走</span>
              <span><Icon name="check" size={13} /> Rust + WebView 低占用</span>
            </div>
          </div>
        </section>
      </div>

      <FolderBrowser
        open={folderOpen}
        initialPath={app.data.workspace.path}
        onSelect={switchWorkspace}
        onClose={() => setFolderOpen(false)}
      />
      <Modal open={switching} onClose={() => {}} title="正在切换工作区" size="sm" hideClose>
        <div class="folder-loading"><span className="spinner" /> 读取工作区文件…</div>
      </Modal>
    </div>
  );
}
