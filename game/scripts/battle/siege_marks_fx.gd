class_name SiegeMarksFx
extends Node3D

## SG2 — traces laissées par l'assaut, d'après les événements du cœur relayés par
## `SiegeAssaultFx` :
## - impacts des pierres et boulets : cratère éclaté (décalque) au point frappé du pan, plus
##   large quand le coup ouvre la brèche ; les marques d'un pan disparaissent quand il tombe
##   (l'effondrement physique S1 prend le relais) ;
## - huile bouillante : vapeur qui monte du pied de la porte, coulures le long du parement,
##   parement mouillé et flaque qui sèchent en `stain_seconds`, grésillement (`boiling_oil`).
## Réglages : `data/fx/siege_engines.json` (`oil`, `impact_marks`). Rendu seulement.

var cfg: Dictionary = {}
var marks_placed := 0  # tests et captures
var _marks: Array = []  # [{node: Decal, piece: int}]
var _stains: Array = []  # [{node: Decal, born: float, life: float}]
var _steam: GPUParticles3D
var _drips: GPUParticles3D
var _crater_tex: ImageTexture
var _stain_tex: ImageTexture
var _puddle_tex: ImageTexture
var time_now := 0.0


func setup(p_cfg: Dictionary) -> void:
	cfg = p_cfg
	_crater_tex = _make_crater(1356)
	_stain_tex = _make_stain(7)
	_puddle_tex = _make_puddle(11)
	_steam = _make_steam()
	_drips = _make_drips()


func update(now: float) -> void:
	time_now = now
	var done: Array = []
	for s in _stains:
		var age := (now - float(s["born"])) / maxf(float(s["life"]), 0.1)
		if age >= 1.0:
			done.append(s)
			continue
		(s["node"] as Decal).modulate.a = 1.0 - smoothstep(0.4, 1.0, age)
	for s in done:
		(s["node"] as Node).queue_free()
		_stains.erase(s)


# --- Impacts ---------------------------------------------------------------------------


## Cratère sur le pan `piece` au point `pos` (face extérieure, normale `out`).
func mark_impact(piece: int, pos: Vector3, out: Vector3, breached: bool) -> void:
	var c: Dictionary = cfg.get("impact_marks", {})
	var max_marks := int(c.get("max_marks", 64))
	if max_marks <= 0:
		return
	var sizes: Array = c.get("size_m", [1.8, 3.0])
	var h := fposmod(sin(float(marks_placed) * 91.7 + pos.x * 0.37) * 43758.55, 1.0)
	var size := lerpf(float(sizes[0]), float(sizes[1]), h) * (float(c.get("breach_scale", 1.5)) if breached else 1.0)
	var decal := Decal.new()
	decal.texture_albedo = _crater_tex
	decal.size = Vector3(size, float(c.get("depth_m", 2.0)), size)
	decal.transform = Transform3D(_wall_basis(out, h * TAU), pos)
	decal.albedo_mix = 1.0
	decal.upper_fade = 0.1
	decal.lower_fade = 0.1
	decal.cull_mask = 1
	add_child(decal)
	_marks.append({"node": decal, "piece": piece})
	marks_placed += 1
	while _marks.size() > max_marks:
		(_marks.pop_front()["node"] as Node).queue_free()


## Le pan est tombé : ses marques partent avec lui.
func clear_piece(piece: int) -> void:
	var kept: Array = []
	for m in _marks:
		if int(m["piece"]) == piece:
			(m["node"] as Node).queue_free()
		else:
			kept.append(m)
	_marks = kept


func mark_count() -> int:
	return _marks.size()


## Repère d'un décalque plaqué sur un mur de normale extérieure `out` (projection le long de
## −Y local, donc +Y = `out`), tourné de `spin` dans le plan du mur.
static func _wall_basis(out: Vector3, spin: float) -> Basis:
	var y := out.normalized()
	var x := Vector3.UP.cross(y).normalized()
	if x.length() < 0.5:
		x = Vector3.RIGHT
	var z := x.cross(y).normalized()
	return Basis(x, y, z) * Basis(Vector3.UP, spin)


# --- Huile bouillante ------------------------------------------------------------------


