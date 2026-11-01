import bpy,sys,os,json
from mathutils import Matrix
source,out=sys.argv[1:3]
bpy.ops.wm.open_mainfile(filepath=source)
arm=next(o for o in bpy.data.objects if o.type=='ARMATURE')
# Keep original authored bind pose, then turn rigid bone-parented robot panels
# into conventional one-bone skinned parts for the strict uploader contract.
arm.animation_data_clear()
arm.data.pose_position='REST';bpy.context.view_layer.update()
for obj in list(bpy.data.objects):
 if obj.type not in {'MESH','ARMATURE'}:bpy.data.objects.remove(obj,do_unlink=True);continue
 if obj.type=='MESH' and obj.parent_type=='BONE':
  bone=obj.parent_bone;world=obj.matrix_world.copy();obj.parent=arm;obj.parent_type='OBJECT';obj.matrix_parent_inverse=Matrix.Identity(4);obj.matrix_world=world
  group=obj.vertex_groups.get(bone) or obj.vertex_groups.new(name=bone);group.add(list(range(len(obj.data.vertices))),1,'REPLACE')
  mod=obj.modifiers.new('Catalog skin','ARMATURE');mod.object=arm
weighted={obj.vertex_groups[weight.group].name for obj in bpy.data.objects if obj.type=='MESH' for vertex in obj.data.vertices for weight in vertex.groups if weight.weight>1e-6}
for bone in arm.data.bones:bone.use_deform=bone.name in weighted
# Source older Blender files contain flat material colors and a stale unused
# Texture.png reference. Use their authored color values, with standard PBR.
for mat in bpy.data.materials:
 color=tuple(mat.diffuse_color);mat.use_nodes=True;nodes=mat.node_tree.nodes;nodes.clear();p=nodes.new('ShaderNodeBsdfPrincipled');p.inputs['Base Color'].default_value=color;p.inputs['Metallic'].default_value=0;p.inputs['Roughness'].default_value=.75;o=nodes.new('ShaderNodeOutputMaterial');mat.node_tree.links.new(p.outputs['BSDF'],o.inputs['Surface'])
robot='RobotArmature'==arm.name
wanted={'Robot_Idle':'Idle','Robot_Running':'Run','Robot_Dance':'Victory'} if robot else {x:x for x in ['Idle','Run','Victory','Defeat','Death']}
for action in list(bpy.data.actions):
 if action.name not in wanted:bpy.data.actions.remove(action)
 else:action.name=wanted[action.name]
arm.data.pose_position='POSE';arm.animation_data_create();arm.animation_data.action=bpy.data.actions.get('Idle');bpy.context.scene.frame_set(0);bpy.context.view_layer.update()
for obj in bpy.context.scene.objects:obj.select_set(obj.type in {'MESH','ARMATURE'})
bpy.context.view_layer.objects.active=arm
bpy.ops.export_scene.gltf(filepath=out,export_format='GLB',use_selection=True,export_animations=True,export_animation_mode='ACTIONS',export_force_sampling=True,export_frame_step=1,export_skins=True,export_morph=False,export_def_bones=True,export_anim_single_armature=True,export_optimize_animation_size=True,export_extras=False,export_cameras=False,export_lights=False,export_yup=True)
print(json.dumps({'output':out,'bytes':os.path.getsize(out),'bones':len(arm.data.bones),'actions':[a.name for a in bpy.data.actions]}));sys.stdout.flush();os._exit(0)
