class_name BattleTerrainSplat
extends RefCounted

## Textures cuites du terrain de bataille (SC BT7, découpé de `battle_terrain.gd`) : splatmaps tamponnées en
## Rust, carte de hauteurs et de relief, texture de décor, teinte et matériau du sol. L'état reste sur
## `BattleTerrain` (`host`).

var host: BattleTerrain


func _init(p_terrain: BattleTerrain) -> void:
	host = p_terrain


func _build_textures() -> void:
	host._splat_w = int(host.SPLAT_RECT.size.x / host.SPLAT_TEXEL)
	host._splat_h = int(host.SPLAT_RECT.size.y / host.SPLAT_TEXEL)
	# SC PF-08 : splatmaps RGBA8 tamponnées en Rust (`StampMap`), converties en images à la fin.
	var a := StampMap.new()
	var b := StampMap.new()
	a.setup(host._splat_w, host._splat_h, 4, host.SPLAT_RECT.position, host.SPLAT_TEXEL)
	b.setup(host._splat_w, host._splat_h, 4, host.SPLAT_RECT.position, host.SPLAT_TEXEL)
	# Sous-bois et boue : disques des zones de la simulation (bord adouci).
	for zone in host.terrain.get("forests", []):
		_stamp_disc(a, Vector2(float(zone["x"]), float(zone["z"])), float(zone["radius"]) + 5.0, 0, 12.0)
	for zone in host.terrain.get("mud", []):
		_stamp_disc(a, Vector2(float(zone["x"]), float(zone["z"])), float(zone["radius"]), 2, 14.0, 0.5 if host.terrain_key == "marsh" else 1.0)
	_stamp_site(a, b)
	_stamp_decor(a, b)
	# Rivière : galets dans le lit et sur les gués, berges humides.
	if host._river_points.size() >= 2:
		var banks: Array = host.terrain["river"].get("banks", [])
		for i in range(host._river_points.size() - 1):
			var p := host._river_points[i]
			if not host.SPLAT_RECT.grow(40.0).has_point(p):
				continue
			var width := host._river_widths[i] if i < host._river_widths.size() else float(host.terrain["river"]["width"])
			var ford := host._in_ford(p.x)
			# EP3 : gués larges et caillouteux, galets plus serrés.
			# VN2 : hors gué, galets seulement sur une frange au bord de l'eau (avant : ~0,5 × la
			# largeur de grève grise et nue de chaque côté, rivière « canal »).
			_stamp_disc(a, p, width * (1.25 if ford else 0.56), 3, 4.0 if ford else 2.5)
			_stamp_disc(b, p, width * 1.25, 0, 8.0)
			# EP3 : berges marécageuses (boue, herbe humide) ou escarpées (terre nue au bord).
			for bank in banks:
				if p.x < float(bank["x0"]) or p.x > float(bank["x1"]) or ford:
					continue
				var side := 1.0 if bool(bank["north"]) else -1.0
				var edge := p + Vector2(0.0, side * (width * 0.5 + 6.0))
				if str(bank["kind"]) == "marsh":
					# VN2 : boue en taches au bord de l'eau (avant : bande pâle continue de 12 m).
					_stamp_disc(a, p + Vector2(0.0, side * (width * 0.5 + 2.0)), 6.0, 2, 6.0, 0.55)
					_stamp_disc(b, edge, 20.0, 0, 10.0)
				else:
					_stamp_disc(a, p + Vector2(0.0, side * (width * 0.5 + 2.0)), 3.5, 3, 2.0, 0.7)
	# EP3 : ruisseaux (galets du lit, berges humides).
	for stream in host.terrain.get("streams", []):
		var pts: PackedVector2Array = stream["points"]
		var w := float(stream["width"])
		for i in range(pts.size() - 1):
			var steps := maxi(int(pts[i].distance_to(pts[i + 1]) / 2.0), 1)
			for s in steps:
				var p := pts[i].lerp(pts[i + 1], float(s) / float(steps))
				_stamp_disc(a, p, w * 0.55 + 0.5, 3, 1.5, 0.8)
				_stamp_disc(b, p, w * 1.3 + 3.0, 0, 4.0)
	for r in host.roads.size():
		var road := host.roads[r]
		var half := (host.road_widths[r] if r < host.road_widths.size() else 4.0) * 0.5
		for i in range(road.size() - 1):
			var p0 := road[i]
			var p1 := road[i + 1]
			var steps := maxi(int(p0.distance_to(p1) / 2.0), 1)
			for s in steps:
				var p := p0.lerp(p1, float(s) / float(steps))
				if host.SPLAT_RECT.grow(10.0).has_point(p):
					_stamp_disc(a, p, half + 0.7, 1, 2.0)
					_stamp_disc(b, p, half + 6.5, 1, 6.0)
					# EP3 : ornières boueuses sur les routes de terre détrempées.
					if host.muddy():
						_stamp_disc(a, p, half * 0.5, 2, 1.5, 0.45)
	if host.terrain.has("siege"):
		var siege: Dictionary = host.terrain["siege"]
		var center: Vector2 = siege.get("center", Vector2(600, 560))
		# Dans les murs : terre battue ; autour : pas de parcelles.
		# VN : terre battue moins couvrante (vue de haut : ville posée sur du sable uniforme).
		_stamp_disc(a, center, 150.0, 1, 30.0, 0.4)
		_stamp_disc(b, center, 260.0, 1, 60.0)
	host.splat_a = ImageTexture.create_from_image(a.image())
	host.splat_b = ImageTexture.create_from_image(b.image())
	# Carte de hauteurs (herbe) : le champ à 10 m, prolongé par `world_height` (grille en Rust).
	var hw := int(host.SPLAT_RECT.size.x / host.HEIGHT_TEXEL) + 1
	var hh := int(host.SPLAT_RECT.size.y / host.HEIGHT_TEXEL) + 1
	var hdata: PackedFloat32Array = host._kernel.height_grid(host.SPLAT_RECT.position, host.HEIGHT_TEXEL, hw, hh)
	var himage := Image.create_from_data(hw, hh, false, Image.FORMAT_RF, hdata.to_byte_array())
	host.height_texture = ImageTexture.create_from_image(himage)
	var relief_bytes: PackedByteArray = host._kernel.relief_from_heights(hdata, hw, hh, host.HEIGHT_TEXEL)
	host.relief_texture = ImageTexture.create_from_image(Image.create_from_data(hw, hh, false, Image.FORMAT_RGBA8, relief_bytes))
	var noise := FastNoiseLite.new()
	noise.seed = 7
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.frequency = 1.0 / 64.0
	noise.fractal_octaves = 4
	var noise_image := noise.get_seamless_image(512, 512)
	noise_image.generate_mipmaps()
	host.macro_noise = ImageTexture.create_from_image(noise_image)


