#!/bin/sh
# Mock card-plan generation used by integration tests.
cat >/dev/null
printf '%s\n' '{"kicker":"2026 · 11","page":"01 / 01","title":"把结论放到第一屏","body":["读者不需要先看你如何抵达，才决定要不要继续读。","先给出判断，再解释理由；卡片只保留一句值得记住的话。"],"footnote":"图注｜先写下来，再慢慢变好","source":"来源｜E2E 卡片测试","imagePrompt":"一张无文字的编辑式桌面照片，纸张、铅笔和晨光。"}'
