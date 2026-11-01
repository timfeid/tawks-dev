"""Rebuilds the five pet GLBs from Quaternius' CC0 Cute Animated Monsters pack.

    uv run --no-project --python 3.11 --with bpy==4.2.0 \
      python -I build-pets.py "<pack>/Cute Animated Monsters - Aug 2020" <out-dir>

Each source texture is repainted in flat colour regions from recolour.json:
a rule matches texels by sRGB hue/saturation/value (and optionally a UV
rectangle, top-left origin), first match wins, and `keep` is how much of the
painted shading survives. The texture is shrunk to 256 px and embedded.
"""
import json
import os
import sys
from pathlib import Path

import bpy
import numpy as np

pack, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
pets = json.loads((Path(__file__).parent / "recolour.json").read_text())


def to_srgb(lin):
    return np.where(lin <= 0.0031308, lin * 12.92, 1.055 * np.power(np.maximum(lin, 0), 1 / 2.4) - 0.055)


def to_linear(s):
    return np.where(s <= 0.04045, s / 12.92, np.power((s + 0.055) / 1.055, 2.4))


def rgb(code):
    return np.array([int(code[i:i + 2], 16) / 255 for i in (1, 3, 5)], np.float32)


def repaint(path, rules):
    image = bpy.data.images.load(str(path))
    w, h = image.size
    colour = to_srgb(np.array(image.pixels[:], np.float32).reshape(h, w, 4)[:, :, :3])
    high, low = colour.max(-1), colour.min(-1)
    spread = np.maximum(high - low, 1e-6)
    sat = np.where(high > 0, (high - low) / np.maximum(high, 1e-6), 0)
    r, g, b = colour[..., 0], colour[..., 1], colour[..., 2]
    hue = np.where(high == r, ((g - b) / spread) % 6, np.where(high == g, (b - r) / spread + 2, (r - g) / spread + 4)) * 60
    hue = np.where(sat < 0.04, -1, hue)
    rows, cols = np.mgrid[0:h, 0:w]
    rows = h - 1 - rows
    result, taken = colour.copy(), np.zeros((h, w), bool)
    for rule in rules:
        match = ~taken
        for key, channel in (("h", hue), ("s", sat), ("v", high)):
            if key in rule:
                lo, hi = rule[key]
                match &= (channel >= lo) & (channel <= hi)
        if "region" in rule:
            x0, y0, x1, y1 = rule["region"]
            match &= (cols >= x0 * w) & (cols < x1 * w) & (rows >= y0 * h) & (rows < y1 * h)
        taken |= match
        if "to" not in rule or not match.any():
            continue
        shade = 1 + rule.get("keep", 0.3) * (high[match] / max(high[match].mean(), 1e-3) - 1)
        result[match] = np.clip(rgb(rule["to"])[None] * shade[:, None], 0, 1)
    image.pixels[:] = np.concatenate([to_linear(result), np.ones((h, w, 1), np.float32)], -1).ravel()
    image.scale(256, 256)
    image.file_format = "PNG"
    image.pack()
    return image


for pet in pets:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.wm.open_mainfile(filepath=str(pack / "Blends" / f"{pet['source']}.blend"), load_ui=False)
    texture = repaint(pack / "Textures" / f"{pet['source']}_Texture.png", pet["rules"])
    for action in list(bpy.data.actions):
        if action.name in pet["actions"]:
            action.name = pet["actions"][action.name]
        else:
            bpy.data.actions.remove(action)
    for obj in list(bpy.data.objects):
        if obj.type not in {"MESH", "ARMATURE"}:
            bpy.data.objects.remove(obj, do_unlink=True)
    for material in bpy.data.materials:
        material.use_nodes = True
        nodes = material.node_tree.nodes
        nodes.clear()
        shader = nodes.new("ShaderNodeBsdfPrincipled")
        shader.inputs["Metallic"].default_value = 0
        shader.inputs["Roughness"].default_value = 0.85
        node = nodes.new("ShaderNodeTexImage")
        node.image = texture
        output = nodes.new("ShaderNodeOutputMaterial")
        material.node_tree.links.new(node.outputs["Color"], shader.inputs["Base Color"])
        material.node_tree.links.new(shader.outputs["BSDF"], output.inputs["Surface"])
        material.name = pet["id"]
    armature = next(obj for obj in bpy.data.objects if obj.type == "ARMATURE")
    armature.name = pet["id"]
    armature.animation_data_create()
    armature.animation_data.action = bpy.data.actions.get("Idle")
    bpy.context.scene.frame_set(0)
    for obj in bpy.context.scene.objects:
        obj.select_set(obj.type in {"MESH", "ARMATURE"})
    bpy.context.view_layer.objects.active = armature
    bpy.ops.export_scene.gltf(
        filepath=str(out_dir / f"{pet['id']}.glb"), export_format="GLB", use_selection=True,
        export_animations=True, export_animation_mode="ACTIONS", export_force_sampling=True,
        export_frame_step=2, export_skins=True, export_morph=False, export_def_bones=True,
        export_anim_single_armature=True, export_optimize_animation_size=True, export_extras=False,
        export_cameras=False, export_lights=False, export_yup=True,
    )
    print(json.dumps({"pet": pet["id"], "bytes": (out_dir / f"{pet['id']}.glb").stat().st_size}))
sys.stdout.flush()
os._exit(0)