## B5 : mares (vase et berges humides), fossés (vase),
## plage (sable, sans parcelles) dans les splatmaps.
func _stamp_site(a: StampMap, b: StampMap) -> void:
	for pool in host._pools:
		var c := Vector2(float(pool["x"]), float(pool["z"]))
		var r := float(pool["radius"])
		_stamp_disc(a, c, r * 1.15, 2, 6.0)
		_stamp_disc(b, c, r * 1.5, 0, 8.0)
	for o in host.terrain.get("obstacles", []):
		var kind := str(o["kind"])
		var p0: Vector2 = o["a"]
		var p1: Vector2 = o["b"]
		var steps := maxi(int(p0.distance_to(p1) / 2.0), 1)
		for s in steps + 1:
			var p := p0.lerp(p1, float(s) / float(steps))
			if kind == "ditch":
				_stamp_disc(a, p, 1.6, 2, 2.0)
				_stamp_disc(b, p, 3.5, 0, 3.0)
			# Pas de labours sous les haies et les clôtures (lisière d'herbe).
			_stamp_disc(b, p, 4.0, 1, 4.0)
	if not host._coast.is_empty():
		var west := str(host._coast["flank"]) == "west"
		var beach := float(host._coast["beach"])
		var shore := float(host._coast["shore_x"])
		var x0 := shore if west else host.FIELD_W - beach - 12.0
		var x1 := beach + 12.0 if west else shore
		# Au-delà de la ligne de rivage, sable mouillé jusqu'au bord de la splatmap.
		x0 = minf(x0, host.SPLAT_RECT.position.x) if west else x0
		x1 = maxf(x1, host.SPLAT_RECT.end.x) if not west else x1
		var ix0 := maxi(int((x0 - host.SPLAT_RECT.position.x) / host.SPLAT_TEXEL), 0)
		var ix1 := mini(int((x1 - host.SPLAT_RECT.position.x) / host.SPLAT_TEXEL), host._splat_w - 1)
		# La valeur ne dépend que de x : une colonne entière à la fois.
		for ix in range(ix0, ix1 + 1):
			var x := host.SPLAT_RECT.position.x + ix * host.SPLAT_TEXEL
			var inland := (x - shore) if west else (shore - x)
			var edge := beach - (x if west else host.FIELD_W - x)
			var v := clampf(edge / 12.0 + 0.5, 0.0, 1.0)
			a.raise_column(ix, 1, v)
			b.raise_column(ix, 1, v)
			if inland < 6.0:
				b.raise_column(ix, 0, clampf(1.0 - inland / 6.0, 0.0, 1.0) * 0.8)


