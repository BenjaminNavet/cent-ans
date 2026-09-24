class_name BattleAtmosphere
extends RefCounted

## Ciel, lumière et météo des batailles (lot V4) : ciel procédural (`battle_sky.gdshader`),
## tonemapping AgX, SSAO, halo, brouillard de distance avec perspective aérienne (l'horizon se
## fond dans le ciel), étalonnage léger, soleil et ombres réglés ; pluie et neige en particules
## GPU accrochées à la caméra. Un préréglage par temps de la simulation (clear, fog, rain, snow).
## Lot V3 (A1-05, A1-14) : ciel HDRI Poly Haven et LUT d'étalonnage par météo et par saison
## (`AtmosphereLibrary`, `data/fx/atmosphere.json`) — le ciel procédural reste le repli ; brouillard
## volumétrique (nappes basses par temps de brouillard) et effets réglés par `RenderQuality`.

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")

## Préréglages : ciel (zénith, horizon, nuages), soleil (énergie, couleur, élévation), brouillard.
const PRESETS := {
	"clear": {
		"zenith": Color(0.2, 0.38, 0.68), "horizon": Color(0.7, 0.78, 0.86), "coverage": 0.38,
		"cloud_light": Color(1.0, 0.98, 0.94), "cloud_shadow": Color(0.56, 0.61, 0.7),
		"sun": 1.75, "sun_color": Color(1.0, 0.94, 0.84), "elevation": 34.0, "sky_energy": 1.0,
		"fog": 0.00032, "fog_color": Color(0.7, 0.76, 0.84), "aerial": 0.65, "ambient": 1.0,
		"saturation": 1.08, "contrast": 1.06,
	},
	"fog": {
		"zenith": Color(0.62, 0.65, 0.68), "horizon": Color(0.78, 0.8, 0.8), "coverage": 0.92,
		"cloud_light": Color(0.86, 0.87, 0.87), "cloud_shadow": Color(0.66, 0.68, 0.7),
		"sun": 0.5, "sun_color": Color(1.0, 0.97, 0.92), "elevation": 26.0, "sky_energy": 0.9,
		"fog": 0.0036, "fog_color": Color(0.74, 0.76, 0.77), "aerial": 0.0, "ambient": 1.1,
		"saturation": 0.85, "contrast": 1.0,
	},
	"rain": {
		"zenith": Color(0.42, 0.45, 0.5), "horizon": Color(0.6, 0.63, 0.66), "coverage": 0.97,
		"cloud_light": Color(0.62, 0.64, 0.67), "cloud_shadow": Color(0.36, 0.38, 0.42),
		"sun": 0.55, "sun_color": Color(0.9, 0.93, 1.0), "elevation": 40.0, "sky_energy": 1.0,
		"fog": 0.0014, "fog_color": Color(0.52, 0.55, 0.59), "aerial": 0.2, "ambient": 1.15,
		"saturation": 0.8, "contrast": 1.04,
	},
	"snow": {
		"zenith": Color(0.58, 0.63, 0.7), "horizon": Color(0.8, 0.83, 0.87), "coverage": 0.88,
		"cloud_light": Color(0.92, 0.93, 0.95), "cloud_shadow": Color(0.66, 0.69, 0.74),
		"sun": 0.6, "sun_color": Color(0.95, 0.96, 1.0), "elevation": 22.0, "sky_energy": 0.95,
		"fog": 0.0017, "fog_color": Color(0.8, 0.83, 0.87), "aerial": 0.2, "ambient": 1.05,
		"saturation": 0.9, "contrast": 1.02,
	},
}


## Soleil plafonné (ombres longues lisibles, faces éclairées vues depuis la caméra par défaut).
const MAX_SUN_ELEVATION := 36.0
const HDRI_AMBIENT_BOOST := 1.35

## Densité du brouillard volumétrique par temps (actif selon le niveau de `RenderQuality`).
const VOLUMETRIC_DENSITY := {"clear": 0.0012, "fog": 0.004, "rain": 0.0035, "snow": 0.004}


