"""(Runs inside the Unreal editor.) Build the prototype level from scene.json with Lumen lighting."""

import json

import unreal

HERE = "/Users/jean_hubert/dev/game_project/tools/proto_moteur"
MAP = "/Game/Proto/ProtoMap"
scene = json.load(open(f"{HERE}/scene.json"))
# glTF/Godot (x, y, z) metres -> Unreal centimetres; axis mapping checked on asymmetric meshes.
AXIS = json.load(open(f"{HERE}/ue/axis.json"))


def to_ue(position):
    """Map a Godot/glTF position (metres) to an Unreal location (centimetres)."""
    x, y, z = position
    values = {"x": x, "y": y, "z": z, "-x": -x, "-y": -y, "-z": -z}
    return unreal.Vector(*(values[axis] * 100.0 for axis in AXIS["location"]))


def mesh_for(asset, node):
    """Return the imported StaticMesh of an asset (the one matching node, when given)."""
    for path in unreal.EditorAssetLibrary.list_assets(f"/Game/Proto/{asset}", recursive=True):
        loaded = unreal.EditorAssetLibrary.load_asset(path)
        if isinstance(loaded, unreal.StaticMesh) and (not node or node in loaded.get_name()):
            return loaded
    raise RuntimeError(f"no mesh for {asset}/{node}")


level = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
if unreal.EditorAssetLibrary.does_asset_exist(MAP):
    level.load_level(MAP)
    actors.destroy_actors(actors.get_all_level_actors())
else:
    level.new_level(MAP)
cache = {}
for placement in scene["placements"]:
    key = (placement["asset"], placement["node"])
    if key not in cache:
        cache[key] = mesh_for(*key)
    rotation = unreal.Rotator(0.0, 0.0, AXIS["yaw_sign"] * placement["yaw"] + AXIS["yaw_offset"])
    actor = actors.spawn_actor_from_object(cache[key], to_ue(placement["pos"]), rotation)
    actor.set_actor_scale3d(unreal.Vector(1, 1, 1) * placement["scale"])
    actor.set_folder_path(placement["asset"])

sun_spec = scene["sun"]
sun = actors.spawn_actor_from_class(unreal.DirectionalLight, unreal.Vector(0, 0, 1000))
sun.set_actor_rotation(unreal.Rotator(0.0, -sun_spec["elevation"], AXIS["sun_yaw_offset"] + sun_spec["azimuth"]), False)
light = sun.get_component_by_class(unreal.DirectionalLightComponent)
light.set_editor_property("intensity", 10.0)
light.set_editor_property("light_color", unreal.Color(*(int(c * 255) for c in sun_spec["color"]), 255))
light.set_editor_property("atmosphere_sun_light", True)
light.set_editor_property("light_source_angle", 0.6)
light.set_editor_property("volumetric_scattering_intensity", 1.5)

actors.spawn_actor_from_class(unreal.SkyAtmosphere, unreal.Vector(0, 0, 0))
sky = actors.spawn_actor_from_class(unreal.SkyLight, unreal.Vector(0, 0, 500))
sky_component = sky.get_component_by_class(unreal.SkyLightComponent)
sky_component.set_editor_property("real_time_capture", True)
sky_component.set_editor_property("mobility", unreal.ComponentMobility.MOVABLE)
actors.spawn_actor_from_class(unreal.VolumetricCloud, unreal.Vector(0, 0, 0))
fog_actor = actors.spawn_actor_from_class(unreal.ExponentialHeightFog, unreal.Vector(0, 0, 0))
fog = fog_actor.get_component_by_class(unreal.ExponentialHeightFogComponent)
fog.set_editor_property("fog_density", scene["fog"]["density"] * 4.0)
fog.set_editor_property("fog_height_falloff", 0.2)
fog.set_editor_property("volumetric_fog_extinction_scale", 2.0)
fog.set_editor_property("enable_volumetric_fog", True)
fog.set_editor_property("volumetric_fog_scattering_distribution", 0.6)
fog.set_editor_property("volumetric_fog_albedo", unreal.Color(*(int(c * 255) for c in scene["fog"]["color"]), 255))

post = actors.spawn_actor_from_class(unreal.PostProcessVolume, unreal.Vector(0, 0, 0))
post.set_editor_property("unbound", True)

cam_spec = scene["camera"]
eye = to_ue(cam_spec["pos"])
target = to_ue(cam_spec["look_at"])
camera = actors.spawn_actor_from_class(unreal.CameraActor, eye, unreal.MathLibrary.find_look_at_rotation(eye, target))
camera.set_actor_label("ProtoCamera")
width, height = scene["resolution"]
camera_component = camera.get_component_by_class(unreal.CameraComponent)
camera_component.set_editor_property("aspect_ratio", width / height)
# Unreal's field of view is horizontal; scene.json gives the vertical one.
import math  # noqa: E402

fov_h = math.degrees(2 * math.atan(math.tan(math.radians(cam_spec["fov_y"]) / 2) * width / height))
camera_component.set_editor_property("field_of_view", fov_h)
level.save_current_level()
level.pilot_level_actor(camera)
print(f"PROTO level built with {len(scene['placements'])} placements")
