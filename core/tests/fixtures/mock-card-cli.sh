#!/bin/sh
# Mock card-plan generation used by integration tests.
cat >/dev/null
printf '%s\n' '{"cards":[{"kicker":"2026 · 11","page":"01 / 02","title":"把结论放到第一屏","body":["读者不需要先看你如何抵达，才决定要不要继续读。","先给出判断，再解释理由；卡片只保留一句值得记住的话。"],"footnote":"图注｜先写下来，再慢慢变好","source":"来源｜E2E 卡片测试","imagePrompt":"一张无文字的编辑式桌面照片，纸张、铅笔和晨光。"},{"kicker":"2026 · 11","page":"02 / 02","title":"素材不是收藏","body":["素材不是收藏，是待使用的证据。","只有被引用、被改写、被放进作品，它才真正开始工作。"],"footnote":"图注｜让素材进入作品","source":"来源｜E2E 卡片测试","imagePrompt":"一张无文字的编辑式资料柜照片，暖米色与深绿。"}]}'