## Disque adouci dans le canal `channel` de `image` (coordonnées monde) ; garde le maximum.
func _stamp_disc(image: StampMap, center: Vector2, radius: float, channel: int, feather: float, strength: float = 1.0) -> void:
	image.stamp_soft_disc(center, radius, channel, feather, strength)


## TX T2c : réglages propres aux sols générés (teintes atténuées, grain fin par couche).
func _apply_tx_ground(layers: Array) -> void:
	var material := host.ground_material
	material.set_shader_parameter("tx_ground", 1.0 if host.ground_tx else 0.0)
	var micro_on := host.ground_tx and not host.micro.is_empty()
	material.set_shader_parameter("micro_on", 1.0 if micro_on else 0.0)
	if not micro_on:
		return
	var layer_of := PackedFloat32Array()
	var size_of := PackedFloat32Array()
	layer_of.resize(host.MAX_GROUND_LAYERS)
	size_of.resize(host.MAX_GROUND_LAYERS)
	var micro_layers: PackedFloat32Array = host.micro["layer"]
	var micro_sizes: PackedFloat32Array = host.micro["size"]
	for i in mini(layers.size(), host.MAX_GROUND_LAYERS):
		layer_of[i] = micro_layers[i]
		size_of[i] = micro_sizes[i]
	material.set_shader_parameter("micro_albedo", host.micro["albedo"])
	material.set_shader_parameter("micro_normal", host.micro["normal"])
	material.set_shader_parameter("micro_layer_of", layer_of)
	material.set_shader_parameter("micro_tile_m", size_of)


