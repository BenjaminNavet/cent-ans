class_name Png16
extends RefCounted

## Décodeur minimal de PNG 16 bits niveaux de gris.
##
## Godot 4.7 réduit les PNG 16 bits à 8 bits au chargement (`Image.FORMAT_L8`),
## ce qui donnerait des marches de ~20 m sur la heightmap. On décode donc le
## flux nous-mêmes : chunks IHDR/IDAT, inflate (zlib) natif, puis défiltrage
## des lignes en GDScript. Le résultat est le tampon brut big-endian (2 octets
## par pixel), utilisable tel quel comme `Image.FORMAT_LA8` (L = octet fort,
## A = octet faible) sans boucle de conversion supplémentaire.

const SIGNATURE := [137, 80, 78, 71, 13, 10, 26, 10]


## Retourne `{"width", "height", "data": PackedByteArray}` ou `{}` si le fichier
## n'est pas un PNG 16 bits niveaux de gris non entrelacé.
static func load_gray16(path: String) -> Dictionary:
	var bytes := FileAccess.get_file_as_bytes(path)
	if bytes.size() < 33:
		return {}
	for i in SIGNATURE.size():
		if bytes[i] != SIGNATURE[i]:
			return {}
	var width := 0
	var height := 0
	var bit_depth := 0
	var color_type := -1
	var interlace := 0
	var idat := PackedByteArray()
	var pos := 8
	while pos + 8 <= bytes.size():
		var length := _read_be32(bytes, pos)
		var chunk_type := bytes.slice(pos + 4, pos + 8).get_string_from_ascii()
		var data_start := pos + 8
		if data_start + length > bytes.size():
			return {}
		if chunk_type == "IHDR":
			width = _read_be32(bytes, data_start)
			height = _read_be32(bytes, data_start + 4)
			bit_depth = bytes[data_start + 8]
			color_type = bytes[data_start + 9]
			interlace = bytes[data_start + 12]
		elif chunk_type == "IDAT":
			idat.append_array(bytes.slice(data_start, data_start + length))
		elif chunk_type == "IEND":
			break
		pos = data_start + length + 4
	if bit_depth != 16 or color_type != 0 or interlace != 0 or width <= 0 or height <= 0:
		return {}
	var stride := width * 2
	var raw := idat.decompress((stride + 1) * height, FileAccess.COMPRESSION_DEFLATE)
	if raw.size() != (stride + 1) * height:
		push_error("Png16: inflate failed for %s" % path)
		return {}
	return {"width": width, "height": height, "data": _unfilter(raw, width, height, 2)}


static func _read_be32(bytes: PackedByteArray, offset: int) -> int:
	return (bytes[offset] << 24) | (bytes[offset + 1] << 16) | (bytes[offset + 2] << 8) | bytes[offset + 3]


## Défiltrage PNG (types 0..4). Les lignes de type 0 (None) sont copiées par
## tranche (rapide) ; les autres passent par une boucle octet par octet.
static func _unfilter(raw: PackedByteArray, width: int, height: int, bpp: int) -> PackedByteArray:
	var stride := width * bpp
	var out := PackedByteArray()
	var prev := PackedByteArray()
	prev.resize(stride)
	var row := PackedByteArray()
	row.resize(stride)
	for y in height:
		var src := y * (stride + 1)
		var filter_type := raw[src]
		src += 1
		match filter_type:
			0:
				row = raw.slice(src, src + stride)
			1:
				for i in stride:
					var left := row[i - bpp] if i >= bpp else 0
					row[i] = (raw[src + i] + left) & 255
			2:
				for i in stride:
					row[i] = (raw[src + i] + prev[i]) & 255
			3:
				for i in stride:
					var left := row[i - bpp] if i >= bpp else 0
					row[i] = (raw[src + i] + ((left + prev[i]) >> 1)) & 255
			4:
				for i in stride:
					var a := row[i - bpp] if i >= bpp else 0
					var b := prev[i]
					var c := prev[i - bpp] if i >= bpp else 0
					var p := a + b - c
					var pa := absi(p - a)
					var pb := absi(p - b)
					var pc := absi(p - c)
					var pred: int
					if pa <= pb and pa <= pc:
						pred = a
					elif pb <= pc:
						pred = b
					else:
						pred = c
					row[i] = (raw[src + i] + pred) & 255
			_:
				push_error("Png16: unknown filter type %d" % filter_type)
				return PackedByteArray()
		out.append_array(row)
		prev = row
		row = PackedByteArray()
		row.resize(stride)
	return out
