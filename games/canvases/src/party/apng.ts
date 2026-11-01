// PNG Third Edition, §11.3.6. The browser encodes each frame; we only assemble
// its existing IDAT payloads. One play ends on the final frame, never a loop.
const signature = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]);
const encoder = new TextEncoder();
const decoder = new TextDecoder();

function uint32(value: number) {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value);
  return bytes;
}
function join(parts: Uint8Array[]) {
  const result = new Uint8Array(
    parts.reduce((size, part) => size + part.length, 0),
  );
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}
const crcTable = Uint32Array.from({ length: 256 }, (_, i) => {
  let crc = i;
  for (let bit = 0; bit < 8; bit++)
    crc = crc & 1 ? 0xedb88320 ^ (crc >>> 1) : crc >>> 1;
  return crc >>> 0;
});
function chunk(type: string, data: Uint8Array) {
  const body = join([encoder.encode(type), data]);
  let crc = 0xffffffff;
  for (const byte of body) crc = crcTable[(crc ^ byte) & 255] ^ (crc >>> 8);
  return join([uint32(data.length), body, uint32((crc ^ 0xffffffff) >>> 0)]);
}
function readPng(bytes: Uint8Array) {
  if (!signature.every((byte, i) => bytes[i] === byte))
    throw new Error("Invalid PNG frame");
  const chunks: { type: string; data: Uint8Array }[] = [];
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  for (let offset = 8; offset + 12 <= bytes.length; ) {
    const length = view.getUint32(offset);
    if (offset + length + 12 > bytes.length)
      throw new Error("Incomplete PNG frame");
    const type = decoder.decode(bytes.subarray(offset + 4, offset + 8));
    chunks.push({ type, data: bytes.slice(offset + 8, offset + 8 + length) });
    offset += length + 12;
    if (type === "IEND") break;
  }
  const header = chunks.find((c) => c.type === "IHDR")?.data;
  const data = chunks.filter((c) => c.type === "IDAT").map((c) => c.data);
  if (
    !header ||
    header.length !== 13 ||
    !data.length ||
    !chunks.some((c) => c.type === "IEND")
  )
    throw new Error("Incomplete PNG frame");
  // Canvas output is truecolour, optionally with alpha, with no shared palette.
  if (![2, 6].includes(header[9]))
    throw new Error("Unsupported PNG frame colour");
  return { header, data };
}

export function encodeOneShotPng(
  frames: Uint8Array[],
  fps: number,
): Uint8Array {
  if (
    !frames.length ||
    frames.length > 120 ||
    !Number.isInteger(fps) ||
    fps < 1 ||
    fps > 60
  )
    throw new Error("Invalid animation length");
  const parsed = frames.map(readPng);
  const header = parsed[0].header;
  if (
    parsed.some((frame) => !frame.header.every((byte, i) => byte === header[i]))
  )
    throw new Error(
      "Animation frames must have the same dimensions and colour",
    );
  const output = [
    signature,
    chunk("IHDR", header),
    chunk("acTL", join([uint32(frames.length), uint32(1)])),
  ];
  let sequence = 0;
  for (let i = 0; i < parsed.length; i++) {
    const control = new Uint8Array(26);
    const view = new DataView(control.buffer);
    view.setUint32(0, sequence++);
    control.set(header.subarray(0, 8), 4); // Full canvas, no offsets.
    view.setUint16(20, 1);
    view.setUint16(22, fps);
    // dispose NONE, blend SOURCE: transparent pixels replace the previous frame.
    output.push(chunk("fcTL", control));
    for (const data of parsed[i].data)
      output.push(
        i === 0
          ? chunk("IDAT", data)
          : chunk("fdAT", join([uint32(sequence++), data])),
      );
  }
  output.push(chunk("IEND", new Uint8Array()));
  return join(output);
}
