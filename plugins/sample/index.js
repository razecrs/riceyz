// Batcave sample plugin. Trigger: "echo".  Host sends {"query":"..."} per line on stdin;
// reply with {"results":[{title, subtitle, action}]} per line on stdout.
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });

rl.on('line', (line) => {
  let q = '';
  try { q = (JSON.parse(line).query || '').trim(); } catch (e) {}
  const results = [];
  if (q) {
    results.push({
      title: `Search Google for "${q}"`,
      subtitle: 'sample plugin',
      action: `open:https://www.google.com/search?q=${encodeURIComponent(q)}`,
    });
    results.push({
      title: `Copy "${q}"`,
      subtitle: 'sample plugin, copies to clipboard',
      action: `copy:${q}`,
    });
  }
  process.stdout.write(JSON.stringify({ results }) + '\n');
});
