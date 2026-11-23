#!/bin/sh
# Mock CLI used by integration tests. Reads the prompt from stdin.
prompt="$(cat)"
if printf '%s' "$prompt" | grep -q 'MEDIADESK_ASSISTANT_PROTOCOL'; then
  asset_id="$(printf '%s' "$prompt" | grep -o 'assetId: [A-Z0-9]*' | head -n 1 | awk '{print $2}')"
  if [ -n "$asset_id" ]; then
    printf '{"reply":"我根据项目上下文调整了这条素材，并说明了改动理由。","edits":[{"assetId":"%s","content":"# 助手修订版\\n\\n这是项目助手写入的新正文。\\n\\n改动理由：更贴合当前项目与对话。","summary":"更新正文并补充改动理由"}]}\n' "$asset_id"
  else
    printf '{"reply":"我看到了项目内容，但当前没有可编辑的素材。","edits":[]}\n'
  fi
else
  printf '# Mock CLI 生成结果\n\n这是由 mock CLI 内核生成的内容。\n\n工作目录：%s\n\n## 要点\n\n- 文本生成已走本地 CLI。\n- 结果仍会写入提示词快照。\n' "$(pwd)"
fi
