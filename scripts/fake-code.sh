#!/bin/sh
# Test double for the VS Code CLI. Records invocation instead of opening an editor.
printf '%s\n' "$@" >> "${MEDIADESK_CODE_LOG:-/tmp/mediadesk-fake-code.log}"
exit 0
