#!/bin/sh
# Mock Claude Code stream-json output for assistant streaming tests.
cat >/dev/null
printf '%s\n' '{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"{\"reply\":\"流式"}}}'
sleep 0.05
printf '%s\n' '{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"解析成功。\",\"edits\":[]}"}}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"result":"{\"reply\":\"流式解析成功。\",\"edits\":[]}"}'
