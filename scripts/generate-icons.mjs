import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, "..");
const brandDir = join(projectRoot, "assets", "brand");
const tauriIconsDir = join(projectRoot, "src-tauri", "icons");
const publicDir = join(projectRoot, "public");
const artworkPath = join(brandDir, "app-icon-artwork.png");
const artworkSvgPath = join(brandDir, "app-icon-artwork.svg");

if (!existsSync(artworkPath)) {
  throw new Error(`Missing icon artwork: ${relative(projectRoot, artworkPath)}`);
}
if (!existsSync(artworkSvgPath)) {
  throw new Error(`Missing icon artwork SVG: ${relative(projectRoot, artworkSvgPath)}`);
}

const artworkDataUri = `data:image/png;base64,${readFileSync(artworkPath).toString("base64")}`;
const macPlate = { x: 100, y: 100, size: 824 };
const windowsPlate = { x: 64, y: 64, size: 896, radius: 43 };

function superellipsePath({ x, y, size }, exponent = 5, steps = 160) {
  const center = x + size / 2;
  const radius = size / 2;
  const points = Array.from({ length: steps }, (_, index) => {
    const angle = (Math.PI * 2 * index) / steps;
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    const px = center + radius * Math.sign(cos) * Math.abs(cos) ** (2 / exponent);
    const py = center + radius * Math.sign(sin) * Math.abs(sin) ** (2 / exponent);
    return `${index === 0 ? "M" : "L"}${px.toFixed(3)} ${py.toFixed(3)}`;
  });
  return `${points.join(" ")} Z`;
}

function artworkInner() {
  const svg = readFileSync(artworkSvgPath, "utf8");
  const inner = svg.replace(/^[\s\S]*?<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "").trim();
  if (!inner) throw new Error("app-icon-artwork.svg is empty");
  return inner;
}

function composeVectorSvg(clipInner, { x, y, size }) {
  const scale = size / 1024;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" role="img" aria-label="AnySwitch">
  <defs><clipPath id="plate">${clipInner}</clipPath></defs>
  <g clip-path="url(#plate)">
    <g transform="translate(${x} ${y}) scale(${scale})">
      ${artworkInner()}
    </g>
  </g>
</svg>
`;
}

function buildMacSvg(imageHref) {
  const path = superellipsePath(macPlate);
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <defs><clipPath id="plate"><path d="${path}"/></clipPath></defs>
  <image href="${imageHref}" x="${macPlate.x}" y="${macPlate.y}" width="${macPlate.size}" height="${macPlate.size}" preserveAspectRatio="xMidYMid slice" clip-path="url(#plate)"/>
</svg>
`;
}

function buildWindowsSvg(imageHref) {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <defs><clipPath id="plate"><rect x="${windowsPlate.x}" y="${windowsPlate.y}" width="${windowsPlate.size}" height="${windowsPlate.size}" rx="${windowsPlate.radius}"/></clipPath></defs>
  <image href="${imageHref}" x="${windowsPlate.x}" y="${windowsPlate.y}" width="${windowsPlate.size}" height="${windowsPlate.size}" preserveAspectRatio="xMidYMid slice" clip-path="url(#plate)"/>
</svg>
`;
}

function buildMacVectorSvg() {
  return composeVectorSvg(`<path d="${superellipsePath(macPlate)}"/>`, macPlate);
}

function buildWindowsVectorSvg() {
  const { x, y, size, radius } = windowsPlate;
  return composeVectorSvg(`<rect x="${x}" y="${y}" width="${size}" height="${size}" rx="${radius}"/>`, windowsPlate);
}

function runTauriIcon(input, output, pngSize) {
  const args = ["icon"];
  if (pngSize) args.push("--png", String(pngSize));
  args.push("--output", output, input);
  // On Windows the CLI shim is a .cmd file, which spawnSync cannot resolve
  // without a shell — going through `pnpm exec` fails with ENOENT there.
  const localTauri = join(
    projectRoot,
    "node_modules",
    ".bin",
    process.platform === "win32" ? "tauri.cmd" : "tauri",
  );
  const result = spawnSync(localTauri, args, {
    cwd: projectRoot,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (result.status !== 0) throw new Error(`tauri icon failed for ${input}`);
}

function copyFile(source, destination) {
  mkdirSync(dirname(destination), { recursive: true });
  cpSync(source, destination);
}

function copyDirectory(source, destination) {
  mkdirSync(destination, { recursive: true });
  for (const name of readdirSync(source)) {
    cpSync(join(source, name), join(destination, name), { recursive: true });
  }
}

mkdirSync(brandDir, { recursive: true });
mkdirSync(tauriIconsDir, { recursive: true });
mkdirSync(publicDir, { recursive: true });

// Standalone vectors derived from app-icon-artwork.svg (no raster <image> href).
writeFileSync(join(brandDir, "app-icon.svg"), buildMacVectorSvg());
writeFileSync(join(brandDir, "app-icon-windows.svg"), buildWindowsVectorSvg());
writeFileSync(join(publicDir, "favicon.svg"), buildMacVectorSvg());

const tempRoot = mkdtempSync(join(tmpdir(), "any-switch-icons-"));
const macSvg = join(tempRoot, "macos.svg");
const windowsSvg = join(tempRoot, "windows.svg");
const macIcons = join(tempRoot, "macos");
const windowsIcons = join(tempRoot, "windows");
const mobileIcons = join(tempRoot, "mobile");
const macBrand = join(tempRoot, "macos-brand");
const windowsBrand = join(tempRoot, "windows-brand");

try {
  writeFileSync(macSvg, buildMacSvg(artworkDataUri));
  writeFileSync(windowsSvg, buildWindowsSvg(artworkDataUri));

  runTauriIcon(macSvg, macIcons);
  runTauriIcon(windowsSvg, windowsIcons);
  // Mobile platforms apply their own masks, so keep the source artwork full-bleed.
  runTauriIcon(artworkPath, mobileIcons);
  runTauriIcon(macSvg, macBrand, 1024);
  runTauriIcon(windowsSvg, windowsBrand, 1024);

  for (const name of ["32x32.png", "64x64.png", "128x128.png", "128x128@2x.png", "icon.png", "icon.icns"]) {
    copyFile(join(macIcons, name), join(tauriIconsDir, name));
  }

  copyFile(join(windowsIcons, "icon.ico"), join(tauriIconsDir, "icon.ico"));
  for (const name of readdirSync(windowsIcons).filter((name) => /^(Square.*Logo|StoreLogo)\.png$/.test(name))) {
    copyFile(join(windowsIcons, name), join(tauriIconsDir, name));
  }

  copyDirectory(join(mobileIcons, "ios"), join(tauriIconsDir, "ios"));
  copyDirectory(join(mobileIcons, "android"), join(tauriIconsDir, "android"));

  copyFile(join(macBrand, "1024x1024.png"), join(brandDir, "app-icon-1024.png"));
  copyFile(join(macBrand, "1024x1024.png"), join(brandDir, "app-icon-preview.png"));
  copyFile(join(windowsBrand, "1024x1024.png"), join(brandDir, "app-icon-windows-1024.png"));
  copyFile(join(macIcons, "32x32.png"), join(publicDir, "favicon.png"));
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}

console.log("Generated macOS, Windows, iOS, Android, brand, and favicon icon assets.");