func _build_material(weather: String) -> void:
	host.resolve_ground()
	host.ground_material = ShaderMaterial.new()
	host.ground_material.shader = host.GROUND_SHADER
	host._apply_ground_noise()
	host.ground_material.set_shader_parameter("albedo_array", host.albedo_array)
	host.ground_material.set_shader_parameter("normal_array", host.normal_array)
	host.ground_material.set_shader_parameter("macro_noise", host.macro_noise)
	host.ground_material.set_shader_parameter("splat_a", host.splat_a)
	host.ground_material.set_shader_parameter("splat_b", host.splat_b)
	host.ground_material.set_shader_parameter("splat_rect", Vector4(host.SPLAT_RECT.position.x, host.SPLAT_RECT.position.y, host.SPLAT_RECT.size.x, host.SPLAT_RECT.size.y))
	var calm := Vector4(150.0, 60.0, 1050.0, 740.0)
	host.ground_material.set_shader_parameter("calm_rect", calm)
	host.ground_material.set_shader_parameter("decor_saturation", host.decor_saturation())
	# GA2 : identité des couches (nombre, taille de répétition) et index des rôles ajoutés
	# (prairie fleurie, herbe piétinée, chaume, labour frais), lus depuis les données
	# (`data/fx/battle_ground_layers.json`), jamais codés en dur dans le shader.
	var ground_layer_list := host.ground_layer_list
	host.ground_material.set_shader_parameter("layer_count", ground_layer_list.size())
	var tile_sizes := PackedFloat32Array()
	tile_sizes.resize(host.MAX_GROUND_LAYERS)
	for i in mini(ground_layer_list.size(), host.MAX_GROUND_LAYERS):
		tile_sizes[i] = float((ground_layer_list[i] as Dictionary).get("tile_size_m", 6.0))
	host.ground_material.set_shader_parameter("layer_tile_size", tile_sizes)
	_apply_tx_ground(ground_layer_list)
	host.ground_material.set_shader_parameter("idx_flowering_meadow", host.ground_role_index("flowering_meadow"))
	host.ground_material.set_shader_parameter("idx_trodden_grass", host.ground_role_index("trodden_grass"))
	host.ground_material.set_shader_parameter("idx_stubble", host.ground_role_index("stubble"))
	host.ground_material.set_shader_parameter("idx_fresh_plough", host.ground_role_index("fresh_plough"))
	if host.decor_on:
		host.ground_material.set_shader_parameter("decor_fields", host.decor_fields)
		host.ground_material.set_shader_parameter("decor_on", 1.0)
	# R2 : relief de détail (rendu seulement) : carte de relief (texels centrés sur la grille de
	# 10 m des hauteurs), roche affleurante selon le terrain, force des normales de détail.
	var hw := int(host.SPLAT_RECT.size.x / host.HEIGHT_TEXEL) + 1
	var hh := int(host.SPLAT_RECT.size.y / host.HEIGHT_TEXEL) + 1
	host.ground_material.set_shader_parameter("relief_map", host.relief_texture)
	host.ground_material.set_shader_parameter("relief_rect", Vector4(host.SPLAT_RECT.position.x - host.HEIGHT_TEXEL * 0.5, host.SPLAT_RECT.position.y - host.HEIGHT_TEXEL * 0.5, hw * host.HEIGHT_TEXEL, hh * host.HEIGHT_TEXEL))
	host.ground_material.set_shader_parameter("relief_on", 1.0)
	host.ground_material.set_shader_parameter("outcrops", clampf((float(host.biome["rocks"]) - 0.7) / 1.5, 0.0, 1.0))
	host.ground_material.set_shader_parameter("detail_bump", lerpf(0.5, 1.0, clampf(float(host.biome["relief"]) / 3.4, 0.0, 1.0)))
	# B5 : sol de saison (neige au sol sans chute de neige, sol détrempé sans pluie), neiges
	# des sommets en montagne, herbe d'hiver ou de plein été.
	var snow_line := float(host.biome["snow_line"])
	if host.season_key != "winter" and snow_line < 10000.0:
		snow_line *= 2.2
	if host.horizon != null and host.horizon.active:
		# EP2 : limite des neiges en altitude réelle ; le sol détaillé (neige uniforme) la prend
		# un peu plus haut que l'anneau d'horizon (neige en plaques).
		snow_line = host.horizon.snow_line_world() + 450.0
	host.ground_material.set_shader_parameter("snow_line", snow_line)
	if host.ground_key == "snowy" and weather != "snow":
		host.ground_material.set_shader_parameter("snow", 0.75)
		host.ground_material.set_shader_parameter("grass_tint", Color(0.85, 0.82, 0.7))
		return
	if host.ground_key == "muddy" and weather != "rain":
		host.ground_material.set_shader_parameter("wetness", 0.45)
	if weather == "clear":
		match host.season_key:
			"winter":
				host.ground_material.set_shader_parameter("grass_tint", Color(0.92, 0.9, 0.72))
				return
			"autumn":
				host.ground_material.set_shader_parameter("grass_tint", Color(1.0, 0.95, 0.72))
				return
			"summer":
				if host.terrain_key != "marsh":
					host.ground_material.set_shader_parameter("grass_tint", Color(0.94, 1.0, 0.78))
					return
	match weather:
		"rain":
			host.ground_material.set_shader_parameter("wetness", 0.75)
			host.ground_material.set_shader_parameter("grass_tint", Color(0.9, 0.95, 0.85))
		"snow":
			host.ground_material.set_shader_parameter("snow", 0.85)
			host.ground_material.set_shader_parameter("grass_tint", Color(0.85, 0.85, 0.78))
		"fog":
			host.ground_material.set_shader_parameter("wetness", 0.25)
		_:
			host.ground_material.set_shader_parameter("grass_tint", Color(0.9, 1.0, 0.8))


