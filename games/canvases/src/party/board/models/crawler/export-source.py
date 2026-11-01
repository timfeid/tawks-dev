"""Convert Quaternius CC0 Zombie_Male; author a looping prone crawl on its original skeleton."""
import bpy,math,sys,os
from mathutils import Matrix
src,out=sys.argv[sys.argv.index("--")+1:][:2]
bpy.ops.wm.open_mainfile(filepath=src)
a=next(o for o in bpy.data.objects if o.type=='ARMATURE')
# Start from the source's rigged idle pose; author a deliberate hands-and-knees
# crawl, with opposing hand/knee reaches and the head raised toward travel.
a.animation_data.action=bpy.data.actions['Idle']
bpy.context.scene.frame_set(0);bpy.context.view_layer.update()
base={b.name:b.matrix.copy() for b in a.pose.bones}
for b in a.pose.bones:
 for c in list(b.constraints):b.constraints.remove(c)
a.animation_data_clear()
for act in list(bpy.data.actions):bpy.data.actions.remove(act)
a.animation_data_create();a.animation_data.action=bpy.data.actions.new('Crawl')
from mathutils import Vector,Quaternion
def point_bone(name,direction):
 b=a.pose.bones[name];old=(b.tail-b.head).normalized();desired=Vector(direction).normalized()
 q=old.rotation_difference(desired);head=b.head.copy()
 b.matrix=Matrix.Translation(head)@q.to_matrix().to_4x4()@Matrix.Translation(-head)@b.matrix
 bpy.context.view_layer.update()
for f in range(49):
 bpy.context.scene.frame_set(f)
 for b in a.pose.bones:b.matrix=base[b.name]
 bpy.context.view_layer.update()
 phase=math.tau*f/48
 for side,sign in [('L',1),('R',-1)]:
  reach=math.sin(phase)*sign
  point_bone('UpperArm.'+side,(.10*sign,-.50,.16+.16*reach))
  point_bone('LowerArm.'+side,(.03*sign,-.10,.43-.10*reach))
  point_bone('Fist.'+side,(0,-.04,.27))
  point_bone('UpperLeg.'+side,(.10*sign,-.40,-.20-.12*reach))
  point_bone('LowerLeg.'+side,(.02*sign,.25,-.45+.08*reach))
 point_bone('Head',(0,.48,.45))
 for b in a.pose.bones:
  b.rotation_mode='QUATERNION';b.keyframe_insert('location',frame=f);b.keyframe_insert('rotation_quaternion',frame=f);b.keyframe_insert('scale',frame=f)
# Prone: face and hands toward the ground, head leading +Z after glTF conversion.
a.rotation_euler=(math.pi/2,0,0)
a.scale*=.55
bpy.context.scene.frame_set(0);bpy.context.view_layer.update()
for obj in list(bpy.data.objects):
 if obj.type not in {'MESH','ARMATURE'}:bpy.data.objects.remove(obj,do_unlink=True)
# Locate the prone character just above the floor across its entire gait.
low=1e9
for f in range(49):
 bpy.context.scene.frame_set(f);bpy.context.view_layer.update()
 for obj in bpy.context.scene.objects:
  if obj.type=='MESH':
   ev=obj.evaluated_get(bpy.context.evaluated_depsgraph_get());mesh=ev.to_mesh()
   low=min(low,min((ev.matrix_world@v.co).z for v in mesh.vertices));ev.to_mesh_clear()
a.location.z-=low
for m in bpy.data.materials:
 color=tuple(m.diffuse_color);m.use_nodes=True;n=m.node_tree.nodes;n.clear();p=n.new('ShaderNodeBsdfPrincipled');p.inputs['Base Color'].default_value=color;p.inputs['Roughness'].default_value=.85;o=n.new('ShaderNodeOutputMaterial');m.node_tree.links.new(p.outputs['BSDF'],o.inputs['Surface'])
for obj in bpy.context.scene.objects:obj.select_set(obj.type in {'MESH','ARMATURE'})
bpy.context.view_layer.objects.active=a;bpy.context.scene.frame_set(0)
bpy.ops.export_scene.gltf(filepath=out,export_format='GLB',use_selection=True,export_animations=True,export_animation_mode='ACTIONS',export_force_sampling=True,export_skins=True,export_def_bones=True,export_anim_single_armature=True,export_extras=False,export_cameras=False,export_lights=False,export_yup=True)
print('Exported',out,os.path.getsize(out),flush=True);os._exit(0)
