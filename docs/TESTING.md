# 测试方案

## 输入框 / 输入法回归方案

### 背景问题

受控输入框在 IME 组合输入期间如果逐次更新 Preact 状态，候选词会被打断；早期的全局守卫又依赖一次准确的 `compositionend`。当 WebKit 在失焦、切换窗口或关闭弹窗时漏发 `compositionend`，守卫会永久停留在“组合中”：

- 所有输入框的 `input` 事件被拦截，输入看起来“失灵”；
- Enter / Escape / ⌘S 等快捷键全部失效；
- 用户只能刷新页面恢复。

标签输入还有一个独立问题：失焦时用 React/Preact state 中的旧 draft，而不是输入框当前 DOM 值，组合输入后立即点走会丢失标签。

### 自动化测试（必须保留）

主用例：[`tests/e2e/core.spec.ts`](../tests/e2e/core.spec.ts)

- `IME composition is not interrupted by Enter or controlled input resets`
- `project editor inputs keep the caret and survive a missing compositionend`
- `project editor inputs keep the caret while editing`

覆盖：

1. 组合输入期间按 Enter 不会误添加标签；
2. `compositionend` 后标题能正常提交；
3. **只有 `compositionstart`、故意不发送 `compositionend`** 时，切换到正文输入框后：
   - 标题最后的 DOM 值被提交；
   - 全局守卫释放；
   - 随后的正文输入和保存正常；
4. 项目素材编辑器标题在普通输入时保持中间光标（`abcd` → 光标移到 `ab|cd` → 输入 `X` 得到 `abXcd`）；
5. 同样的“缺失 compositionend”场景发生在**项目素材编辑器标题**时，失焦后保存写回正确标题；
6. 标签输入失焦使用输入框当前 DOM 值提交，不依赖旧 state；
7. 标题和正文 textarea 在中间光标位置插入字符都不会跳到末尾。

运行：

```bash
npm run e2e:prepare
CHROMIUM_PATH="$HOME/Library/Caches/ms-playwright/chromium-1200/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing" \
  npx playwright test -g "IME composition|project editor inputs"
```

### 手工验收矩阵

每个版本至少抽查一次：

| 场景 | 操作 | 期望 |
|---|---|---|
| 中文输入 | 在灵感标题/正文、素材标题/正文、助手输入框输入完整候选词 | 候选词不被重置；提交后文字完整 |
| 中途失焦 | 组合输入到一半，直接点击正文/标签/保存按钮 | 已输入的 DOM 值提交；不丢字；保存可继续 |
| 切换窗口 | 组合输入中切到其他 App 再回来 | 输入框仍可编辑；Enter 快捷键恢复 |
| 关闭弹窗 | 组合输入中按 Esc 或点关闭 | 不卡死；重新打开输入框可正常输入 |
| 标签输入 | 输入标签后直接点走，不按 Enter | 标签写入；草稿清空 |
| 光标编辑 | 在标题中间插入/删除字符 | 光标不跳到末尾，字符不丢失 |
| 原生窗口 | Tauri/WKWebView 与 Edge 各验证一次 | 行为一致 |

### 后续修改约束

- 不要移除 `installImeGuard()` 的 `focusout` / `window blur` / 30 秒 watchdog 兜底；
- 任何新的受控输入框必须接入 `onInput`，不要用 `onChange` 代替逐字输入；
- 不要在组合输入期间强制重设 value 或重新挂载输入框；
- 修改输入相关逻辑后，必须重跑上述 E2E 用例，不能只跑构建；
- 发布前记录：输入/IME 定向用例必须通过；若完整批量运行出现宿主 `page setup` 超时，必须单独重跑并确认不是业务失败。
