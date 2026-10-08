class_name Hash
extends RefCounted

## Tirages déterministes entiers → [0, 1[ (décor, foule, cicatrices). Les formules sont figées :
## les changer déplace le décor déjà placé. `lcg01` est identique au `hash01i` du shader et au Rust.


## Congruentiel + mélange de bits ; base de `h01`.
static func lcg01(n: int) -> float:
	var h := (n * 1103515245 + 12345) & 0x7fffffff
	h = (h ^ (h >> 13)) * 1274126177 & 0x7fffffff
	return float(h % 100000) / 100000.0


## Tirage de jusqu'à trois entiers (clés premières XOR) via `lcg01`.
static func h01(a: int, b: int = 0, c: int = 0) -> float:
	return lcg01(a * 73856093 ^ b * 19349663 ^ c * 83492791)


## Tirage de deux entiers via le `hash` natif de `Vector2i` (pas 10007).
static func vec01(a: int, b: int) -> float:
	return float(absi(hash(Vector2i(a, b))) % 10007) / 10007.0


## Tirage 24 bits d'une clé et d'un sel via le `hash` natif de `Vector2i`.
static func vec24(key: int, salt: int) -> float:
	var h := hash(Vector2i(key, salt))
	return float(h & 0xFFFFFF) / float(0x1000000)
