#!/usr/bin/env node
/**
 * Bump app version across package.json / tauri.conf.json / Cargo.toml /
 * Cargo.lock, create tag `vX.Y.Z`, optionally push to trigger Release CI.
 *
 * Usage:
 *   pnpm bump 0.1.1
 *   pnpm bump --push 0.1.1
 */
import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execFileSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith('--')));
const positional = args.filter((a) => !a.startsWith('--'));
const autoPush = flags.has('--push');

const version = positional[0];
if (!version) {
  console.error('用法: pnpm bump [--push] <version>');
  console.error('示例: pnpm bump 0.1.1');
  console.error('      pnpm bump --push 0.1.1  (自动 push commit 和 tag)');
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(`无效版本号: ${version}`);
  process.exit(1);
}

const jsonFiles = ['package.json', 'src-tauri/tauri.conf.json'];
const cargoToml = 'src-tauri/Cargo.toml';
const cargoLock = 'src-tauri/Cargo.lock';
const cargoPackage = 'xiaobai-switch-plus';
const allFiles = [...jsonFiles, cargoToml, cargoLock];
const tag = `v${version}`;

function git(args, options = {}) {
  return execFileSync('git', args, { cwd: root, stdio: 'inherit', ...options });
}

function gitOutput(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf-8' }).trim();
}

const tagExists = (() => {
  try {
    gitOutput(['rev-parse', '--verify', '--quiet', `refs/tags/${tag}`]);
    return true;
  } catch {
    return false;
  }
})();

function readJsonVersion(rel) {
  const filepath = resolve(root, rel);
  const json = JSON.parse(readFileSync(filepath, 'utf-8'));
  return { rel, filepath, kind: 'json', json, old: json.version };
}

function readCargoVersion(rel) {
  const filepath = resolve(root, rel);
  const content = readFileSync(filepath, 'utf-8');
  const match = content.match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`无法在 ${rel} 中找到 package version`);
  }
  return { rel, filepath, kind: 'cargo', content, old: match[1] };
}

const currentVersions = [
  ...jsonFiles.map(readJsonVersion),
  readCargoVersion(cargoToml),
];

if (tagExists && currentVersions.some(({ old }) => old !== version)) {
  console.error(
    `tag ${tag} 已存在，但版本文件尚未全部更新到 ${version}，已停止以避免覆盖已有发布标签。`,
  );
  process.exit(1);
}

let changed = false;

for (const entry of currentVersions) {
  const { rel, filepath, old } = entry;
  if (old === version) {
    console.log(`⏭️  ${rel}: 已是 ${version}`);
    continue;
  }

  if (entry.kind === 'json') {
    entry.json.version = version;
    writeFileSync(filepath, JSON.stringify(entry.json, null, 2) + '\n');
  } else {
    const next = entry.content.replace(
      /^version\s*=\s*"[^"]+"/m,
      `version = "${version}"`,
    );
    writeFileSync(filepath, next);
  }

  console.log(`✅ ${rel}: ${old} → ${version}`);
  changed = true;
}

console.log(`\n版本检查完成: ${version}`);

function syncCargoLock() {
  try {
    execFileSync(
      'cargo',
      ['update', '--manifest-path', cargoToml, '--offline', '-p', cargoPackage],
      { cwd: root, stdio: 'inherit' },
    );
  } catch {
    console.error(
      '无法同步 src-tauri/Cargo.lock。请确认已安装 cargo，并在仓库根目录可执行 cargo update。',
    );
    process.exit(1);
  }
}

syncCargoLock();
if (!changed) {
  console.log('版本文件已是目标版本，仍会检查 Cargo.lock 是否需要提交。');
}

git(['add', ...allFiles]);

try {
  git(['diff', '--cached', '--quiet', '--', ...allFiles], { stdio: 'ignore' });
  console.log('\n版本文件没有产生 Git diff，跳过 commit。');
} catch {
  git(['commit', '-m', `chore(version): bump version to ${tag}`]);
}

if (tagExists) {
  console.log(`🏷️  tag 已存在，跳过创建: ${tag}`);
} else {
  git(['tag', tag]);
  console.log(`🏷️  已创建 tag: ${tag}`);
}

if (autoPush) {
  git(['push']);
  git(['push', '--tags']);
  console.log(`\n🚀 已推送 commit 和 tag: ${tag}`);
} else {
  console.log(`📌 执行 git push && git push --tags 即可触发 release`);
}
