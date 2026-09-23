extends SceneTree

## Génère un jeu de données `data/map/` synthétique (512², île, 6 provinces de
## Voronoï, 2 rivières) conforme au contrat docs/design/m1-campaign-map.md,
## dans `game/tests/fixtures/map/` (ou `--out=<dossier>` après `--`).
##
## Usage : godot --headless --path game --script res://tools/gen_synthetic_map.gd
##
## La heightmap est écrite en PNG 16 bits par un encodeur maison (Godot ne sait
## écrire que du 8 bits) ; les lignes utilisent le filtre 0.

const SIZE := 512
const HEIGHT_MIN_M := -200.0
const HEIGHT_MAX_M := 4800.0
const LAND_THRESHOLD := 0.45
const NOISE_SEED := 1337

## Bosses gaussiennes (x, y, rayon) formant l'île principale + un îlot.
const BLOBS := [
	Vector3(256, 250, 150), Vector3(170, 300, 110), Vector3(345, 190, 95),
	Vector3(300, 350, 85), Vector3(430, 400, 38),
]
## Graines de Voronoï (snappées sur la terre).
const SEEDS := [
	Vector2(210, 200), Vector2(320, 190), Vector2(190, 320),
	Vector2(280, 270), Vector2(320, 350), Vector2(235, 130),
]
const NAMES := ["Val d'Orme", "Marche de Brenne", "Comté de Losse", "Haute-Combe", "Plaine d'Aunis", "Cap Ferrand"]
const CAPITALS := ["Ormeval", "Brenne", "Losse", "Combe-le-Haut", "Aunis", "Ferrand"]
const OWNERS := ["fac_france", "fac_england", "fac_france", "fac_burgundy", "fac_england", "fac_france"]
const RIVER_SOURCES := [Vector2i(250, 270), Vector2i(320, 205)]
const RIVER_NAMES := ["Orme", "Brenne"]
const RIVER_STRAHLER := [3, 2]

var out_dir: String
var base_field := PackedFloat32Array()
var heights_m := PackedFloat32Array()
var land := PackedByteArray()
var ids := PackedByteArray()
var seeds: Array[Vector2i] = []
var _crc_table := PackedInt64Array()


func _init() -> void:
	out_dir = ProjectSettings.globalize_path("res://tests/fixtures/map")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
	DirAccess.make_dir_recursive_absolute(out_dir)
	var t0 := Time.get_ticks_msec()
	_generate_heights()
	_write_heightmap()
	_write_land_mask()
	_assign_provinces()
	_write_province_ids()
	var provinces := _describe_provinces()
	_write_json("provinces.geojson", {"type": "FeatureCollection", "features": provinces})
	_write_json("rivers.geojson", {"type": "FeatureCollection", "features": _build_rivers()})
	_write_json("coastline.geojson", {"type": "FeatureCollection", "features": _build_coastline()})
	_write_json("map.json", {
		"crs": "EPSG:3035",
		"bounds_projected": [2600000.0, 1500000.0, 2600000.0 + SIZE * 800.0, 1500000.0 + SIZE * 800.0],
		"size_px": [SIZE, SIZE],
		"meters_per_px": 800.0,
		"height_min_m": HEIGHT_MIN_M,
		"height_max_m": HEIGHT_MAX_M,
		"synthetic": true,
	})
	print("gen_synthetic_map: wrote %s in %d ms" % [out_dir, Time.get_ticks_msec() - t0])
	quit(0)


# --- Relief ------------------------------------------------------------------


