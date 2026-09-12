export function isOutcomeFile(filePath) {
  return filePath && filePath.includes('docs/outcomes/');
}

export function isPersonaFile(filePath) {
  return filePath && filePath.includes('docs/personas/');
}

export function isSrcFile(filePath) {
  if (!filePath) return false;
  if (/\.(config\.|d\.ts|node_modules|\.next|dist\/|build\/|\.test\.|\.spec\.|__tests__)/.test(filePath)) return false;
  return /\.(ts|tsx|js|jsx|py|svelte|vue)$/.test(filePath);
}

export function isSourceCodePath(filePath) {
  if (!filePath) return false;
  if (/(\.odd\/|docs\/|memory\/|MEMORY\.md|CLAUDE\.md|\.odd-flow|hooks\/|skills\/|skill\/|scripts\/)/.test(filePath)) return false;
  if (/(next\.config\.|postcss\.config\.|tailwind\.config\.|tsconfig|package\.json|\.env|drizzle\.config|vercel\.)/.test(filePath)) return false;
  return true;
}

export function isBriefFile(filePath) {
  return filePath && /session-brief-[0-9]+\.md$/.test(filePath);
}
