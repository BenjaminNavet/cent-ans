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
- [x] Textures : générées une fois à la demande et partagées (`static var`), à 6 px/m, par
      taille arrondie au pas de 2 m et par style plein/pointillé. Chaque texture existe en
      albédo et en émission. Le trait fait 1 m sur toutes les formations (voir le correctif
      plus bas).
- [x] `battle_scene.gd` : `outlines` + `_update_outlines()` (survol = `markers.world_hover` +
      membres du repère B2 survolé) ; `_rings` supprimé ; `BattleMeshes.outline` supprimé (plus
      aucun usage).
- [x] Test `game/tests/cb_m1_outline_test.gd` : table d'états, textures, intégration (une décale
      par régiment, plus d'anneau, suivi, pointillé au survol, pulsé sur la cible, masquage).
- [x] Capture `game/tests/cbm_outline_shot.gd` → `docs/img/cb/cbm1-outline.png` (non relue).
- [x] `cb_m1_outline_test.gd`, `cb0_input_equivalence_test.gd`, `smoke.gd` : codes de sortie 0.

## Correctif « rectangles pleins » (retour de la session principale)

- **Cause** : dans Godot 4, l'émission d'une décale (`texture_emission`) s'ajoute sans tenir
  compte de l'alpha de l'albédo. La texture partagée avait un fond transparent BLANC
  (1, 1, 1, 0) : tout le rectangle émettait la couleur de `modulate` (rouge plein).
- **Correctif** : paire de textures par clé. Albédo sur fond blanc transparent, pour que les
  mipmaps ne foncent pas le trait au loin ; émission sur fond NOIR transparent.
- **Épaisseur constante** : textures à 6 px/m, taille de décale arrondie au pas de 2 m
  supérieur. Le trait fait 1 m, les tirets 4 m, les espaces 2,5 m.
- **Lisibilité** : couleur du camp éclaircie de 30 % pour la sélection, émission 0,9.
- **Mesure sans lecture d'image** : `cbm_outline_shot.gd --probe=<dossier>` (vue plongeante,
  rendus décales masquées / visibles / masquées), puis `python3 game/tests/cbm_outline_probe.py
  <dossier>`. Écart RVB moyen, avec la part de points teintés (> 12) :

  | Décale | Zone | Avant | Après | Bruit |
  |---|---|---|---|---|
  | Sélectionnée | cœur | 43,2 (100 %) | 1,1 (0 %) | 0,2 |
  | Sélectionnée | bord | 66,2 (100 %) | 116,1 (100 %) | 0,0 |
  | Ciblée | cœur | 51,0 (99 %) | 0,6 (0 %) | 0,4 |
  | Ciblée | bord | 73,9 (100 %) | 82,9 (100 %) | 0,5 |

- **Pièges de la sonde** : `unproject_position` rend des coordonnées de canevas (étirement
  1440×900), à ramener aux pixels de l'image. Une vue rasante réduit le trait à 2-3 px. Il faut
  laisser la scène se stabiliser (120 images) avant de comparer.
- **Capture** : `docs/img/cb/cbm1-outline.png` régénérée, caméra rapprochée (110 à 300 m).

## Écarts

- `outline_state` prend un 5e argument `player_side` (le dictionnaire d'unité ne dit pas si
  elle est ennemie).
- Survol par la carte du HUD non branché (seuls terrain et repère B2) : à voir avec CB-M2/CB4.
- Nombre de textures non borné par rapport d'aspect mais par la taille arrondie. On compte
  2 textures par taille (pas de 2 m) et par style, créées à la demande et jamais libérées :
  quelques dizaines par bataille.

## Prochaine étape

Vérification visuelle de `docs/img/cb/cbm1-outline.png` par la session principale, puis fusion.
