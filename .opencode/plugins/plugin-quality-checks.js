import { readFileSync, existsSync } from 'fs';
import { isSrcFile } from './plugin-paths.js';

export function checkOutcomeQuality(filePath) {
  if (!existsSync(filePath)) return [];
  const content = readFileSync(filePath, 'utf8');
  const warnings = [];
  const fields = [
    [/Field 1.*Persona|## Persona/i, 'Field 1 (Persona)'],
    [/Field 2.*Trigger|## Trigger/i, 'Field 2 (Trigger)'],
    [/Field 3.*Walkthrough|## Walkthrough/i, 'Field 3 (Walkthrough)'],
    [/Field 4.*Verif|## Verif/i, 'Field 4 (Verification)'],
    [/Field 5.*Contracts|## Contracts/i, 'Field 5 (Contracts Exposed)'],
    [/Field 6.*Dep|## Dep/i, 'Field 6 (Dependencies)'],
  ];
  const missing = fields.filter(([pattern]) => !pattern.test(content)).map(([, name]) => name);
  if (missing.length > 0) warnings.push(`Incomplete outcome — missing: ${missing.join(', ')}`);

  const banned = /\b(user story|user stories|sprint|epic|backlog|api endpoint|database schema|database table|json payload|http request)\b/i;
  if (banned.test(content)) warnings.push('Banned vocabulary detected — ODD outcomes must use domain language only');
  return warnings;
}

export function checkPersonaQuality(filePath) {
  if (!existsSync(filePath)) return [];
  const content = readFileSync(filePath, 'utf8');
  const dims = ['Identity', 'Reality', 'Psychology', 'Trigger', 'History', 'Success', 'Constraints'];
  const missing = dims.filter((dimension) => !new RegExp(`## ${dimension}|${dimension.toLowerCase()}`, 'i').test(content));
  return missing.length > 0 ? [`Thin persona — missing dimensions: ${missing.join(', ')}`] : [];
}

export function checkCodeElegance(filePath) {
  if (!existsSync(filePath) || !isSrcFile(filePath)) return [];
  const content = readFileSync(filePath, 'utf8');
  const lines = content.split('\n');
  const warnings = [];

  if (lines.length > 300) warnings.push(`File is ${lines.length} lines — ODD limit is 300. Split into smaller modules.`);

  const importCount = lines.filter((line) => /^import /.test(line)).length;
  if (importCount > 8) warnings.push(`${importCount} imports (limit: 8). Module may be too coupled.`);

  const fnPattern = /^[\t ]*(export )?(async )?(function |const [a-zA-Z]+ = (\([^)]*\)|[a-zA-Z]+) =>)/;
  let fnStart = -1;
  let fnName = '';
  let depth = 0;
  let started = false;

  for (let index = 0; index < lines.length; index += 1) {
    if (fnStart === -1 && fnPattern.test(lines[index])) {
      fnStart = index;
      fnName = lines[index].trim().slice(0, 60);
      depth = 0;
      started = false;
    }
    if (fnStart === -1) continue;

    const opens = (lines[index].match(/{/g) || []).length;
    const closes = (lines[index].match(/}/g) || []).length;
    if (opens > 0) started = true;
    depth += opens - closes;
    if (started && depth <= 0) {
      const length = index - fnStart;
      if (length > 25) warnings.push(`Function exceeds 25-line limit: ${fnName} (line ${fnStart + 1}, ${length} lines)`);
      fnStart = -1;
    }
  }

  const deepLines = lines
    .map((line, index) => ({ line, num: index + 1 }))
    .filter(({ line }) => /^ {16,}\S/.test(line) && !/^\s*[/*]/.test(line) && !/className/.test(line) && !/^\s*</.test(line));

  if (deepLines.length > 0) {
    const samples = deepLines.slice(0, 3).map((entry) => `    Line ${entry.num}: ${entry.line.trim().slice(0, 70)}`);
    const extra = deepLines.length > 3 ? `\n    ... and ${deepLines.length - 3} more` : '';
    warnings.push(`Deep nesting detected (>3 levels):\n${samples.join('\n')}${extra}`);
  }

  return warnings;
}

export function checkSecurityBaseline(filePath) {
  if (!existsSync(filePath) || !isSrcFile(filePath)) return [];
  const content = readFileSync(filePath, 'utf8');
  const warnings = [];

  const riskyPatterns = [
    [/\b(api[_-]?key|secret|token|password)\b[^=\n]{0,40}[:=]\s*['"`][^'"`\n]{8,}['"`]/i, 'Possible hardcoded secret or credential literal'],
    [/dangerouslySetInnerHTML/, 'Unsafe HTML rendering detected — prove sanitisation or remove it'],
    [/(localStorage|sessionStorage)\.(setItem|getItem)\([^)]*(token|session|auth|jwt)/i, 'Client-side token/session storage detected'],
    [/strategy\s*:\s*['"]jwt['"]/i, 'JWT session shortcut detected — prefer server-managed session state'],
    [/(rejectUnauthorized|NODE_TLS_REJECT_UNAUTHORIZED|skipCsrfCheck|verify)\s*[:=]\s*(false|0|['"]false['"])/i, 'Security verification appears disabled'],
  ];

  for (const [pattern, message] of riskyPatterns) {
    if (pattern.test(content)) warnings.push(message);
  }

  return warnings;
}

export function checkBriefQuality(filePath) {
  if (!existsSync(filePath)) return [];
  const content = readFileSync(filePath, 'utf8');
  const lines = content.split('\n');
  const warnings = [];

  if (lines.length < 200) warnings.push(`Brief is only ${lines.length} lines (minimum 200). Full walkthroughs likely missing.`);

  const requiredSections = [
    '## Overview',
    '## Active Personas',
    '## Outcomes In Scope',
    '## Available From Previous',
    '## Build Sequence',
    '## Known Failure',
    '## Not In Scope',
  ];

  requiredSections.forEach((section) => {
    if (!content.includes(section)) warnings.push(`Missing required section: ${section}`);
  });

  const outcomeCount = (content.match(/^### Outcome/gm) || []).length;
  const walkthroughCount = (content.match(/^\*\*Walkthrough/gm) || []).length;
  const verificationCount = (content.match(/^\*\*Verification/gm) || []).length;
  const contractCount = (content.match(/^\*\*Contracts exposed/gm) || []).length;

  if (walkthroughCount < outcomeCount) warnings.push(`${outcomeCount} outcomes but only ${walkthroughCount} walkthroughs`);
  if (verificationCount < outcomeCount) warnings.push(`${outcomeCount} outcomes but only ${verificationCount} verification sections`);
  if (contractCount < outcomeCount) warnings.push(`${outcomeCount} outcomes but only ${contractCount} contracts sections`);

  return warnings;
}
