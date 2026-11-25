#!/bin/sh
cat >/dev/null
printf '%s\n' '{"reply":"已创建测试技能与预设。","actions":[{"type":"createSkill","name":"测试文卡方法","description":"把文章拆成短卡","appliesTo":["article","card"],"content":"# 文卡方法\n\n一张卡片只保留一个观点。","code":[{"name":"card.ts","language":"typescript","content":"export const card = true;"}]},{"type":"createPreset","name":"测试文章预设","description":"克制的长文规范","kind":"article","skillIds":[],"content":"# 文章预设\n\n先给结论，再展开论证。"}]}'