## Applique le préréglage `key` ; `camera` reçoit les précipitations éventuelles ; `season`
## (spring, summer, autumn, winter) choisit le ciel HDRI et l'étalonnage de saison.
static func apply(world_env: WorldEnvironment, sun: DirectionalLight3D, key: String, camera: Camera3D, season: String = "summer") -> void:
	var p: Dictionary = (PRESETS.get(key, PRESETS["clear"]) as Dictionary).duplicate()
	# La ressource de la scène est partagée entre instances : on travaille sur une copie.
	var env: Environment = world_env.environment.duplicate() as Environment
	world_env.environment = env
	var sky_mat := ShaderMaterial.new()
	sky_mat.shader = SKY_SHADER
	sky_mat.set_shader_parameter("zenith_color", p["zenith"])
	sky_mat.set_shader_parameter("horizon_color", p["horizon"])
	sky_mat.set_shader_parameter("ground_color", (p["horizon"] as Color).darkened(0.45))
	sky_mat.set_shader_parameter("cloud_coverage", p["coverage"])
	sky_mat.set_shader_parameter("cloud_light", p["cloud_light"])
	sky_mat.set_shader_parameter("cloud_shadow", p["cloud_shadow"])
	sky_mat.set_shader_parameter("sun_disc", 1.0 if key == "clear" else 0.0)
	sky_mat.set_shader_parameter("exposure", p["sky_energy"])
	var sky := Sky.new()
	sky.sky_material = sky_mat
	sky.radiance_size = Sky.RADIANCE_SIZE_256
	env.sky = sky
	env.background_mode = Environment.BG_SKY
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = p["ambient"]
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	env.tonemap_exposure = 1.0
	env.ssao_enabled = true
	env.ssao_radius = 1.6
	env.ssao_intensity = 1.8
	env.ssao_power = 1.4
	env.ssao_detail = 0.6
	env.ssao_light_affect = 0.05
	env.glow_enabled = true
	env.glow_intensity = 0.35
	env.glow_bloom = 0.03
	env.glow_hdr_threshold = 1.1
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_EXPONENTIAL
	env.fog_density = p["fog"]
	env.fog_light_color = p["fog_color"]
	env.fog_light_energy = 1.0
	env.fog_aerial_perspective = p["aerial"]
	env.fog_sky_affect = 0.35 if key == "clear" else 0.8
	env.fog_sun_scatter = 0.15 if key == "clear" else 0.0
	if key == "fog":
		# Nappes basses dans les creux.
		env.fog_height = 12.0
		env.fog_height_density = 0.04
	env.adjustment_enabled = true
	env.adjustment_brightness = 1.0
	env.adjustment_contrast = p["contrast"]
	env.adjustment_saturation = p["saturation"]
	# V3 : ciel HDRI et étalonnage (LUT) ; contraste et saturation passent dans la LUT.
	var look := AtmosphereLibrary.battle_look(key, season)
	if not look.is_empty():
		AtmosphereLibrary.apply_to_environment(env, look, p["fog_color"], (p["horizon"] as Color).darkened(0.45))
		if (env.sky.sky_material as ShaderMaterial).shader != SKY_SHADER:
			env.adjustment_contrast = 1.0
			env.adjustment_saturation = 1.0
			p["elevation"] = minf(AtmosphereLibrary.sun_elevation(look, float(p["elevation"])), MAX_SUN_ELEVATION)
			# Le ciel HDRI est plus sombre et plus bleu côté opposé au soleil que le ciel
			# procédural : une part d'ambiance neutre (couleur du brouillard) éclaire les faces à
			# l'ombre sans les bleuir.
			env.ambient_light_sky_contribution = 0.7
			env.ambient_light_color = (p["fog_color"] as Color).lightened(0.15)
			env.ambient_light_energy = float(p["ambient"]) * HDRI_AMBIENT_BOOST
	_setup_volumetric_fog(env, key, p)
	_setup_sun(sun, p)
	RenderQuality.register(world_env, sun, "battle", key)
	match key:
		"rain":
			_add_precipitation(camera, true)
		"snow":
			_add_precipitation(camera, false)


