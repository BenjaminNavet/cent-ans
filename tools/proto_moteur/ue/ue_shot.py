"""(Runs inside the Unreal editor.) HighResShot of the level viewport piloting ProtoCamera."""

import json

import unreal

HERE = "/Users/jean_hubert/dev/game_project/tools/proto_moteur"
scene = json.load(open(f"{HERE}/scene.json"))
actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
level = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
camera = next(a for a in actors.get_all_level_actors() if a.get_actor_label() == "ProtoCamera")
level.pilot_level_actor(camera)
level.editor_set_game_view(True)
width, height = scene["resolution"]
world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
unreal.SystemLibrary.execute_console_command(world, f'HighResShot {width}x{height} filename="{HERE}/out/unreal.png"')
print("PROTO HighResShot requested")
