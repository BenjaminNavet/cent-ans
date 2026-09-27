# CB-M1 — Contours de formation en décales (état)

Branche : `feat/cb-m1-outline`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
(section CB-M1). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`
(« Contour de formation »).

## État : terminé, en attente de vérification visuelle et de fusion (session principale)

- [x] `game/scripts/battle/battle_formation_outline.gd` (`BattleFormationOutline`) : un `Decal` par
      régiment (créé au premier `update`), `size = (front + 3, 30, profondeur + 3)`, suit
      `x, y, z, facing`. États : Sélectionnée (plein, couleur du camp), Survolée (plein pâle),
      Ennemie survolée (pointillé rouge), Ennemie ciblée (rouge pulsé par `modulate` dans
      `_process`), Aucun (déroute, absente). Mise à jour sans allocation ; décales cachées
      non touchées ; texture/couleur changées seulement si l'état ou les proportions changent.
- [x] Textures : générées une fois à la demande et partagées (`static var`), par rapport
      d'aspect (1 à 10, deux sens) × plein/pointillé, soit 32 textures au plus, pour garder un
      trait d'épaisseur égale sur le front et sur les flancs.
- [x] `battle_scene.gd` : `outlines` + `_update_outlines()` (survol = `markers.world_hover` +
      membres du repère B2 survolé) ; `_rings` supprimé ; `BattleMeshes.outline` supprimé (plus
      aucun usage).
- [x] Test `game/tests/cb_m1_outline_test.gd` : table d'états, textures, intégration (une décale
      par régiment, plus d'anneau, suivi, pointillé au survol, pulsé sur la cible, masquage).
- [x] Capture `game/tests/cbm_outline_shot.gd` → `docs/img/cb/cbm1-outline.png` (non relue).
- [x] `cb_m1_outline_test.gd`, `cb0_input_equivalence_test.gd`, `smoke.gd` : codes de sortie 0.

## Écarts

- `outline_state` prend un 5e argument `player_side` (le dictionnaire d'unité ne dit pas si
  elle est ennemie).
- Survol par la carte du HUD non branché (seuls terrain et repère B2) : à voir avec CB-M2/CB4.
- Épaisseur du trait proportionnelle au petit côté de la formation (≈ 1/24), pas constante en
  mètres.

## Prochaine étape

Vérification visuelle de `docs/img/cb/cbm1-outline.png` par la session principale, puis fusion.
