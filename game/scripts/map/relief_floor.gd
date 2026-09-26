class_name ReliefFloor
extends RefCounted

## Lot ZG8 (ADR 0036) : fond de vallée lissé de la carte de campagne, base du relief exagéré
## (`MapData.display_height`). Grille basse résolution calculée une fois au chargement :
## 1. par cellule de `floor_cell_px` pixels (8 px = 5,75 km), minimum des altitudes échantillonnées
##    tous les `floor_sample_step` pixels, altitudes bornées à 0 (la mer ne creuse pas le fond :
##    la côte reste au niveau 0, `fond` ≥ 0) ;
## 2. filtre minimum de rayon `floor_min_radius` cellules (fonds de vallée, ~17 km) ;
## 3. `floor_blur_passes` flous de boîte de rayon `floor_blur_radius` (pas de marches) ;
## 4. ZG7c : fond relevé à `sommets − local_relief_cap_m` (maximum par cellule, filtre maximum
##    sur le rayon total des flous, puis mêmes flous) : le terme local `h − fond` ne dépasse plus ≈ le plafond, les montagnes ne
##    deviennent pas des aiguilles (Alpes, Pyrénées, puys, Snowdonia : +100 % de relief exagéré
##    en plus du ×2,5-3,4 de ZG4) ; collines et falaises sous le plafond ne changent pas.
##    Relever le fond ne fait que baisser la hauteur affichée (jamais sous `s·h`).
## Tout est séparable et réparti sur le `WorkerThreadPool` (lignes indépendantes).
##
## Convention (même que la heightmap) : le pixel carte i est centré en x monde = i ; la cellule c
## couvre les pixels [c·C, (c+1)·C − 1], centre en x = c·C + (C − 1) / 2.


## Calcule le fond (mètres) : {"data": PackedFloat32Array, "base": PackedFloat32Array (SZ1, fond non
## plafonné), "squash": PackedFloat32Array (SZ1, facteur k d'écrasement des montagnes, 0..1),
## "side": Vector2i, "cell": float, "ms": float}.
static func compute(data: MapData, profile: ReliefExaggerationProfile) -> Dictionary:
	var t0 := Time.get_ticks_usec()
	var cell := maxi(profile.floor_cell_px, 1)
	var side := Vector2i(ceili(float(data.size.x) / cell), ceili(float(data.size.y) / cell))
	# Lignes produites en parallèle dans un Array (les tableaux compacts capturés par une lambda
	# sont copiés : écrire dedans depuis la tâche serait perdu).
	var out_rows: Array = []
	out_rows.resize(side.y)
	var step := clampi(profile.floor_sample_step, 1, cell)
	var size := data.size
	var bytes := data.height_bytes
	var bpp := data.height_bpp
	var little := data.height_little_endian
	var h_min := data.height_min_m
	var h_range := data.height_max_m - data.height_min_m
	var rows := func(r: int) -> void:
		var line := PackedFloat32Array()
		line.resize(side.x)
		var line_max := PackedFloat32Array()
		line_max.resize(side.x)
		for c in side.x:
			var best := INF
			var top := -INF
			var y := r * cell
			while y < mini((r + 1) * cell, size.y):
				var row := y * size.x
				var x := c * cell
				while x < mini((c + 1) * cell, size.x):
					var o := row + x
					var v: float
					if bpp == 2:
						o *= 2
						v = float(bytes[o] | (bytes[o + 1] << 8)) if little else float((bytes[o] << 8) | bytes[o + 1])
						v /= 65535.0
					else:
						v = float(bytes[o]) / 255.0
					best = minf(best, h_min + v * h_range)
					top = maxf(top, h_min + v * h_range)
					x += step
				y += step
			line[c] = maxf(best if best < INF else 0.0, 0.0)
			line_max[c] = maxf(top if top > -INF else 0.0, 0.0)
		out_rows[r] = [line, line_max]
	_parallel(rows, side.y, "relief floor cells")
	var mins := _join(out_rows.map(func(pair: Array) -> PackedFloat32Array: return pair[0]))
	var grid := _filter(mins, side, maxi(profile.floor_min_radius, 0), MODE_MIN)
	for pass_index in maxi(profile.floor_blur_passes, 0):
		grid = _filter(grid, side, maxi(profile.floor_blur_radius, 0), MODE_MEAN)
	# SZ1 : base (fond non plafonné) et facteur d'écrasement des montagnes (amplitude régionale).
	var base := grid.duplicate()
	var squash := PackedFloat32Array()
	squash.resize(grid.size())
	var amplitude := PackedFloat32Array()
	amplitude.resize(grid.size())
	if profile.local_relief_cap_m > 0.0 or profile.mountain_knee_m > 0.0:
		var tops := _join(out_rows.map(func(pair: Array) -> PackedFloat32Array: return pair[1]))
		# Maximum sur le rayon total des flous qui suivent : un pic isolé (puy, aiguille) garde sa
		# hauteur au centre après lissage au lieu d'être dilué par ses voisins plus bas.
		var reach := maxi(profile.floor_min_radius, 0) + maxi(profile.floor_blur_radius, 0) * maxi(profile.floor_blur_passes, 0)
		tops = _filter(tops, side, reach, MODE_MAX)
		for pass_index in maxi(profile.floor_blur_passes, 0):
			tops = _filter(tops, side, maxi(profile.floor_blur_radius, 0), MODE_MEAN)
		for i in grid.size():
			if profile.local_relief_cap_m > 0.0:
				grid[i] = maxf(grid[i], tops[i] - profile.local_relief_cap_m)
			amplitude[i] = tops[i] - base[i]
			squash[i] = profile.mountain_squash_of(amplitude[i])
	return {"data": grid, "base": base, "squash": squash, "amplitude": amplitude, "side": side, "cell": float(cell), "ms": (Time.get_ticks_usec() - t0) / 1000.0}


