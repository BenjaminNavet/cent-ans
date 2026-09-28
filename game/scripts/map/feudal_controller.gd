class_name FeudalController
extends Node

## Lot FE6 (spec FE § 6) : interface de la féodalité sur la carte de campagne. Panneau « Arbre
## féodal » (zone `SIDE_PANEL`), obligations et objectifs, « Qui peut entrer en guerre », actions
## du joueur (commise, concession, révolte, hommage, arbitrage). Tout vient du cœur
## (`CampaignSim.get_feudal_*`) ; ce contrôleur n'affiche et ne transmet que des ordres.

var map: Node = null  # CampaignMap


func setup(campaign_map: Node) -> void:
	map = campaign_map