func _generate_heights() -> void:
	var noise := FastNoiseLite.new()
	noise.seed = NOISE_SEED
	noise.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	noise.fractal_type = FastNoiseLite.FRACTAL_FBM
	noise.fractal_octaves = 4
	noise.frequency = 0.012
	base_field.resize(SIZE * SIZE)
	heights_m.resize(SIZE * SIZE)
	land.resize(SIZE * SIZE)
	for y in SIZE:
		for x in SIZE:
			var f := 0.0
			for blob: Vector3 in BLOBS:
				var d := Vector2(x - blob.x, y - blob.y).length() / blob.z
				f += exp(-d * d)
			var i := y * SIZE + x
			base_field[i] = f
			var h: float
			if f > LAND_THRESHOLD:
				var t := clampf((f - LAND_THRESHOLD) / 1.6, 0.0, 1.0)
				h = pow(t, 4.0) * 4200.0 + 5.0
				h += noise.get_noise_2d(x, y) * 180.0 * minf(t * 5.0, 1.0)
			else:
				h = -200.0 * clampf((LAND_THRESHOLD - f) / 0.25, 0.0, 1.0)
			h = clampf(h, HEIGHT_MIN_M, HEIGHT_MAX_M)
			heights_m[i] = h
			land[i] = 255 if h > 0.0 else 0


func _write_heightmap() -> void:
	var values := PackedInt32Array()
	values.resize(SIZE * SIZE)
	for i in SIZE * SIZE:
		values[i] = int(round((heights_m[i] - HEIGHT_MIN_M) / (HEIGHT_MAX_M - HEIGHT_MIN_M) * 65535.0))
	var png := _encode_png16(SIZE, SIZE, values)
	var file := FileAccess.open(out_dir.path_join("heightmap.png"), FileAccess.WRITE)
	file.store_buffer(png)
	file.close()


func _write_land_mask() -> void:
	var image := Image.create_from_data(SIZE, SIZE, false, Image.FORMAT_L8, land)
	image.save_png(out_dir.path_join("land_mask.png"))


# --- Provinces ---------------------------------------------------------------


func _assign_provinces() -> void:
	for seed_pos: Vector2 in SEEDS:
		seeds.append(_nearest_land(Vector2i(seed_pos)))
	ids.resize(SIZE * SIZE)
	for y in SIZE:
		for x in SIZE:
			var i := y * SIZE + x
			if land[i] == 0:
				ids[i] = 0
				continue
			var best := 0
			var best_d := INF
			for s in seeds.size():
				var d := Vector2(x - seeds[s].x, y - seeds[s].y).length_squared()
				if d < best_d:
					best_d = d
					best = s + 1
			ids[i] = best


func _nearest_land(from: Vector2i) -> Vector2i:
	return _nearest_with_value(from, land, 255)


func _nearest_with_value(from: Vector2i, values: PackedByteArray, target: int) -> Vector2i:
	for radius in range(0, SIZE):
		for dy in range(-radius, radius + 1):
			for dx in range(-radius, radius + 1):
				if absi(dx) != radius and absi(dy) != radius:
					continue
				var p := from + Vector2i(dx, dy)
				if p.x < 0 or p.y < 0 or p.x >= SIZE or p.y >= SIZE:
					continue
				if values[p.y * SIZE + p.x] == target:
					return p
	return from


func _write_province_ids() -> void:
	var rgb := PackedByteArray()
	rgb.resize(SIZE * SIZE * 3)
	for i in SIZE * SIZE:
		rgb[i * 3] = ids[i] & 255
		rgb[i * 3 + 1] = ids[i] >> 8
		rgb[i * 3 + 2] = 0
	var image := Image.create_from_data(SIZE, SIZE, false, Image.FORMAT_RGB8, rgb)
	image.save_png(out_dir.path_join("province_ids.png"))


