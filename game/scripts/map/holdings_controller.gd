class_name HoldingsController
extends Node

## Liste « Colonies » (touche B), lot HL2 : accès rapide aux colonies du joueur façon Total
## War, groupées par province repliable. Rendu, UI et entrées seulement ; toutes les données
## viennent de `CampaignSim.get_holdings_overview(faction)` (lot HL1) :
## `docs/superpowers/specs/2026-09-27-liste-colonies-design.md` § 2.
## Un clic sur une colonie ouvre son panneau (`SettlementController.open_settlement`) ; un clic
## sur « ⌖ » d'une province centre la caméra et ouvre le panneau de province. On ne construit
## ni ne recrute depuis cette liste.

const PANEL_WIDTH := 440.0
const MIN_HEIGHT := 160.0
const FOCUS_DISTANCE := 220.0

var map: Node = null  # CampaignMap
var panel: PanelContainer = null


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "HoldingsController"


## Vrai si la simulation expose l'API des colonies (vraie `CampaignSim`).
func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_holdings_overview")


## Ouvre ou ferme le panneau ; l'ouvrir ferme « Mes unités » (même coin de l'écran).
func toggle() -> void:
	pass


func is_open() -> bool:
	return panel != null and panel.visible


## Reconstruit la liste depuis `get_holdings_overview`. Sans effet si le panneau est fermé.
func refresh() -> void:
	pass


## Nombre de lignes de colonie actuellement affichées (après filtre).
func row_count() -> int:
	return 0


## Nombre de lignes de province actuellement affichées (après filtre).
func province_row_count() -> int:
	return 0


## Filtre actif : "all" / "idle" / "upgrade" / "endangered".
func set_filter(_key: String) -> void:
	pass


## Tri des provinces : "income" (défaut) / "name" / "unrest".
func set_sort(_key: String) -> void:
	pass


## Sélectionne la colonie `id` et y porte la caméra (ouvre son panneau).
func focus_settlement(_id: String) -> void:
	pass
