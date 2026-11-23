#!/bin/sh
# Mock assistant that proposes new card assets when no asset exists.
cat >/dev/null
printf '%s\n' '{"reply":"card 目录为空。我建议新建 2 张文卡图文案素材。","edits":[],"newAssets":[{"kind":"card","title":"先写下来，再慢慢变好","content":"先写下来，再慢慢变好。\n\n创作不是一次到位，而是让想法先有一个可以修改的形状。","summary":"关于创作节奏的短句卡"},{"kind":"card","title":"素材不是收藏","content":"素材不是收藏，是待使用的证据。\n\n只有被引用、被改写、被放进作品，它才真正开始工作。","summary":"关于素材使用的短句卡"}]}'
