import { Image } from '@tauri-apps/api/image';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { iconPaths, type IconName } from '../components/Icon.svelte';

const SIZE = 64;
/** Tile color of the app icon (src-tauri/app-icon.svg). */
const APP_BLUE = '#4c8bf5';

/** Perceived brightness of a `#rrggbb` color, 0..1. */
function brightness(hex: string): number {
  const n = parseInt(hex.slice(1, 7), 16);
  if (Number.isNaN(n)) return 0;
  return (0.299 * ((n >> 16) & 255) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255)) / 255;
}

/**
 * Sets the title bar / taskbar icon of this window to one of the app's icons on a colored
 * tile – drawn like the app icon, so only the tile color differs.
 */
export async function setWindowIcon(name: IconName, color = APP_BLUE) {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = SIZE;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.roundRect(0, 0, SIZE, SIZE, 14);
  ctx.fill();

  // The icons are 24×24 strokes; draw them at 40 px in the middle of the tile.
  const scale = 40 / 24;
  ctx.translate((SIZE - 40) / 2, (SIZE - 40) / 2);
  ctx.scale(scale, scale);
  // Keep the symbol readable on light project colors.
  ctx.strokeStyle = brightness(color) > 0.75 ? '#1f2329' : '#fff';
  ctx.lineWidth = 2.2;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.stroke(new Path2D(iconPaths[name]));

  const rgba = ctx.getImageData(0, 0, SIZE, SIZE).data.buffer;
  await getCurrentWindow().setIcon(await Image.new(rgba, SIZE, SIZE));
}
