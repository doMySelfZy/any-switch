import { existsSync, writeFileSync, statSync, unlinkSync } from 'fs';
import { resolve } from 'path';

export function markerValid(markerPath, maxAgeMs) {
  if (!existsSync(markerPath)) return false;
  try {
    const { mtimeMs } = statSync(markerPath);
    return (Date.now() - mtimeMs) < maxAgeMs;
  } catch {
    return false;
  }
}

export function markerExists(name) {
  return existsSync(resolve(process.cwd(), '.odd', name));
}

export function touchMarker(name) {
  try {
    writeFileSync(resolve(process.cwd(), '.odd', name), '');
  } catch {}
}

export function removeMarker(name) {
  const filePath = resolve(process.cwd(), '.odd', name);
  if (!existsSync(filePath)) return;
  try {
    unlinkSync(filePath);
  } catch {}
}