## Brouillard volumétrique (V3, A1-14) : voile léger qui accroche la lumière (rayons), plus dense
## par mauvais temps ; par temps de brouillard, nappe basse (`FogVolume`) sur tout le champ.
## Activé ou non par `RenderQuality` (coût GPU) ; réglé ici quel que soit le niveau.
static func _setup_volumetric_fog(env: Environment, key: String, p: Dictionary) -> void:
	env.volumetric_fog_density = float(VOLUMETRIC_DENSITY.get(key, 0.0015))
	env.volumetric_fog_albedo = p["fog_color"]
	env.volumetric_fog_emission = Color.BLACK
	env.volumetric_fog_anisotropy = 0.6 if key == "clear" else 0.25
	env.volumetric_fog_length = 320.0
	env.volumetric_fog_detail_spread = 2.0
	env.volumetric_fog_gi_inject = 0.0
	env.volumetric_fog_ambient_inject = 0.35
	env.volumetric_fog_sky_affect = 0.0
	env.volumetric_fog_temporal_reprojection_enabled = true


## Nappe de brouillard au ras du sol (temps « fog ») : `FogVolume` couvrant le champ.
static func add_ground_mist(parent: Node3D, key: String, center: Vector3, size: Vector2) -> void:
	if key != "fog" or parent == null:
		return
	var volume := FogVolume.new()
	volume.name = "GroundMist"
	volume.shape = RenderingServer.FOG_VOLUME_SHAPE_BOX
	volume.size = Vector3(size.x, 12.0, size.y)
	volume.position = center + Vector3(0, 1.0, 0)
	var material := FogMaterial.new()
	material.density = 0.01
	material.albedo = (PRESETS["fog"]["fog_color"] as Color).lightened(0.1)
	material.height_falloff = 0.35
	material.edge_fade = 0.3
	volume.material = material
	parent.add_child(volume)


## Soleil : élévation du préréglage, venant du sud-ouest ; ombres en 4 cascades.
static func _setup_sun(sun: DirectionalLight3D, p: Dictionary) -> void:
	sun.rotation = Vector3(deg_to_rad(-float(p["elevation"])), deg_to_rad(-142.0), 0.0)
	sun.light_energy = p["sun"]
	sun.light_color = p["sun_color"]
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 420.0
	sun.directional_shadow_split_1 = 0.07
	sun.directional_shadow_split_2 = 0.18
	sun.directional_shadow_split_3 = 0.42
	sun.directional_shadow_blend_splits = true
	sun.directional_shadow_fade_start = 0.85
	sun.shadow_bias = 0.03
	sun.shadow_normal_bias = 1.4
	sun.shadow_opacity = 1.0 if float(p["sun"]) > 1.0 else 0.6
	sun.light_angular_distance = 0.0


## Pluie (traits rapides) ou neige (flocons lents qui dérivent) autour de la caméra.
static func _add_precipitation(camera: Camera3D, rain: bool) -> void:
	if camera == null:
		return
	var particles := GPUParticles3D.new()
	particles.name = "Precipitation"
	particles.amount = 9000 if rain else 7000
	particles.lifetime = 1.4 if rain else 7.0
	particles.preprocess = particles.lifetime
	particles.local_coords = false
	particles.visibility_aabb = AABB(Vector3(-90, -120, -90), Vector3(180, 180, 180))
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	process.emission_box_extents = Vector3(70, 4, 70)
	process.direction = Vector3(0.15, -1, 0.05) if rain else Vector3(0.3, -1, 0.1)
	process.spread = 4.0 if rain else 25.0
	process.initial_velocity_min = 26.0 if rain else 1.2
	process.initial_velocity_max = 32.0 if rain else 2.2
	process.gravity = Vector3(0, -9.8, 0) if rain else Vector3(0.25, -0.6, 0.1)
	if not rain:
		process.turbulence_enabled = true
		process.turbulence_noise_strength = 1.2
		process.turbulence_noise_scale = 6.0
	process.particle_flag_align_y = rain
	particles.process_material = process
	var mesh := QuadMesh.new()
	mesh.size = Vector2(0.012, 0.7) if rain else Vector2(0.08, 0.08)
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.albedo_color = Color(0.75, 0.8, 0.88, 0.16) if rain else Color(1, 1, 1, 0.85)
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y if rain else BaseMaterial3D.BILLBOARD_ENABLED
	mat.billboard_keep_scale = true
	mesh.material = mat
	particles.draw_pass_1 = mesh
	particles.position = Vector3(0, 30, -45)
	camera.add_child(particles)