const MODE_MIN := 0
const MODE_MEAN := 1
const MODE_MAX := 2


## Filtre séparable (minimum, moyenne de boîte ou maximum) de rayon `radius`, bords répliqués.
static func _filter(src: PackedFloat32Array, side: Vector2i, radius: int, mode: int) -> PackedFloat32Array:
	if radius <= 0:
		return src
	var inv := 1.0 / (2 * radius + 1)
	var rows: Array = []
	rows.resize(side.y)
	var horizontal := func(r: int) -> void:
		var base := r * side.x
		var line := PackedFloat32Array()
		line.resize(side.x)
		for c in side.x:
			var acc := src[base + clampi(c - radius, 0, side.x - 1)]
			for k in range(-radius + 1, radius + 1):
				var v := src[base + clampi(c + k, 0, side.x - 1)]
				if mode == MODE_MEAN:
					acc += v
				elif mode == MODE_MIN:
					acc = minf(acc, v)
				else:
					acc = maxf(acc, v)
			line[c] = acc * inv if mode == MODE_MEAN else acc
		rows[r] = line
	_parallel(horizontal, side.y, "relief floor filter h")
	var tmp := _join(rows)
	var vertical := func(r: int) -> void:
		var line := PackedFloat32Array()
		line.resize(side.x)
		for c in side.x:
			var acc := tmp[clampi(r - radius, 0, side.y - 1) * side.x + c]
			for k in range(-radius + 1, radius + 1):
				var v := tmp[clampi(r + k, 0, side.y - 1) * side.x + c]
				if mode == MODE_MEAN:
					acc += v
				elif mode == MODE_MIN:
					acc = minf(acc, v)
				else:
					acc = maxf(acc, v)
			line[c] = acc * inv if mode == MODE_MEAN else acc
		rows[r] = line
	_parallel(vertical, side.y, "relief floor filter v")
	return _join(rows)


static func _join(rows: Array) -> PackedFloat32Array:
	var result := PackedFloat32Array()
	for line: PackedFloat32Array in rows:
		result.append_array(line)
	return result


static func _parallel(task: Callable, count: int, label: String) -> void:
	var id := WorkerThreadPool.add_group_task(task, count, -1, true, label)
	WorkerThreadPool.wait_for_group_task_completion(id)


## Texture du fond pour les shaders (RGBF : R fond, G base, B facteur d'écrasement SZ1 ; lue au
## texel près par `campaign_relief.gdshaderinc`). Grille sans base ni écrasement : G = R, B = 0.
static func texture_of(grid: Dictionary) -> ImageTexture:
	var side: Vector2i = grid["side"]
	var data: PackedFloat32Array = grid["data"]
	var base: PackedFloat32Array = grid.get("base", data)
	var squash: PackedFloat32Array = grid.get("squash", PackedFloat32Array())
	var packed := PackedFloat32Array()
	packed.resize(data.size() * 3)
	for i in data.size():
		packed[3 * i] = data[i]
		packed[3 * i + 1] = base[i] if base.size() == data.size() else data[i]
		packed[3 * i + 2] = squash[i] if squash.size() == data.size() else 0.0
	var image := Image.create_from_data(side.x, side.y, false, Image.FORMAT_RGBF, packed.to_byte_array())
	return ImageTexture.create_from_image(image)
