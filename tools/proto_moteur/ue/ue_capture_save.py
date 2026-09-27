"""(Runs inside the Unreal editor.) Render ProtoCapture now, repeatedly, and save it to out/unreal.exr."""

import unreal

HERE = "/Users/jean_hubert/dev/game_project/tools/proto_moteur"
FRAMES = 64  # repeated captures let Lumen and temporal AA accumulate with the persistent view state
actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
capture_actor = next(a for a in actors.get_all_level_actors() if a.get_actor_label() == "ProtoCapture")
capture = capture_actor.get_component_by_class(unreal.SceneCaptureComponent2D)
for _ in range(FRAMES):
    capture.capture_scene()
world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
unreal.RenderingLibrary.export_render_target(world, capture.get_editor_property("texture_target"), f"{HERE}/out", "unreal.exr")
print("PROTO capture exported")
