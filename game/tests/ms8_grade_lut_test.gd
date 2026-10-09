extends TestCase

## MS8 : la LUT d'étalonnage cuite en Rust (`GradeLut`) doit égaler l'ancien calcul GDScript
## (copié ci-dessous comme oracle) à 1/255 près sur tous les préréglages (campagne par saison,
## bataille météo x saison x heure), et être plus rapide. Mesure les deux temps.
## Usage : godot --headless --path game --script res://tests/ms8_grade_lut_test.gd


func _init() -> void:
	var cases: Array = []
	for season in AtmosphereLibrary.SEASONS:
		var preset := CampaignAtmosphere.resolve_preset(season)
		cases.append([preset["grades"], float(preset["strength"])])
	for weather in ["clear", "fog", "rain", "snow"]:
		for season in AtmosphereLibrary.SEASONS:
			for time_key in ["morning", "midday", "evening"]:
				var look := BattleAtmosphere.resolve_look(weather, season, time_key)
				cases.append([look.get("grades", []), float(look.get("strength", 1.0))])
	cases.append([[], 1.0])
	cases.append([cases[0][0], 0.5])
	var size := clampi(int(AtmosphereLibrary.data().get("lut_size", 24)), 8, 64)
	var native_us := 0
	var oracle_us := 0
	var max_diff := 0
	for entry in cases:
		var grades: Array = entry[0]
		var strength: float = entry[1]
		var started := Time.get_ticks_usec()
		var texture := AtmosphereLibrary.grade_lut(grades, strength)
		native_us += Time.get_ticks_usec() - started
		AtmosphereLibrary._lut_cache.clear()
		started = Time.get_ticks_usec()
		var expected := _oracle_layers(grades, strength, size)
		oracle_us += Time.get_ticks_usec() - started
		check(texture != null and texture.get_width() == size, "texture size")
		var bytes: PackedByteArray = ClassDB.instantiate("GradeLut").bake(JSON.stringify(grades), strength, size)
		check_eq(bytes.size(), size * size * size * 3, "bake byte count")
		var layer_bytes := size * size * 3
		for b in size:
			var want := expected[b].get_data()
			for i in layer_bytes:
				max_diff = maxi(max_diff, absi(bytes[b * layer_bytes + i] - want[i]))
	check(max_diff <= 1, "écart max %d/255 > 1" % max_diff)
	print("ms8_grade_lut_test: %d préréglages, écart max %d/255, natif %.1f ms, oracle GDScript %.1f ms (x%.0f)" % [
		cases.size(), max_diff, native_us / 1000.0, oracle_us / 1000.0, float(oracle_us) / maxf(native_us, 1.0)])
	finish()


func _oracle_layers(grades: Array, strength: float, size: int) -> Array[Image]:
	var prepared: Array = []
	for grade in grades:
		if grade is Dictionary and not (grade as Dictionary).is_empty():
			prepared.append(_prepare(grade))
	var layers: Array[Image] = []
	var scale := 1.0 / float(size - 1)
	for b in size:
		var image := Image.create_empty(size, size, false, Image.FORMAT_RGB8)
		for g in size:
			for r in size:
				var source := Vector3(r * scale, g * scale, b * scale)
				var color := source
				for grade in prepared:
					color = _grade(color, grade)
				color = source.lerp(color, strength)
				image.set_pixel(r, g, Color(clampf(color.x, 0.0, 1.0), clampf(color.y, 0.0, 1.0), clampf(color.z, 0.0, 1.0)))
		layers.append(image)
	return layers


## Oracle : ancien calcul GDScript de la LUT (avant MS8), copié tel quel.
func _prepare(grade: Dictionary) -> Dictionary:
	var temperature := float(grade.get("temperature", 0.0))
	var tint := float(grade.get("tint", 0.0))
	var balance := Vector3(1.0 + 0.1 * temperature, 1.0 - 0.06 * tint, 1.0 - 0.12 * temperature)
	return {
		"balance": balance,
		"lift": _vec(grade.get("lift", [0, 0, 0]), Vector3.ZERO),
		"gamma": _vec(grade.get("gamma", [1, 1, 1]), Vector3.ONE),
		"gain": _vec(grade.get("gain", [1, 1, 1]), Vector3.ONE),
		"contrast": float(grade.get("contrast", 1.0)),
		"saturation": float(grade.get("saturation", 1.0)),
		"shadow_saturation": float(grade.get("shadow_saturation", 1.0)),
		"shadows": _vec(grade.get("shadows", [1, 1, 1]), Vector3.ONE),
		"highlights": _vec(grade.get("highlights", [1, 1, 1]), Vector3.ONE),
	}


func _vec(values: Variant, fallback: Vector3) -> Vector3:
	if values is Array and (values as Array).size() >= 3:
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	return fallback


## Un texel : balance des blancs, lift/gain, gamma, virage ombres/lumières, contraste (pivot
## 0,45, adouci en S), saturation (luminance conservée).
func _grade(c: Vector3, g: Dictionary) -> Vector3:
	c = c * (g["balance"] as Vector3)
	var lift: Vector3 = g["lift"]
	c = (c + lift * (Vector3.ONE - c)) * (g["gain"] as Vector3)
	var gamma: Vector3 = g["gamma"]
	c = Vector3(pow(maxf(c.x, 0.0), 1.0 / gamma.x), pow(maxf(c.y, 0.0), 1.0 / gamma.y), pow(maxf(c.z, 0.0), 1.0 / gamma.z))
	var luma := c.dot(Vector3(0.2126, 0.7152, 0.0722))
	var toning := (g["shadows"] as Vector3).lerp(g["highlights"] as Vector3, smoothstep(0.0, 1.0, luma))
	c = c * toning
	var contrast := float(g["contrast"])
	if not is_equal_approx(contrast, 1.0):
		var pivot := 0.45
		var linear := Vector3.ONE * pivot + (c - Vector3.ONE * pivot) * contrast
		# Les extrêmes sont adoucis (épaule et pied) pour ne pas écrêter.
		c = Vector3(_soft_clip(linear.x), _soft_clip(linear.y), _soft_clip(linear.z))
	var saturation := float(g["saturation"])
	if not is_equal_approx(saturation, 1.0):
		luma = c.dot(Vector3(0.2126, 0.7152, 0.0722))
		c = Vector3.ONE * luma + (c - Vector3.ONE * luma) * saturation
	# Ombres désaturées (la saturation HSV des tons sombres gonfle : max - min rapporté à
	# un max faible) ; pleine sur les tons sombres, nulle au-delà de la luminance 0,5.
	var shadow_saturation := float(g["shadow_saturation"])
	if not is_equal_approx(shadow_saturation, 1.0):
		luma = c.dot(Vector3(0.2126, 0.7152, 0.0722))
		var keep := lerpf(shadow_saturation, 1.0, smoothstep(0.0, 0.5, luma))
		c = Vector3.ONE * luma + (c - Vector3.ONE * luma) * keep
	return c


func _soft_clip(x: float) -> float:
	if x < 0.05:
		return 0.05 * exp((x - 0.05) / 0.05) if x > -1.0 else 0.0
	if x > 0.95:
		return 1.0 - 0.05 * exp(-(x - 0.95) / 0.05)
	return x
