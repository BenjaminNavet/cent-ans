# CB — icônes des contrôles de bataille (pipeline DA5)

Branche `feat/cb-icons` (depuis main 6d581bb2). Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § Icônes.

## État : terminé, prêt à fusionner (non fusionné)
- Catalogue `data/ui/icons_ink.json` : 16 dessins nouveaux (groupe `cb`, préfixe `cb_`) + cibles ajoutées
  à 6 dessins existants (réemploi gratuit : `stance_normal` → garde, `martial` → mêlée, `enemy_army` →
  pastille mêlée, `fire_at_will` → pastille tir, `form_army` → alerte renforts, `rally` → capacité
  bannière) ; `order_pavise` retiré (l'ordre n'existe plus, le dessin `pavise` sert à `battle_ability_pavise`
  et reste la source de `stance_entrenched`) ; section `cursors` (schéma à jour).
- 25 clés servies : `battle_mode_<run|guard|skirmish|melee|breach>`,
  `battle_state_<rout|wavering|under_fire|charge|melee|shoot|tired>`,
  `battle_alert_<rout|general_down|flanked|reinforcements|ammo_out|wall_breached|gate_destroyed>`,
  `battle_ability_<aimed_shot|pavise|banner_rally|close_ranks|planted_pikes>`, `battle_lock`.
- 6 curseurs 32 px dérivés gratuitement (`ink_icons.build_cursors`, point chaud au centre) dans
  `game/assets/ui/cursors/` : move (Mouvement), melee (épées croisées), ranged et ranged_blocked (cible
  du tir tendu, grisé et barré), siege (tour et échelle), forbidden (cercle barré dessiné).
- Coût : sonde 2 images 0,09 $ + lot 14 images 0,64 $ = 0,73 $ (plafond 3 $) ; cumul DA 21,29 $.
  Lignes déplacées à la main dans la section Direction artistique (l'outil écrit dans la dernière
  section du grand livre, ici PO).
- Branchement Godot : `BattleModeIcons.draw_ink_icon` (texture via `HudStyle.icon`, teinte, faux si
  absente → glyphe en code) utilisé par les modes (barre d'ordres, cartes, pastilles), pastilles d'état,
  alertes CB5, capacités CB4, cadenas ; `BattleCursor` lit déjà `assets/ui/cursors/<contexte>.png`.
- Tests : `game/tests/cb_icons_test.gd` + 13 tests Godot existants verts ; pytest 816 verts.

## Suites possibles
- Contrôle visuel en jeu : `godot --path game --script res://tests/cb2_modes_shot.gd`,
  `cb4_abilities_shot.gd`, `cb5_alerts_shot.gd`.
- `cb_planted_pikes` a un trait plus épais (8,6 px à 128 px, pointes pleines) que la famille (5 px).
