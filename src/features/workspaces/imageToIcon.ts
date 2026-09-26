export async function imageToIcon(file: Blob): Promise<string> {
  const bitmap = await createImageBitmap(file);
  const size = 128;
  const scale = Math.min(size / bitmap.width, size / bitmap.height);
  const [w, h] = [bitmap.width * scale, bitmap.height * scale];
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = size;
  canvas.getContext('2d')!.drawImage(bitmap, (size - w) / 2, (size - h) / 2, w, h);
  return canvas.toDataURL('image/png');
}
