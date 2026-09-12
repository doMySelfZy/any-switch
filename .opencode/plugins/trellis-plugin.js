import { readFileSync, existsSync, writeFileSync } from 'fs';
import { execFileSync } from 'child_process';
import { resolve } from 'path';
import { createAfterHook, createBeforeHook, createChatHook } from './plugin-gates.js';
export { isSourceCodePath } from './plugin-paths.js';

function readState() {
  const stateFile = resolve(process.cwd(), '.odd', 'state.json');
  if (!existsSync(stateFile)) return null;
  try {
    return JSON.parse(readFileSync(stateFile, 'utf8'));
  } catch {
    return null;
  }
}

function writeState(state) {
  writeFileSync(resolve(process.cwd(), '.odd', 'state.json'), JSON.stringify(state, null, 2));
}

function readLastCommit() {
  try {
    return execFileSync('git', ['log', '-1', '--format=%H %s'], {
      cwd: process.cwd(),
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return 'unknown';
  }
}

export default function oddStudioPlugin() {
  return {
    'tool.execute.before': createBeforeHook(readState),
    'tool.execute.after': createAfterHook(readState, readLastCommit, writeState),
    'chat.message': createChatHook(readState),
  };
}