## Pot d'huile versé du haut de la porte (`top`, face `out`, largeur `width`) sur `ground`.
func pour_oil(top: Vector3, ground: Vector3, out: Vector3, width: float, wall_height: float) -> void:
	var c: Dictionary = cfg.get("oil", {})
	var delay := float(c.get("sound_delay_s", 0.9))
	BattleAudio.play_at("boiling_oil", top)
	_drips.transform = Transform3D(Basis.looking_at(-out, Vector3.UP), top + out * 0.15)
	_drips.restart()
	_drips.emitting = true
	get_tree().create_timer(delay).timeout.connect(func() -> void:
		if is_instance_valid(_steam):
			_steam.position = ground
			_steam.restart()
			_steam.emitting = true)
	var life := float(c.get("stain_seconds", 40.0))
	# Parement mouillé sous les mâchicoulis.
	var stain := Decal.new()
	stain.texture_albedo = _stain_tex
	stain.size = Vector3(float(c.get("stain_width_m", 3.2)), 1.6, wall_height)
	var basis := _wall_basis(out, 0.0)
	stain.transform = Transform3D(basis, top - Vector3(0, wall_height * 0.5, 0) + out * 0.2)
	stain.cull_mask = 1
	add_child(stain)
	_stains.append({"node": stain, "born": time_now, "life": life})
	# Flaque fumante au pied de la porte.
	var puddle := Decal.new()
	puddle.texture_albedo = _puddle_tex
	puddle.size = Vector3(width * 0.8, 2.0, 5.0)
	puddle.transform = Transform3D(Basis(Vector3.UP, atan2(out.x, out.z)), ground)
	puddle.cull_mask = 1
	add_child(puddle)
	_stains.append({"node": puddle, "born": time_now, "life": life})


func _make_steam() -> GPUParticles3D:
	var c: Dictionary = cfg.get("oil", {})
	var particles := GPUParticles3D.new()
	particles.name = "OilSteam"
	particles.amount = int(c.get("steam_amount", 90))
	particles.lifetime = float(c.get("steam_lifetime_s", 3.2))
	particles.one_shot = true
	particles.explosiveness = 0.6
	particles.emitting = false
	particles.local_coords = false
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	process.emission_box_extents = Vector3(3.0, 0.2, 2.5)
	process.direction = Vector3(0, 1, 0)
	process.spread = 25.0
	process.initial_velocity_min = 0.6
	process.initial_velocity_max = float(c.get("steam_rise", 2.4))
	process.gravity = Vector3(0.3, 0.8, 0)
	process.damping_min = 0.4
	process.damping_max = 0.9
	process.scale_min = 0.6
	process.scale_max = 1.3
	var curve := Curve.new()
	curve.add_point(Vector2(0.0, 0.4))
	curve.add_point(Vector2(1.0, 2.2))
	var scale_tex := CurveTexture.new()
	scale_tex.curve = curve
	process.scale_curve = scale_tex
	var ramp := Gradient.new()
	ramp.set_color(0, Color(1, 1, 1, 0.55))
	ramp.set_color(1, Color(1, 1, 1, 0.0))
	var ramp_tex := GradientTexture1D.new()
	ramp_tex.gradient = ramp
	process.color_ramp = ramp_tex
	particles.process_material = process
	var quad := QuadMesh.new()
	var size := float(c.get("steam_size_m", 2.4))
	quad.size = Vector2(size, size)
	var mat := StandardMaterial3D.new()
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	mat.billboard_keep_scale = true
	mat.vertex_color_use_as_albedo = true
	mat.albedo_texture = BattleEffects._puff_texture()
	mat.albedo_color = Color(0.93, 0.92, 0.9, 0.75)
	mat.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_DISABLED
	quad.material = mat
	particles.draw_pass_1 = quad
	particles.visibility_aabb = AABB(Vector3(-10, -2, -10), Vector3(20, 16, 20))
	add_child(particles)
	return particles


