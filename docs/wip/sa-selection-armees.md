# Lot SA — sélection, survol et échelle des armées (carte de campagne)

Branche `feat/sa`, worktree `../gp-sa`. ADR 0160. Rendu et entrées seulement (`core/` intact).

Demande (2026-10-02) : cliquer l'ost sélectionne Paris ; pas de surbrillance de ce qu'on vise ;
modèles d'armée médiocres et changement de taille bizarre au zoom ; s'inspirer de Total War.

## Fait
- SA1 visée unique `CampaignMap.pick_target(point, exclude_army)` : survol, clic gauche et clic
  droit (`ArmyMovementController.pick_target`) lisent la même fonction. Armée visée en plein
  (silhouette projetée `ArmyMarker.screen_rect`, ≥ 32 px, ou plaque) > ville ; armée frôlée
  (< 10 px) seulement s'il n'y a pas de ville. Second clic au même endroit : alternance (Q2).
- SA2 survol (`CampaignMap.hover_at`) : armée — socle clair, figurines éclaircies, plaque à
  liseré doré ; ville — écu agrandi à halo clair (`settlement_icon.gdshader`, niveau 0,4) et
  anneau clair autour de l'emprise ; curseur « main » (sauf quand une armée du joueur est
  sélectionnée : curseur du déplacement) ; province éteinte tant qu'un objet est visé.
- SA3 l'ost en garnison se tient au sud-est de sa ville : écart = rayon de la ville (zone
  L1-L3 ou emprise) + 0,4 + demi-emprise de l'ost × échelle ; suspendu (glissement 0,3 s)
  pendant une marche animée.
- SA4 échelle `MIN_SCALE × (d / 19)^0,65` (`map.army_scale_exponent`, schéma à jour) ; loi
  linéaire d'avant sur le parchemin (poids stratégique). Plaque toujours au-dessus de
  l'étendard.
- SA5 socle : disque teinté + liseré (décal), figurines éclaircies au repos (+15 %).
- Test `game/tests/sa_pick_test.gd` (échelle, écart, visée, survol) ; captures
  `game/tests/sa_shot.gd` (24 / 60 / 150 / 400 / 1000 jugées : l'ost est à côté de Paris, la
  plaque le surmonte, le survol se voit).

## Points ouverts
- Entre 600 et 1100 de distance, les figurines sont petites (≈ 15 px en 720p) : la plaque sert
  de repère. Si le joueur les veut plus grandes : `army_scale_exponent` vers 0,75.
- Le modèle lui-même (général monté des figurines de bataille) n'a pas changé : seuls socle,
  éclairage et taille ont été repris. Un vrai pion « seigneur » dédié serait un lot d'art.
- Erreur headless ancienne `Rect2 size is negative` (`_declutter_step`, fenêtre 64×64) : déjà
  sur main, sans rapport.
- Partie pilote : juger le survol et la taille en jeu réel.

## État
Terminé, fusionné dans main (2026-10-02). main fusionné dans la branche avant : le liseré de
relation du lot EN (rouge = ennemi) est gardé, le survol l'éclaircit.
Tests : `sa_pick_test`, `smoke`, `c5_settlements_ui_test`, `m4_free_movement_ui_test`, `cv3_4`,
`nv1`, `p2d`, `rs_n_raze`, `unit_roster` verts. `at1_attack_order_test`, `m5a_vision_ui_test`
et `da7d_overlap_test` (chrono) échouent à l'identique sur main.
