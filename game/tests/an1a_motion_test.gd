extends SceneTree

## Lot AN1a : mouvement secondaire en shader (ADR 0096). Vérifications sans rendu :
## - données `secondary_motion` d'`atmosphere.json` lues, uniformes posés sur les matériaux
##   (os des jambes, chaîne de la queue, amplitudes, vent), éteints pour les cadavres et avec
##   `--no-an1a` ;
## - réplique CPU du tri des sommets du shader (`sm_weight`) sur les maillages des trois LOD :
##   surcots et jaques bougent (bas seulement), chausses et bas des jambes non ; caparaçon,
##   crinière et queue reconnus sur les montés.
## Usage : godot --headless --path game --script res://tests/an1a_motion_test.gd [-- --coarse-figures]

const C_LIVERY := 0
const C_CLOTH := 4
const C_ARMS := 6
const C_COAT := 10
const C_QUILT := 12

var _failures := 0


func _init() -> void:
	await process_frame
	var cfg := BattleSecondaryMotion.settings()
	_expect(not cfg.is_empty() and bool(cfg.get("enabled", false)), "secondary_motion lu et actif")
	_check_material("infantry", 0, false)
	_check_material("cavalry", 0, true)
	_check_corpse()
	_check_flag()
	# GA3-L3b : figurines générées comprises (archer_2, infantry_1, infantry_5).
	for kind_variant in [["infantry", 0], ["infantry", 4], ["archer", 0], ["cavalry", 0], ["standard", 1], ["archer", 2], ["infantry", 1], ["infantry", 5]]:
		for level in 3:
			_check_mesh(str(kind_variant[0]), int(kind_variant[1]), level)
	if _failures == 0:
		print("an1a_motion_test OK")
	quit(0 if _failures == 0 else 1)


func _expect(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("an1a_motion_test: " + message)


func _material(kind: String, variant: int) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, kind, variant)
	BattleSecondaryMotion.setup_material(mat, kind, variant)
	return mat


func _check_material(kind: String, variant: int, mounted: bool) -> void:
	var mat := _material(kind, variant)
	_expect(bool(mat.get_shader_parameter("sm_enabled")), "%s : mouvement secondaire actif" % kind)
	var legs: Vector2i = mat.get_shader_parameter("sm_leg_bones")
	_expect(legs.y - legs.x == 5, "%s : six os de jambes contigus (%s)" % [kind, legs])
	var tail: Vector2i = mat.get_shader_parameter("sm_tail_bones")
	if mounted:
		_expect(tail.y - tail.x == 6, "cheval : chaîne Tail1-7 (%s)" % tail)
		_expect(legs.x > tail.y, "cavalier : os des jambes du cavalier (préfixe R:) après ceux du cheval")
	else:
		_expect(tail.y < 0, "à pied : pas de queue")
	var amps: Array = mat.get_shader_parameter("sm_amp")
	_expect(amps.size() == 4 and (amps[0] as Vector4).x > 0.0, "amplitudes par pièce")
	_expect(float(mat.get_shader_parameter("wind_strength")) > 0.0, "vent posé")


func _check_corpse() -> void:
	var soldiers := BattleSoldiers.new()
	var mat: ShaderMaterial = soldiers._make_skinned_material("attacker", "infantry", 0, true)
	_expect(not bool(mat.get_shader_parameter("sm_enabled")), "cadavres : mouvement secondaire éteint")
	var alive: ShaderMaterial = soldiers._make_skinned_material("attacker", "infantry", 0, false)
	_expect(bool(alive.get_shader_parameter("sm_enabled")), "vivants : mouvement secondaire actif")
	soldiers.free()


func _check_flag() -> void:
	var mat := ShaderMaterial.new()
	mat.shader = BattleStandards.FLAG_SHADER
	BattleSecondaryMotion.setup_flag(mat)
	var gait: Vector4 = mat.get_shader_parameter("flag_gait_wind")
	_expect(gait.z > gait.y and gait.y > 0.0, "étendard : l'allure ajoute du vent apparent (%s)" % gait)
	_expect(float(mat.get_shader_parameter("flag_ripple")) > 0.0, "étendard : ondulation le long de la hampe")


## Réplique de `sm_weight` (shader) : nombre de sommets par pièce, et aucun sommet des chausses.
func _check_mesh(kind: String, variant: int, level: int) -> void:
	if not BattleSkinned.has_figure(kind, variant):
		return
	var mesh: ArrayMesh = BattleSkinned.mesh(kind, variant, level)
	if mesh == null:
		_expect(false, "%s_%d LOD%d : maillage absent" % [kind, variant, level])
		return
	var mat := _material(kind, variant)
	var legs: Vector2i = mat.get_shader_parameter("sm_leg_bones")
	var tail_bones: Vector2i = mat.get_shader_parameter("sm_tail_bones")
	var sever: Array = mat.get_shader_parameter("sever_bones")
	var horse_last := int((sever[0] as Vector4i).y)
	var band: Vector2 = mat.get_shader_parameter("sm_cap_band")
	var counts := [0, 0, 0, 0]
	var shin_moves := 0
	for s in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(s)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
		var bones: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
		var weights: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM1]
		for i in verts.size():
			var code := int(colors[i].a * 16.0 + 0.5)
			var part := -1
			var wt := 0.0
			if code == C_LIVERY or code == C_CLOTH or code == C_QUILT:
				var on_legs := 0.0
				for k in 4:
					var b := int(bones[i * 4 + k] + 0.5)
					if b >= legs.x and b <= legs.y:
						on_legs += weights[i * 4 + k]
				part = 0
				var hose := smoothstep(0.93, 0.99, on_legs) if code == C_CLOTH else 0.0
				wt = smoothstep(0.02, 0.85, on_legs) * (1.0 - hose)
			elif tail_bones.y >= 0 and code == C_ARMS:
				var horse := 0.0
				for k in 4:
					if int(bones[i * 4 + k] + 0.5) <= horse_last:
						horse += weights[i * 4 + k]
				if horse > 0.5:
					part = 1
					wt = 1.0 - smoothstep(band.x, band.y, verts[i].y)
			elif tail_bones.y >= 0 and code == C_COAT:
				var tail := 0.0
				for k in 4:
					var b := int(bones[i * 4 + k] + 0.5)
					if b >= tail_bones.x and b <= tail_bones.y:
						tail += weights[i * 4 + k]
				if tail > 0.5:
					part = 3
					wt = 1.0
				elif absf(roundf(colors[i].r * 255.0) / 255.0 - BattleSecondaryMotion.MANE_TINT) < 0.002:
					part = 2
					wt = 1.0
			if part >= 0 and wt > 0.001:
				counts[part] += 1
				# Chausses sous le genou (à pied) : jamais déplacées.
				if part == 0 and tail_bones.y < 0 and verts[i].y < 0.45:
					shin_moves += 1
	var label := "%s_%d LOD%d" % [kind, variant, level]
	print("AN1a %s : étoffe %d, caparaçon %d, crinière %d, queue %d" % [label, counts[0], counts[1], counts[2], counts[3]])
	_expect(shin_moves == 0, "%s : %d sommets de chausses sous le genou déplacés" % [label, shin_moves])
	if kind == "cavalry" or (kind == "standard" and variant == 1):
		_expect(counts[3] > 0, "%s : queue reconnue" % label)
		if BattleSkinned.fine_enabled():
			_expect(counts[2] > 0, "%s : crinière reconnue" % label)
	if kind == "infantry" and variant == 0:
		_expect(counts[0] > 0, "%s : bas du surcot reconnu" % label)
