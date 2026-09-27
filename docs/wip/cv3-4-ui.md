# CV3-4 — Interface de campagne : postures, rencontres, classes de résultat

Branche : `feat/cv3-4-campaign-ui`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 4 (UI).
Pont utilisé : `get_stance_options`, `get_last_battle_outcome`, `get_encounter_sites`,
`get_pending_encounters`, `choose_encounter_option` (lots CV3-1, CV3-3).

## État : terminé
- Postures : `game/scripts/ui/stance_bar.gd` (`StanceBar`, 6 boutons à icônes dans le cartouche du
  sceau, remplace le menu déroulant de `general_seal.gd`) ; raison du refus du cœur en bulle, bouton
  grisé ; `HudController.show_army` ajoute `army.stance_options` (`get_stance_options`) pour une
  armée du joueur ; le sceau s'agrandit pour contenir la rangée (`_fit_plate`, relayout de `MapUI`).
- Icônes encre (sans dépense) : `stance_ambush` (renard du trait Rusé), `stance_forced_march`
  (pas doublés composés depuis Mouvement, `tools/da5_raw/icons/stance_forced_march.jpg`),
  `stance_entrenched` (pavois), `encounter` (rouleau de la chronique) — entrées `source` de
  `data/ui/icons_ink.json`, construites par `ink_icons.build()` (déterministe, aucun autre fichier
  changé).
- Étendard : `game/scripts/map/stance_badge.gd` (`StanceBadge.apply`, une ligne en fin de
  `ArmyMarker.setup`) — pastille parchemin au sommet de la hampe (enfant du fleuron), fantôme
  (`GeometryInstance3D.transparency` 0,55) pour une armée du joueur en embuscade.
- Rencontres : `game/scripts/map/encounter_controller.gd` (calque 2D projeté, marqueurs, bulle,
  clic = `movement_ctl.order_move_point`) ; `game/scripts/ui/encounter_window.gd`
  (`EncounterWindow extends ChronicleWindow`) ; ouverte après `refresh_all` quand une rencontre
  attend, sauf fin de tour / dialogue de bataille / rapport de saison (elle suit sa fermeture) ;
  « Plus tard » l'écarte jusqu'à `open_window()`. `ArmyMarkers.world_at_pixel` (pixel → monde).
- Classes de résultat : `game/scripts/ui/outcome_band.gd` (`OutcomeBand`, couleur par classe) sur
  l'écran de fin (`battle_result_screen.gd`, lit `aftermath.campaign_outcome` posé par
  `battle_scene.gd` depuis `resolve_battle().outcome`) ; `game/scripts/map/outcome_notice.gd`
  (bandeau 6 s sur la carte pour tout nouveau résultat du joueur hors 3D ; `mark_seen` au retour
  d'une bataille 3D ; pas d'annonce du résultat d'une partie chargée).
- Bulles : `RichTooltip.HUD_TEXTS` `hud_stance_*`, `hud_encounter` (valeurs via `{rule.*}`) ;
  cœur : 7 valeurs de postures dans `rule_constants.rs`. Codex : `cdx_jeu_postures`,
  `cdx_jeu_rencontres` ; `cdx_jeu_mouvement` ne dit plus « pas de marche forcée ».
- Mise en scène : `CampaignSim.debug_place_encounter(enc, x, y)` →
  `CampaignState::debug_put_encounter_site` (+ test `cv3_encounters.rs`).
- Tests : `game/tests/cv3_4_ui_test.gd` (OK), smoke OK, `hud_components_test`, `ub1_ui_test` OK,
  pytest 782 OK. Captures : `game/tests/cv3_4_ui_shot.gd` → `docs/img/cv3/ui-*.png`.

## Points ouverts
- Pas d'illustration propre aux rencontres (`res://assets/encounters/<id>.jpg` lue si elle existe).
- Le cartouche du sceau déborde de 14 px sur la marge de 16 px avant le bandeau d'ost (préexistant,
  largeur des étiquettes de nom).
- Transparence d'embuscade : `transparency` sur les `MultiMeshInstance3D` des figurines ; à revoir
  si CV3-5 change leurs matériaux (ALPHA écrit par le shader).
