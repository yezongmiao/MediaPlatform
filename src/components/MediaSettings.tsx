import { useEffect, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { MediaProfileView } from '../lib/types';
import { Button, Field, InlineError } from './ui';
import { Icon } from './Icon';

type MediaKind = 'illustration' | 'video';

interface ProfileDraft {
  baseUrl: string;
  model: string;
  apiKey: string;
  clearKey: boolean;
  protocol: string;
  createPath: string;
  statusPath: string;
  style: string;
  hasKey: boolean;
}

function toDraft(profile?: MediaProfileView): ProfileDraft {
  return {
    baseUrl: profile?.baseUrl ?? '',
    model: profile?.model ?? '',
    apiKey: '',
    clearKey: false,
    protocol: profile?.protocol ?? 'openai',
    createPath: profile?.createPath ?? '',
    statusPath: profile?.statusPath ?? '',
    style: profile?.style ?? '',
    hasKey: profile?.hasApiKey ?? false,
  };
}

export function MediaSettings() {
  const app = useApp();
  const config = app.data.config;
  const [active, setActive] = useState<MediaKind>('illustration');
  const [illustration, setIllustration] = useState<ProfileDraft>(() =>
    toDraft(config.mediaGen?.illustration),
  );
  const [video, setVideo] = useState<ProfileDraft>(() => toDraft(config.mediaGen?.video));
  const [timeoutSecs, setTimeoutSecs] = useState(config.mediaGen?.timeoutSecs ?? 300);
  const [pollIntervalSecs, setPollIntervalSecs] = useState(
    config.mediaGen?.pollIntervalSecs ?? 5,
  );
  const [videoTimeoutSecs, setVideoTimeoutSecs] = useState(
    config.mediaGen?.videoTimeoutSecs ?? 600,
  );
  const [advanced, setAdvanced] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    setIllustration(toDraft(config.mediaGen?.illustration));
    setVideo(toDraft(config.mediaGen?.video));
    setTimeoutSecs(config.mediaGen?.timeoutSecs ?? 300);
    setPollIntervalSecs(config.mediaGen?.pollIntervalSecs ?? 5);
    setVideoTimeoutSecs(config.mediaGen?.videoTimeoutSecs ?? 600);
  }, [config]);

  const draft = active === 'illustration' ? illustration : video;
  const setDraft = (patch: Partial<ProfileDraft>) => {
    if (active === 'illustration') {
      setIllustration((prev) => ({ ...prev, ...patch }));
    } else {
      setVideo((prev) => ({ ...prev, ...patch }));
    }
  };

  const configured =
    (illustration.baseUrl.trim() && illustration.model.trim()) ||
    (video.baseUrl.trim() && video.model.trim());

  const save = async () => {
    setSaving(true);
    setError('');
    try {
      const makeProfile = (profile: ProfileDraft) => ({
        baseUrl: profile.baseUrl.trim(),
        apiKey: profile.clearKey ? '' : profile.apiKey.trim() ? profile.apiKey.trim() : undefined,
        model: profile.model.trim(),
        protocol: profile.protocol,
        createPath: profile.createPath.trim(),
        statusPath: profile.statusPath.trim(),
        style: profile.style.trim(),
      });
      await api.saveMediaConfig({
        illustration: makeProfile(illustration),
        video: makeProfile(video),
        timeoutSecs,
        pollIntervalSecs,
        videoTimeoutSecs,
      });
      setIllustration((prev) => ({ ...prev, apiKey: '', clearKey: false }));
      setVideo((prev) => ({ ...prev, apiKey: '', clearKey: false }));
      await app.refreshConfig();
      setSaved(true);
      window.setTimeout(() => setSaved(false), 1800);
      app.toast({ message: '媒体生成配置已保存', tone: 'success' });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const statusText = configured ? '已配置媒体模型' : '尚未配置';

  return (
    <section class="panel settings-card media-card">
      <div class="settings-card-head">
        <div>
          <p class="eyebrow">MEDIA · 媒体生成</p>
          <h2>插图与视频生成模型</h2>
        </div>
        <span class={`settings-badge ${configured ? 'is-ok' : ''}`}>
          <span class="status-dot" />
          {statusText}
        </span>
      </div>
      <p class="settings-note">
        文章、脚本、文卡图由文本内核生成；插图和视频需要单独的媒体接口。图像走 OpenAI Images 兼容格式，视频支持 OpenAI、通用 task 与 Seedance 任务格式。
      </p>

      <div class="media-tabs">
        <button class={`media-tab ${active === 'illustration' ? 'is-active' : ''}`} onClick={() => setActive('illustration')}>
          <Icon name="image" size={16} />
          图像模型
          <span>{illustration.baseUrl && illustration.model ? '已配置' : '未配置'}</span>
        </button>
        <button class={`media-tab ${active === 'video' ? 'is-active' : ''}`} onClick={() => setActive('video')}>
          <Icon name="video" size={16} />
          视频模型
          <span>{video.baseUrl && video.model ? '已配置' : '未配置'}</span>
        </button>
      </div>

      <div class="settings-form-grid">
        <Field label="Base URL" hint="例如 https://api.openai.com/v1">
          <input
            class="input"
            value={draft.baseUrl}
            placeholder="https://..."
            onInput={(e) => setDraft({ baseUrl: (e.target as HTMLInputElement).value })}
          />
        </Field>
        <Field label="模型 ID">
          <input
            class="input"
            value={draft.model}
            placeholder={active === 'illustration' ? 'gpt-image-1' : 'sora / seedance / 自定义'}
            onInput={(e) => setDraft({ model: (e.target as HTMLInputElement).value })}
          />
        </Field>
      </div>

      <Field
        label="API Key"
        hint={
          draft.hasKey && !draft.clearKey
            ? '已保存在本机。留空保持不变；输入新值可覆盖。'
            : '留空表示该媒体模型不需要 Key（例如本地服务）。'
        }
      >
        <div class="secret-input">
          <input
            class="input"
            type="password"
            value={draft.apiKey}
            placeholder={draft.hasKey && !draft.clearKey ? '••••••••••••（已保存）' : 'sk-…'}
            autoComplete="off"
            onInput={(e) => setDraft({ apiKey: (e.target as HTMLInputElement).value, clearKey: false })}
          />
          {draft.hasKey ? (
            <button
              type="button"
              class="text-link danger-link"
              onClick={() => setDraft({ clearKey: !draft.clearKey, apiKey: '' })}
            >
              {draft.clearKey ? '撤销清除' : '清除'}
            </button>
          ) : null}
        </div>
      </Field>

      <div class="settings-form-grid">
        <Field label="协议">
          <select
            class="select"
            value={draft.protocol}
            onChange={(e) => setDraft({ protocol: (e.target as HTMLSelectElement).value })}
          >
            <option value="openai">OpenAI 兼容</option>
            {active === 'video' ? <option value="task">通用 task</option> : null}
            {active === 'video' ? <option value="seedance">Seedance / 方舟</option> : null}
          </select>
        </Field>
        <Field label="风格（会附加进提示词）">
          <input
            class="input"
            value={draft.style}
            placeholder={active === 'illustration' ? '暖纸色编辑插画、克制' : '纪实、自然光、慢镜头'}
            onInput={(e) => setDraft({ style: (e.target as HTMLInputElement).value })}
          />
        </Field>
      </div>

      <button class={`advanced-toggle ${advanced ? 'is-open' : ''}`} onClick={() => setAdvanced(!advanced)}>
        <span>
          <Icon name="sliders" size={15} />
          接口路径与轮询
        </span>
        <Icon name="chevron-down" size={15} />
      </button>

      {advanced ? (
        <div class="media-advanced">
          <div class="settings-form-grid">
            <Field
              label="创建路径"
              hint={active === 'illustration' ? '留空使用 /images/generations' : '留空使用 /videos；Seedance 默认 /contents/generations/tasks'}
            >
              <input
                class="input"
                value={draft.createPath}
                placeholder={active === 'illustration' ? '/images/generations' : draft.protocol === 'seedance' ? '/contents/generations/tasks' : '/videos'}
                onInput={(e) => setDraft({ createPath: (e.target as HTMLInputElement).value })}
              />
            </Field>
            <Field label="任务状态路径" hint="仅任务式视频使用；{id} 会替换为任务 ID；留空为创建路径/{id}">
              <input
                class="input"
                value={draft.statusPath}
                placeholder={draft.protocol === 'seedance' ? '/contents/generations/tasks/{id}' : '/videos/{id}'}
                onInput={(e) => setDraft({ statusPath: (e.target as HTMLInputElement).value })}
              />
            </Field>
          </div>
          <div class="settings-form-grid three">
            <Field label="请求超时（秒）">
              <input
                class="input"
                type="number"
                min={10}
                max={1800}
                value={timeoutSecs}
                onInput={(e) => setTimeoutSecs(Number((e.target as HTMLInputElement).value))}
              />
            </Field>
            <Field label="任务轮询间隔（秒）">
              <input
                class="input"
                type="number"
                min={1}
                max={60}
                value={pollIntervalSecs}
                onInput={(e) => setPollIntervalSecs(Number((e.target as HTMLInputElement).value))}
              />
            </Field>
            <Field label="视频任务超时（秒）">
              <input
                class="input"
                type="number"
                min={30}
                max={3600}
                value={videoTimeoutSecs}
                onInput={(e) => setVideoTimeoutSecs(Number((e.target as HTMLInputElement).value))}
              />
            </Field>
          </div>
        </div>
      ) : null}

      <p class="settings-note">
        生成成功后媒体文件会直接写入项目对应目录，并生成 sidecar；图像接口返回的 <code>url</code> 或 <code>b64_json</code> 都会处理。
      </p>
      {error ? <InlineError message={error} /> : null}
      <div class="settings-actions-row end">
        <Button variant="primary" icon={saved ? 'check' : undefined} loading={saving} onClick={save}>
          {saved ? '已保存' : '保存媒体配置'}
        </Button>
      </div>
    </section>
  );
}
