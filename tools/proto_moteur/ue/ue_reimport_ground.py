"""(Runs inside the Unreal editor.) Drop the imported ground so ue_import.py imports the new one."""

import unreal

unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).new_level("/Temp/Untitled")
unreal.EditorAssetLibrary.delete_directory("/Game/Proto/ground")
print("PROTO ground dropped")
