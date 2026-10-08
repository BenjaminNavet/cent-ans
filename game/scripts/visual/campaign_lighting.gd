class_name CampaignLighting
extends RefCounted

## Lot RV-B : éclairage de jeu de la carte de campagne, lu dans `data/fx/campaign_lighting.json`
## (schéma `data/schemas/fx_campaign_lighting.schema.json`). Le soleil de base de chaque saison
## reste dans `data/fx/atmosphere.json` (`campaign.seasons`) ; ce fichier y ajoute :
## - une ambiance de ciel froide (versants à l'ombre bleutés, opposés aux versants au soleil chauds) ;
## - l'exposition ;
## - une variation douce d'un tour à l'autre entre début et fin d'après-midi (soleil plus haut et
##   plus au sud, ou plus bas, plus à l'ouest et plus chaud ; jamais de matin à l'est, qui
##   inverserait la lecture du relief) ;
## - les durées d'interpolation (aucun saut de lumière) ;
## - la perspective aérienne selon l'inclinaison de la caméra (nette en vue basse, presque nulle de
##   dessus : ADR 0142, une brume forte délave la carte).
## Fonctions pures : `CampaignAtmosphere` reste seul propriétaire du soleil. Purement visuel.

const DATA_PATH := "fx/campaign_lighting.json"

static var _data: Dictionary = {}
static var _loaded: bool = false


## Données lues une seule fois ({} et un seul avertissement si le fichier manque : les jeux de
## données de test n'en ont pas, la lumière de la scène reste alors telle quelle).
static func data() -> Dictionary:
	if not _loaded:
		_loaded = true
		var parsed: Variant = DataFile.read_json(DATA_PATH) if DataFile.exists(DATA_PATH) else null
		if parsed is Dictionary:
			_data = parsed
		if _data.is_empty():
			push_warning("CampaignLighting: %s missing or invalid" % DATA_PATH)
	return _data


## Phase du tour dans [-1, 1] : -1 = début d'après-midi, +1 = fin d'après-midi. Déterministe
## (même tour, même lumière, y compris après chargement), irrégulière d'un tour à l'autre mais
## bornée : deux sinus de périodes incommensurables.
static func turn_phase(turn: int) -> float:
	var t := float(turn)
	return clampf(0.65 * sin(t * 2.39996) + 0.35 * sin(t * 0.91 + 1.3), -1.0, 1.0)


## Soleil visé : `base` = préréglage de saison (`CampaignAtmosphere.resolve_preset`), modulé par la
## phase du tour (`turn` < 0 : pas de variation). Renvoie {elevation, azimuth, color, energy}.
static func sun_target(base: Dictionary, turn: int, cfg: Dictionary = data()) -> Dictionary:
	var elevation := float(base.get("sun_elevation", 20.0))
	var azimuth := float(base.get("sun_azimuth", 245.0))
	var color: Color = base.get("sun_color", Color.WHITE)
	var energy := float(base.get("sun_energy", 1.0))
	var variation: Dictionary = cfg.get("turn_variation", {})
	if bool(variation.get("enabled", false)) and turn >= 0:
		var p := turn_phase(turn)
		elevation -= p * float(variation.get("elevation_deg", 0.0))
		azimuth += p * float(variation.get("azimuth_deg", 0.0))
		var warm := p * float(variation.get("warmth", 0.0))
		# Plus chaud en fin d'après-midi : moins de bleu, un peu moins de vert.
		color = Color(color.r, clampf(color.g * (1.0 - 0.4 * warm), 0.0, 1.0), clampf(color.b * (1.0 - warm), 0.0, 1.0))
		energy *= 1.0 - p * float(variation.get("energy", 0.0))
	var bounds: Array = cfg.get("elevation_bounds_deg", [12.0, 40.0])
	elevation = clampf(elevation, float(bounds[0]), float(bounds[1]))
	return {"elevation": elevation, "azimuth": azimuth, "color": color, "energy": energy}


