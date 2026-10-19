// No em dashes or en dashes anywhere. This scans every
// text file git tracks or has staged, and fails lint when one slips in.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const TEXT = /\.(md|ts|tsx|js|mjs|css|html|json|rs|toml|yml|yaml|txt)$/;
// Built from code points so this file never contains the characters it bans.
const DASH = new RegExp(`[${String.fromCharCode(0x2013)}${String.fromCharCode(0x2014)}]`);

const files = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard'], { encoding: 'utf8' })
  .split('\n')
  .filter((file) => file && TEXT.test(file));

const hits = [];
for (const file of files) {
  const lines = readFileSync(file, 'utf8').split('\n');
  lines.forEach((line, index) => {
    if (DASH.test(line)) hits.push(`${file}:${index + 1}: ${line.trim()}`);
  });
}

if (hits.length > 0) {
  process.stderr.write(`Long dashes are not allowed:\n${hits.join('\n')}\n`);
  process.exit(1);
}
