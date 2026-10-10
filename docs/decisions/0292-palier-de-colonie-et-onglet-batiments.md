# 0292 — Palier de colonie et onglet Bâtiments illustré

Statut : accepté

## Contexte
Lot CO-C. Le joueur trouve l'onglet « Bâtiments » d'une colonie trop sec : une liste de lignes. On veut une grande
illustration de la colonie à son niveau de développement et des cartes illustrées pour les emplacements
(ADR 0185 la barre, ADR 0275 le plafond).

## Décision
- Palier de développement, 1 à 6, purement visuel : aucun effet de jeu, aucune règle ne le lit.
  `sim-campaign/src/development_tier.rs` : `CampaignState::settlement_tier` (et `development_score`).
  Score = somme des `tier` du bâtiment le plus haut de chaque chaîne debout (`normalize_building_tiers`) +
  `fortification_level` de la colonie (borné par `fortification_max`). Maximum atteignable = somme des `tier` des
  `building_slot_cap` meilleures chaînes permises au type de colonie (tier maximal de chaque chaîne) + `fortification_max`.
  Le palier n+1 commence à `thresholds_percent[n]` % du maximum.
- Seuils dans `data/settlements/rules.json` `development_tiers` : `thresholds_percent` [10, 25, 40, 60, 80] et
  `fortification_max` 4 (plus haut niveau de fortification des données). Schéma `settlement_rules.schema.json`,
  struct `DevelopmentTiers`. Écart à la demande : un objet et non une liste nue, pour que la fortification maximale
  ne soit pas codée en dur.
- Pont : `CampaignSim.settlement_tier(id) -> int` (1 pour un id inconnu).
- Noms des paliers par type de colonie dans `data/ui/settlement_tiers.json` (schéma `settlement_tiers_ui`), avec le
  dossier des illustrations `res://assets/illustrations/settlement_tiers/<type>_<n>.jpg`.
- Godot : `game/scripts/ui/settlement_buildings_view.gd` (`SettlementBuildingsView`), placé en tête de l'onglet. En-tête
  (image du palier, sinon palier inférieur, sinon bâtiment principal, sinon fond parchemin ; légende ; jauge à six
  crans) puis grille de deux colonnes : une carte par emplacement (`used/max`), carte de bâtiment (image
  `<bld_id>.jpg` ou pictogramme, nom, niveau en chiffres romains et sceaux, deux effets clés, chantier, Améliorer,
  Raser, infobulle IB de la chaîne) ou carte « Emplacement libre ». Une carte libre déplie le choix de construction
  existant. Les listes détaillées (`buildings_list`, `buildable_list`) restent dans une section repliée par défaut :
  les actions et tests existants ne changent pas.

## Conséquences
- Le palier se règle dans les données seules. Une colonie de départ bien dotée (cité à 11 bâtiments) monte vite ; les
  seuils peuvent se resserrer sans toucher au code.
- Les illustrations des paliers sont générées par un autre lot ; en leur absence l'onglet reste lisible.
- La barre des emplacements (ADR 0185) est inchangée.
