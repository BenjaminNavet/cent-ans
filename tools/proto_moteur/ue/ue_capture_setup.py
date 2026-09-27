"""(Runs inside the Unreal editor.) Offscreen capture: a SceneCapture2D at ProtoCamera into a render target."""

import json

import unreal

HERE = "/Users/jean_hubert/dev/game_project/tools/proto_moteur"
scene = json.load(open(f"{HERE}/scene.json"))
width, height = (2 * v for v in scene["resolution"])  # 2x supersampling, reduced by exr_to_png.py
actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
level_actors = actors.get_all_level_actors()
for actor in level_actors:
    if actor.get_actor_label() == "ProtoCapture":
        actors.destroy_actor(actor)
camera = next(a for a in level_actors if a.get_actor_label() == "ProtoCamera")
world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
target = unreal.RenderingLibrary.create_render_target2d(world, width, height, unreal.TextureRenderTargetFormat.RTF_RGBA16F)
capture_actor = actors.spawn_actor_from_class(unreal.SceneCapture2D, camera.get_actor_location(), camera.get_actor_rotation())
capture_actor.set_actor_label("ProtoCapture")
capture = capture_actor.get_component_by_class(unreal.SceneCaptureComponent2D)
capture.set_editor_property("texture_target", target)
capture.set_editor_property("capture_source", unreal.SceneCaptureSource.SCS_FINAL_TONE_CURVE_HDR)
capture.set_editor_property("fov_angle", camera.get_component_by_class(unreal.CameraComponent).get_editor_property("field_of_view"))
capture.set_editor_property("capture_every_frame", True)
capture.set_editor_property("always_persist_rendering_state", True)
unreal.PROTO_TARGET = target  # kept alive for ue_capture_save.py
print("PROTO capture ready")
