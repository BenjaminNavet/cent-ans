class_name ArmyScale
extends RefCounted

## Loi d'échelle des marqueurs d'armée de la carte en fonction de la distance caméra (SC MC8).
## Fonctions et constantes pures, sans état.

## Échelle du pion sur le parchemin = distance caméra × facteur, bornée (taille constante à
## l'écran) ; en vue normale la loi est sous-linéaire (`scale_for_distance`).
const SCALE_PER_DISTANCE := 0.014
## SA (ADR 0160) : exposant par défaut de la loi d'échelle en vue normale (1 = taille écran
## constante, 0 = taille monde fixe) ; réglage `map.army_scale_exponent`.
const DEFAULT_SCALE_EXPONENT := 0.65
## Q2 : plafond de taille monde au palier « près » (la ville doit dominer l'armée).
## À cette échelle l'étendard royal fait ~3 unités et l'escorte ~2,5 de large, soit environ un
## quart du diamètre de Paris (L1, `core_radius_px` 6) et moins qu'une ville L2/L3 (4,5-7) ;
## les figurines ont la hauteur des maisons. Atteint vers la distance 20 ; 0,8 auparavant, qui
## faisait recouvrir Paris par l'ost au plus près.
const MIN_SCALE := 0.27
const MAX_SCALE := 14.0
## Lot ZG4 : distance sous laquelle l'échelle décroît de nouveau avec la distance.
const CLOSE_KNEE_DISTANCE := 12.0

## SA (ADR 0160) : exposant de la loi d'échelle (`map.army_scale_exponent` de
## `data/ui/campaign_map.json`, borné à [0 ; 1]).
static func scale_exponent() -> float:
	return clampf(float(ArmyFigures.map_settings().get("army_scale_exponent", DEFAULT_SCALE_EXPONENT)), 0.0, 1.0)


## Échelle des marqueurs d'armée pour une distance caméra.
## - Sous `CLOSE_KNEE_DISTANCE` (lot ZG4, vues vallée et site) : proportionnelle à la distance
##   (pas d'étendard de 2 km au-dessus d'un site vu à 200 m).
## - Jusqu'à `MIN_SCALE / SCALE_PER_DISTANCE` (≈ 19) : taille monde fixe `MIN_SCALE` (Q2).
## - Au-delà, vue normale (SA, ADR 0160) : `MIN_SCALE × (d / 19)^exponent`. Avec un exposant
##   inférieur à 1, l'ost rétrécit à l'écran en dézoomant, comme un objet posé sur la carte,
##   au lieu de gonfler avec la distance ; `exponent` = 1 redonne la taille écran constante.
## - Sur le parchemin (`strategic_weight` → 1) : retour à la loi linéaire bornée, l'étendard
##   devient un pion de taille écran constante.
static func scale_for_distance(camera_distance: float, strategic_weight: float = 0.0, exponent: float = DEFAULT_SCALE_EXPONENT) -> float:
	if camera_distance < CLOSE_KNEE_DISTANCE:
		return MIN_SCALE * maxf(camera_distance, 0.02) / CLOSE_KNEE_DISTANCE
	var token := clampf(camera_distance * SCALE_PER_DISTANCE, MIN_SCALE, MAX_SCALE)
	var knee := MIN_SCALE / SCALE_PER_DISTANCE
	var posed := minf(MIN_SCALE * pow(maxf(camera_distance / knee, 1.0), exponent), token)
	return lerpf(posed, token, clampf(strategic_weight, 0.0, 1.0))
