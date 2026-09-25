class_name ReliefFloor
extends RefCounted

## Lot ZG8 (ADR 0036) : fond de vallée lissé de la carte de campagne, base du relief exagéré
## (`MapData.display_height`). Grille basse résolution calculée une fois au chargement :
## 1. par cellule de `floor_cell_px` pixels (8 px = 5,75 km), minimum des altitudes échantillonnées
##    tous les `floor_sample_step` pixels, altitudes bornées à 0 (la mer ne creuse pas le fond :
##    la côte reste au niveau 0, `fond` ≥ 0) ;
## 2. filtre minimum de rayon `floor_min_radius` cellules (fonds de vallée, ~17 km) ;
## 3. `floor_blur_passes` flous de boîte de rayon `floor_blur_radius` (pas de marches).
## Tout est séparable et réparti sur le `WorkerThreadPool` (lignes indépendantes).
##
## Convention (même que la heightmap) : le pixel carte i est centré en x monde = i ; la cellule c
## couvre les pixels [c·C, (c+1)·C − 1], centre en x = c·C + (C − 1) / 2.


## Calcule le fond (mètres) : {"data": PackedFloat32Array, "side": Vector2i, "cell": float, "ms": float}.
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
		for c in side.x:
			var best := INF
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
					x += step
				y += step
			line[c] = maxf(best if best < INF else 0.0, 0.0)
		out_rows[r] = line
	_parallel(rows, side.y, "relief floor cells")
	var mins := _join(out_rows)
	var grid := _filter(mins, side, maxi(profile.floor_min_radius, 0), true)
	for pass_index in maxi(profile.floor_blur_passes, 0):
		grid = _filter(grid, side, maxi(profile.floor_blur_radius, 0), false)
	return {"data": grid, "side": side, "cell": float(cell), "ms": (Time.get_ticks_usec() - t0) / 1000.0}


## Filtre séparable (minimum ou moyenne de boîte) de rayon `radius`, bords répliqués.
static func _filter(src: PackedFloat32Array, side: Vector2i, radius: int, use_min: bool) -> PackedFloat32Array:
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
			var acc := INF if use_min else 0.0
			for k in range(-radius, radius + 1):
				var v := src[base + clampi(c + k, 0, side.x - 1)]
				acc = minf(acc, v) if use_min else acc + v
			line[c] = acc if use_min else acc * inv
		rows[r] = line
	_parallel(horizontal, side.y, "relief floor filter h")
	var tmp := _join(rows)
	var vertical := func(r: int) -> void:
		var line := PackedFloat32Array()
		line.resize(side.x)
		for c in side.x:
			var acc := INF if use_min else 0.0
			for k in range(-radius, radius + 1):
				var v := tmp[clampi(r + k, 0, side.y - 1) * side.x + c]
				acc = minf(acc, v) if use_min else acc + v
			line[c] = acc if use_min else acc * inv
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


## Texture du fond pour les shaders (RF, lue au texel près par `campaign_relief.gdshaderinc`).
static func texture_of(grid: Dictionary) -> ImageTexture:
	var side: Vector2i = grid["side"]
	var data: PackedFloat32Array = grid["data"]
	var image := Image.create_from_data(side.x, side.y, false, Image.FORMAT_RF, data.to_byte_array())
	return ImageTexture.create_from_image(image)
