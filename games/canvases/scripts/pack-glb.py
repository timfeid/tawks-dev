"""Makes Kenney kit models self-contained: each GLB's external colormap is
embedded so the file can be bundled on its own.

    python3 -I scripts/pack-glb.py <kit>/Models/GLB\\ format <out-dir> name [name ...]
"""

import json
import struct
import sys
from pathlib import Path


def pack(source: Path, out: Path) -> None:
    data = source.read_bytes()
    json_len = struct.unpack_from("<I", data, 12)[0]
    gltf = json.loads(data[20 : 20 + json_len])
    bin_start = 20 + json_len
    bin_chunk = b""
    if bin_start < len(data):
        bin_len = struct.unpack_from("<I", data, bin_start)[0]
        bin_chunk = data[bin_start + 8 : bin_start + 8 + bin_len]

    for image in gltf.get("images", []):
        uri = image.pop("uri", None)
        if not uri:
            continue
        png = (source.parent / uri).read_bytes()
        offset = len(bin_chunk)
        bin_chunk += png + b"\0" * ((4 - len(png) % 4) % 4)
        gltf.setdefault("bufferViews", []).append({"buffer": 0, "byteOffset": offset, "byteLength": len(png)})
        image["bufferView"] = len(gltf["bufferViews"]) - 1
        image["mimeType"] = "image/png"
    gltf["buffers"] = [{"byteLength": len(bin_chunk)}]

    body = json.dumps(gltf, separators=(",", ":")).encode()
    body += b" " * ((4 - len(body) % 4) % 4)
    total = 12 + 8 + len(body) + 8 + len(bin_chunk)
    out.write_bytes(
        struct.pack("<III", 0x46546C67, 2, total)
        + struct.pack("<II", len(body), 0x4E4F534A)
        + body
        + struct.pack("<II", len(bin_chunk), 0x004E4942)
        + bin_chunk
    )


if __name__ == "__main__":
    kit, out_dir, *names = sys.argv[1:]
    for name in names:
        pack(Path(kit) / f"{name}.glb", Path(out_dir) / f"{name}.glb")