## Coulures : filets sombres et luisants qui glissent le long du parement sous le pot.
func _make_drips() -> GPUParticles3D:
	var c: Dictionary = cfg.get("oil", {})
	var particles := GPUParticles3D.new()
	particles.name = "OilDrips"
	particles.amount = int(c.get("drip_amount", 48))
	particles.lifetime = float(c.get("drip_lifetime_s", 1.4))
	particles.one_shot = true
	particles.explosiveness = 0.35
	particles.emitting = false
	particles.local_coords = false
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	process.emission_box_extents = Vector3(1.6, 0.1, 0.05)
	process.direction = Vector3(0, -1, 0)
	process.spread = 2.0
	process.initial_velocity_min = 0.5
	process.initial_velocity_max = 1.5
	process.gravity = Vector3(0, -4.0, 0)
	process.scale_min = 0.7
	process.scale_max = 1.4
	var curve := Curve.new()
	curve.add_point(Vector2(0.0, 0.3))
	curve.add_point(Vector2(0.3, 1.0))
	curve.add_point(Vector2(1.0, 1.4))
	var scale_tex := CurveTexture.new()
	scale_tex.curve = curve
	process.scale_curve = scale_tex
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2(0.12, 1.1)
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.2, 0.12, 0.04)
	mat.metallic_specular = 0.8
	mat.roughness = 0.2
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y
	mat.billboard_keep_scale = true
	quad.material = mat
	particles.draw_pass_1 = quad
	particles.visibility_aabb = AABB(Vector3(-6, -14, -6), Vector3(12, 16, 12))
	add_child(particles)
	return particles


# --- Textures procédurales -------------------------------------------------------------


## Cratère : cœur sombre et poudreux, auréole de pierre éclatée, fissures rayonnantes.
static func _make_crater(seed_value: int) -> ImageTexture:
	var size := 128
	var img := Image.create(size, size, false, Image.FORMAT_RGBA8)
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	var rays: Array = []
	for i in 9:
		rays.append([rng.randf() * TAU, rng.randf_range(0.55, 1.0), rng.randf_range(0.02, 0.05)])
	for y in size:
		for x in size:
			var p := (Vector2(x, y) / float(size - 1)) * 2.0 - Vector2.ONE
			var r := p.length()
			var angle := atan2(p.y, p.x)
			var n := rng.randf() * 0.15
			var core := 1.0 - smoothstep(0.18, 0.42 + n, r)
			var halo := (1.0 - smoothstep(0.4, 0.75, r)) * 0.55
			var crack := 0.0
			for ray in rays:
				var da := absf(wrapf(angle - float(ray[0]), -PI, PI))
				if r < float(ray[1]) and da < float(ray[2]) * (1.2 - r):
					crack = maxf(crack, 0.8 * (1.0 - r / float(ray[1])))
			var alpha := clampf(maxf(maxf(core, halo), crack), 0.0, 1.0)
			var shade := lerpf(0.33, 0.12, core) - crack * 0.08
			img.set_pixel(x, y, Color(shade, shade * 0.96, shade * 0.9, alpha))
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


## Parement mouillé : bande sombre en haut, filets qui s'allongent vers le bas.
static func _make_stain(seed_value: int) -> ImageTexture:
	var w := 64
	var h := 128
	var img := Image.create(w, h, false, Image.FORMAT_RGBA8)
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	var lengths: Array = []
	for x in w:
		lengths.append(rng.randf_range(0.25, 0.95) if rng.randf() < 0.45 else rng.randf_range(0.05, 0.3))
	for y in h:
		var v := float(y) / float(h - 1)  # 0 = haut du mur
		for x in w:
			var edge := 1.0 - smoothstep(0.7, 1.0, absf(float(x) / float(w - 1) * 2.0 - 1.0))
			var reach := float(lengths[x])
			var a := (1.0 - smoothstep(reach * 0.8, reach, v)) * edge * 0.75
			img.set_pixel(x, y, Color(0.09, 0.06, 0.03, a))
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)


## Flaque d'huile : tache sombre aux bords irréguliers.
static func _make_puddle(seed_value: int) -> ImageTexture:
	var size := 64
	var img := Image.create(size, size, false, Image.FORMAT_RGBA8)
	var noise := FastNoiseLite.new()
	noise.seed = seed_value
	noise.frequency = 0.08
	for y in size:
		for x in size:
			var p := (Vector2(x, y) / float(size - 1)) * 2.0 - Vector2.ONE
			var r := p.length() + noise.get_noise_2d(x, y) * 0.35
			var a := (1.0 - smoothstep(0.55, 0.85, r)) * 0.8
			img.set_pixel(x, y, Color(0.07, 0.05, 0.03, a))
	img.generate_mipmaps()
	return ImageTexture.create_from_image(img)
