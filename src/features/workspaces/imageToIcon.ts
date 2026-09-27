const ICON_SIZE = 128;

export async function imageToIcon(file: Blob): Promise<string> {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(ICON_SIZE / bitmap.width, ICON_SIZE / bitmap.height);
  const width = bitmap.width * scale;
  const height = bitmap.height * scale;
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = ICON_SIZE;
  canvas.getContext('2d')!.drawImage(bitmap, (ICON_SIZE - width) / 2, (ICON_SIZE - height) / 2, width, height);
  return canvas.toDataURL('image/png');
}
