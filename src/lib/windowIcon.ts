import { Image } from '@tauri-apps/api/image';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { iconPaths, type IconName } from '../components/Icon.svelte';

const SIZE = 64;

/** Sets the title bar / taskbar icon of this window to one of the app's icons on a blue tile. */
export async function setWindowIcon(name: IconName) {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = SIZE;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  ctx.fillStyle = '#4c8bf5';
  ctx.beginPath();
  ctx.roundRect(0, 0, SIZE, SIZE, 14);
  ctx.fill();

  // The icons are 24×24 strokes; draw them at 40 px in the middle of the tile.
  const scale = 40 / 24;
  ctx.translate((SIZE - 40) / 2, (SIZE - 40) / 2);
  ctx.scale(scale, scale);
  ctx.strokeStyle = '#fff';
  ctx.lineWidth = 2.2;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  ctx.stroke(new Path2D(iconPaths[name]));

  const rgba = ctx.getImageData(0, 0, SIZE, SIZE).data.buffer;
  await getCurrentWindow().setIcon(await Image.new(rgba, SIZE, SIZE));
}
