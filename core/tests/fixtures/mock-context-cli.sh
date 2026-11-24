#!/bin/sh
prompt=$(cat)
chars=$(printf '%s' "$prompt" | wc -m | tr -d ' ')
printf '{"reply":"prompt_chars=%s","edits":[],"newAssets":[]}\n' "$chars"