## Parcelles du décor peintes au sol (texture `decor_fields`) ; terre battue des cours, des abords
## des maisons et du camp ; boue du fossé du manoir ; pas de parcelles procédurales sous le décor.
func _stamp_decor(a: StampMap, b: StampMap) -> void:
	var decor: Dictionary = host.terrain.get("decor", {})
	if decor.is_empty():
		return
	var sw := host._splat_w
	var sh := host._splat_h
	var fields := Image.create_empty(sw, sh, false, Image.FORMAT_RGBA8)
	fields.fill(Color(0, 0, 0, 0))
	var codes := {"ploughed": 1, "crop": 2, "sown": 4, "stubble": 6}
	for area in decor.get("areas", []):
		var kind := str(area["kind"])
		var code := 0
		match kind:
			"ploughland":
				code = int(codes.get(str(area.get("state", "ploughed")), 1))
			"meadow", "orchard":
				code = 3
			"vineyard":
				code = 5
		var c := Vector2(float(area["x"]), float(area["z"]))
		var half := Vector2(float(area["length"]), float(area["width"])) * 0.5
		var yaw := float(area["yaw"])
		var yaw01 := fposmod(yaw, PI) / PI
		var reach := half.length()
		var ix0 := maxi(int((c.x - reach - host.SPLAT_RECT.position.x) / host.SPLAT_TEXEL), 0)
		var ix1 := mini(int((c.x + reach - host.SPLAT_RECT.position.x) / host.SPLAT_TEXEL) + 1, sw - 1)
		var iz0 := maxi(int((c.y - reach - host.SPLAT_RECT.position.y) / host.SPLAT_TEXEL), 0)
		var iz1 := mini(int((c.y + reach - host.SPLAT_RECT.position.y) / host.SPLAT_TEXEL) + 1, sh - 1)
		for iz in range(iz0, iz1 + 1):
			for ix in range(ix0, ix1 + 1):
				var w := Vector2(host.SPLAT_RECT.position.x + ix * host.SPLAT_TEXEL, host.SPLAT_RECT.position.y + iz * host.SPLAT_TEXEL)
				var local := (w - c).rotated(-yaw)
				var edge := minf(half.x - absf(local.x), half.y - absf(local.y))
				if edge < 0.0:
					continue
				# Pas de parcelles procédurales sous le décor.
				b.raise_pixel(ix, iz, 1, 1.0)
				if code == 0:
					# Hameau, ferme, cimetière, manoir, camp : herbe foulée et terre par endroits.
					a.raise_pixel(ix, iz, 1, 0.25 if kind in ["hamlet", "church"] else 0.35)
					continue
				# DA6 : rampe de lisière sur 8 m (fondu, bord bruité dans les shaders), 3 m sinon.
				fields.set_pixel(ix, iz, Color(float(code) / 8.0, yaw01, clampf(edge / 8.0, 0.0, 1.0), 1.0))
	# Cours de ferme, abords des maisons et des moulins : terre battue.
	for bld in decor.get("buildings", []):
		var p := Vector2(float(bld["x"]), float(bld["z"]))
		_stamp_disc(a, p, float(bld["width"]) * 0.5 + 3.0, 1, 3.0, 0.7)
	for camp in decor.get("camps", []):
		var area: Dictionary = camp["area"]
		var c := Vector2(float(area["x"]), float(area["z"]))
		var r := minf(float(area["length"]), float(area["width"])) * 0.5
		_stamp_disc(a, c, r, 1, r * 0.6, 0.55)
		for item in camp.get("items", []):
			if str(item["kind"]) == "campfire":
				_stamp_disc(a, Vector2(float(item["x"]), float(item["z"])), 1.6, 1, 1.5)
	for moat in decor.get("moats", []):
		var c := Vector2(float(moat["x"]), float(moat["z"]))
		var yaw := float(moat["yaw"])
		var hl := float(moat["length"]) * 0.5
		var hw := float(moat["width"]) * 0.5
		var ring := float(moat["ring"])
		var corners: Array[Vector2] = [Vector2(-hl, -hw), Vector2(hl, -hw), Vector2(hl, hw), Vector2(-hl, hw)]
		for s in 4:
			var p0: Vector2 = corners[s] * (1.0 - ring * 0.5 / maxf(minf(hl, hw), 1.0))
			var p1: Vector2 = corners[(s + 1) % 4] * (1.0 - ring * 0.5 / maxf(minf(hl, hw), 1.0))
			var steps := maxi(int(p0.distance_to(p1) / 2.0), 1)
			for k in steps + 1:
				var q := c + p0.lerp(p1, float(k) / float(steps)).rotated(yaw)
				_stamp_disc(a, q, ring * 0.6, 2, 2.0)
	host.decor_fields = ImageTexture.create_from_image(fields)
	host.decor_on = true
