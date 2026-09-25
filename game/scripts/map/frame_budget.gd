class_name FrameBudget
extends RefCounted

## PB1 : budget de temps du fil principal pour les constructions progressives de la carte
## (tuiles de relief proches, rubans de route, hameaux). Au zoom, des dizaines de tuiles changent
## de niveau d'un coup ; un nombre fixe de constructions par image donnait des images de plusieurs
## centaines de ms. Chaque système construit au moins un élément par image (progression garantie),
## puis continue tant que le budget commun de l'image n'est pas épuisé.
## `CampaignMap._process` ouvre l'image avec `begin_frame()`.

const BUDGET_USEC := 6000

static var _frame_start_usec: int = 0
static var _frame: int = -1
## Vrai pendant un `flush()` (captures, tests) : tout est construit dans l'image.
static var unlimited: bool = false


static func begin_frame() -> void:
	_frame = Engine.get_process_frames()
	_frame_start_usec = Time.get_ticks_usec()


## Vrai s'il reste du temps pour une construction de plus dans l'image courante. Hors d'une
## image ouverte par `begin_frame` (tests, outils), toujours vrai : seuls les plafonds
## `max_*_per_frame` s'appliquent.
static func has_time() -> bool:
	if unlimited or _frame != Engine.get_process_frames():
		return true
	return Time.get_ticks_usec() - _frame_start_usec < BUDGET_USEC