## Ambiance de ciel pour `season` : {color, energy, sky_contribution}. {} sans données.
static func ambient(season: String, cfg: Dictionary = data()) -> Dictionary:
	var block: Dictionary = cfg.get("ambient", {})
	if block.is_empty():
		return {}
	var result := {
		"color": AtmosphereLibrary._rgb(block.get("color"), Color(0.7, 0.76, 0.88)),
		"energy": float(block.get("energy", 0.6)),
		"sky_contribution": float(block.get("sky_contribution", 0.4)),
	}
	var seasons: Dictionary = block.get("seasons", {})
	var over: Dictionary = seasons.get(season, {})
	if over.has("color"):
		result["color"] = AtmosphereLibrary._rgb(over["color"])
	if over.has("energy"):
		result["energy"] = float(over["energy"])
	if over.has("sky_contribution"):
		result["sky_contribution"] = float(over["sky_contribution"])
	return result


## Teinte de la perspective aérienne : couleur de brume de la saison mêlée vers un bleu de
## lointain (`aerial.blue`, part `aerial.blue_mix`).
static func fog_color(season_fog: Color, cfg: Dictionary = data()) -> Color:
	var block: Dictionary = cfg.get("aerial", {})
	if block.is_empty():
		return season_fog
	var blue := AtmosphereLibrary._rgb(block.get("blue"), season_fog)
	return season_fog.lerp(blue, clampf(float(block.get("blue_mix", 0.0)), 0.0, 1.0))


## Réglages de brume de profondeur pour l'inclinaison `pitch_deg` de la caméra (30° = vue basse,
## 70° = vue de dessus) : {density, aerial_perspective, begin_factor, end_factor, sky_affect}.
## Chaque paire des données vaut [vue basse, vue de dessus]. {} sans données (brume de la scène).
static func aerial(pitch_deg: float, cfg: Dictionary = data()) -> Dictionary:
	var block: Dictionary = cfg.get("aerial", {})
	if block.is_empty():
		return {}
	var low_pitch := float(block.get("low_pitch_deg", 30.0))
	var top_pitch := float(block.get("top_pitch_deg", 70.0))
	# 1 = vue basse inclinée, 0 = vue de dessus.
	var low := 1.0 - smoothstep(low_pitch, top_pitch, pitch_deg)
	var result := {}
	for key in ["density", "aerial_perspective", "begin_factor", "end_factor"]:
		var pair: Array = block.get(key, [])
		if pair.size() == 2:
			result[key] = lerpf(float(pair[1]), float(pair[0]), low)
	result["sky_affect"] = float(block.get("sky_affect", 0.4))
	return result


## Rapproche `current` de `target` (dictionnaires de `sun_target`) avec un taux exponentiel
## (fraction `k` dans [0, 1] de l'écart comblée cette image). Azimut par le plus court chemin.
static func blend_sun(current: Dictionary, target: Dictionary, k: float) -> Dictionary:
	var az_delta := wrapf(float(target["azimuth"]) - float(current["azimuth"]), -180.0, 180.0)
	return {
		"elevation": lerpf(float(current["elevation"]), float(target["elevation"]), k),
		"azimuth": float(current["azimuth"]) + az_delta * k,
		"color": (current["color"] as Color).lerp(target["color"], k),
		"energy": lerpf(float(current["energy"]), float(target["energy"]), k),
	}


## Écart résiduel entre deux soleils (degrés et composantes), pour arrêter l'interpolation.
static func sun_gap(a: Dictionary, b: Dictionary) -> float:
	var gap := absf(float(a["elevation"]) - float(b["elevation"]))
	gap = maxf(gap, absf(wrapf(float(a["azimuth"]) - float(b["azimuth"]), -180.0, 180.0)))
	var ca: Color = a["color"]
	var cb: Color = b["color"]
	gap = maxf(gap, 100.0 * maxf(absf(ca.r - cb.r), maxf(absf(ca.g - cb.g), absf(ca.b - cb.b))))
	return maxf(gap, 100.0 * absf(float(a["energy"]) - float(b["energy"])))
