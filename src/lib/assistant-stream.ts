/**
 * Assistant models stream a JSON object (`{"reply":"...","edits":[...]}`).
 * While it is arriving, show the readable reply portion instead of raw JSON.
 * Plain-text replies (or non-JSON CLIs) are shown as-is.
 */
export function extractStreamReply(raw: string): string {
  if (!raw) return '';
  const replyMarker = /"reply"\s*:/.exec(raw);
  if (!replyMarker) {
    return raw.trimStart().startsWith('{') ? '' : raw;
  }
  let cursor = replyMarker.index + replyMarker[0].length;
  while (cursor < raw.length && /\s/.test(raw[cursor])) cursor += 1;
  if (raw[cursor] !== '"') return '';
  cursor += 1;

  let out = '';
  let escaped = false;
  while (cursor < raw.length) {
    const char = raw[cursor];
    if (escaped) {
      escaped = false;
      if (char === 'n') out += '\n';
      else if (char === 'r') out += '\r';
      else if (char === 't') out += '\t';
      else if (char === 'b') out += '\b';
      else if (char === 'f') out += '\f';
      else if (char === 'u') {
        const hex = raw.slice(cursor + 1, cursor + 5);
        if (!/^[0-9a-fA-F]{4}$/.test(hex)) return out;
        out += String.fromCharCode(parseInt(hex, 16));
        cursor += 4;
      } else if (char === '"' || char === '\\' || char === '/') {
        out += char;
      } else {
        out += char;
      }
      cursor += 1;
      continue;
    }
    if (char === '\\') {
      escaped = true;
      cursor += 1;
      continue;
    }
    if (char === '"') break;
    out += char;
    cursor += 1;
  }
  return out;
}