func _describe_provinces() -> Array:
	var count := SEEDS.size()
	var sums := []
	var pixels := []
	var height_sums := []
	var neighbors := []
	for _k in count + 1:
		sums.append(Vector2.ZERO)
		pixels.append(0)
		height_sums.append(0.0)
		neighbors.append({})
	for y in SIZE:
		for x in SIZE:
			var i := y * SIZE + x
			var k := ids[i]
			if k == 0:
				continue
			sums[k] += Vector2(x, y)
			pixels[k] += 1
			height_sums[k] += heights_m[i]
			if x + 1 < SIZE:
				_note_neighbor(neighbors, k, ids[i + 1])
			if y + 1 < SIZE:
				_note_neighbor(neighbors, k, ids[i + SIZE])
	var features := []
	for k in range(1, count + 1):
		var centroid := Vector2i((sums[k] / maxi(pixels[k], 1)).round())
		centroid = _nearest_with_value(centroid, ids, k)
		var capital := _nearest_with_value(centroid + Vector2i(7, -5), ids, k)
		var mean_h: float = height_sums[k] / maxi(pixels[k], 1)
		var terrain := "plains" if mean_h < 300.0 else ("hills" if mean_h < 900.0 else "mountains")
		var neighbor_ids := []
		for n in neighbors[k].keys():
			neighbor_ids.append("prov_synth_%d" % n)
		neighbor_ids.sort()
		var rings := _trace_rings(ids, k)
		features.append({
			"type": "Feature",
			"id": "prov_synth_%d" % k,
			"properties": {
				"index": k,
				"id": "prov_synth_%d" % k,
				"name": NAMES[k - 1],
				"owner": OWNERS[k - 1],
				"terrain": terrain,
				"capital_name": CAPITALS[k - 1],
				"centroid": [float(centroid.x) + 0.5, float(centroid.y) + 0.5],
				"capital_px": [float(capital.x) + 0.5, float(capital.y) + 0.5],
				"neighbors": neighbor_ids,
				"area_px": pixels[k],
			},
			"geometry": _rings_to_geometry(rings),
		})
	return features


func _note_neighbor(neighbors: Array, a: int, b: int) -> void:
	if b == 0 or a == b:
		return
	neighbors[a][b] = true
	neighbors[b][a] = true


func _rings_to_geometry(rings: Array) -> Dictionary:
	if rings.size() == 1:
		return {"type": "Polygon", "coordinates": [_ring_coords(rings[0])]}
	var polygons := []
	for ring in rings:
		polygons.append([_ring_coords(ring)])
	return {"type": "MultiPolygon", "coordinates": polygons}


func _ring_coords(ring: PackedVector2Array) -> Array:
	var coords := []
	for p in ring:
		coords.append([p.x, p.y])
	return coords


# --- Rivières et côte --------------------------------------------------------


## Descente de plus grande pente sur le champ de base (sans bruit) jusqu'à la mer.
func _build_rivers() -> Array:
	var features := []
	for r in RIVER_SOURCES.size():
		var p: Vector2i = _nearest_land(RIVER_SOURCES[r])
		var coords := [[p.x + 0.5, p.y + 0.5]]
		var visited := {}
		for step in 2000:
			visited[p] = true
			var best := p
			var best_f: float = base_field[p.y * SIZE + p.x]
			for dy in range(-1, 2):
				for dx in range(-1, 2):
					var q := p + Vector2i(dx, dy)
					if q.x < 0 or q.y < 0 or q.x >= SIZE or q.y >= SIZE or visited.has(q):
						continue
					var f: float = base_field[q.y * SIZE + q.x]
					if f < best_f:
						best_f = f
						best = q
			if best == p:
				break
			p = best
			if step % 3 == 0 or land[p.y * SIZE + p.x] == 0:
				coords.append([p.x + 0.5, p.y + 0.5])
			if land[p.y * SIZE + p.x] == 0:
				break
		features.append({
			"type": "Feature",
			"properties": {"name": RIVER_NAMES[r], "strahler": RIVER_STRAHLER[r]},
			"geometry": {"type": "LineString", "coordinates": coords},
		})
	return features


func _build_coastline() -> Array:
	var features := []
	for ring in _trace_rings(land, 255):
		features.append({
			"type": "Feature",
			"properties": {"kind": "coast"},
			"geometry": {"type": "LineString", "coordinates": _ring_coords(ring)},
		})
	return features


## Contours des pixels `values == target` : arêtes de pixels orientées puis
## chaînées en anneaux fermés (les points colinéaires sont supprimés).
func _trace_rings(values: PackedByteArray, target: int) -> Array:
	var edges: Dictionary = {}  # clé point de départ -> Array de points d'arrivée
	var stride := SIZE + 1
	for y in SIZE:
		for x in SIZE:
			if values[y * SIZE + x] != target:
				continue
			if y == 0 or values[(y - 1) * SIZE + x] != target:
				_add_edge(edges, y * stride + x, y * stride + x + 1)
			if x == SIZE - 1 or values[y * SIZE + x + 1] != target:
				_add_edge(edges, y * stride + x + 1, (y + 1) * stride + x + 1)
			if y == SIZE - 1 or values[(y + 1) * SIZE + x] != target:
				_add_edge(edges, (y + 1) * stride + x + 1, (y + 1) * stride + x)
			if x == 0 or values[y * SIZE + x - 1] != target:
				_add_edge(edges, (y + 1) * stride + x, y * stride + x)
	var rings := []
	while not edges.is_empty():
		var start: int = edges.keys()[0]
		var ring_keys := [start]
		var current := start
		while true:
			if not edges.has(current):
				break
			var ends: Array = edges[current]
			var next: int = ends.pop_back()
			if ends.is_empty():
				edges.erase(current)
			ring_keys.append(next)
			current = next
			if current == start:
				break
		rings.append(_simplify_ring(ring_keys, stride))
	rings.sort_custom(func(a: PackedVector2Array, b: PackedVector2Array) -> bool: return a.size() > b.size())
	return rings


func _add_edge(edges: Dictionary, from: int, to: int) -> void:
	if not edges.has(from):
		edges[from] = []
	edges[from].append(to)


func _simplify_ring(keys: Array, stride: int) -> PackedVector2Array:
	var points := PackedVector2Array()
	var count := keys.size()
	for i in count:
		var p := Vector2(keys[i] % stride, keys[i] / stride)
		var prev := Vector2(keys[(i - 1 + count) % count] % stride, keys[(i - 1 + count) % count] / stride)
		var next := Vector2(keys[(i + 1) % count] % stride, keys[(i + 1) % count] / stride)
		if i != 0 and i != count - 1 and (p - prev) == (next - p):
			continue
		points.append(p)
	return points


# --- Encodage ----------------------------------------------------------------


func _write_json(file_name: String, data: Dictionary) -> void:
	var file := FileAccess.open(out_dir.path_join(file_name), FileAccess.WRITE)
	file.store_string(JSON.stringify(data, "", false))
	file.store_string("\n")
	file.close()


func _encode_png16(width: int, height: int, values: PackedInt32Array) -> PackedByteArray:
	var raw := PackedByteArray()
	raw.resize((width * 2 + 1) * height)
	var o := 0
	for y in height:
		raw[o] = 0
		o += 1
		var row := y * width
		for x in width:
			var v := values[row + x]
			raw[o] = v >> 8
			raw[o + 1] = v & 255
			o += 2
	var ihdr := PackedByteArray()
	ihdr.append_array(_be32(width))
	ihdr.append_array(_be32(height))
	ihdr.append_array(PackedByteArray([16, 0, 0, 0, 0]))
	var png := PackedByteArray([137, 80, 78, 71, 13, 10, 26, 10])
	png.append_array(_chunk("IHDR", ihdr))
	png.append_array(_chunk("IDAT", raw.compress(FileAccess.COMPRESSION_DEFLATE)))
	png.append_array(_chunk("IEND", PackedByteArray()))
	return png


func _chunk(type: String, data: PackedByteArray) -> PackedByteArray:
	var body := type.to_ascii_buffer()
	body.append_array(data)
	var out := _be32(data.size())
	out.append_array(body)
	out.append_array(_be32(_crc32(body)))
	return out


func _be32(value: int) -> PackedByteArray:
	return PackedByteArray([(value >> 24) & 255, (value >> 16) & 255, (value >> 8) & 255, value & 255])


func _crc32(data: PackedByteArray) -> int:
	if _crc_table.is_empty():
		_crc_table.resize(256)
		for n in 256:
			var c := n
			for _k in 8:
				c = (0xEDB88320 ^ (c >> 1)) if (c & 1) == 1 else (c >> 1)
			_crc_table[n] = c
	var crc := 0xFFFFFFFF
	for b in data:
		crc = _crc_table[(crc ^ b) & 255] ^ (crc >> 8)
	return (crc ^ 0xFFFFFFFF) & 0xFFFFFFFF


