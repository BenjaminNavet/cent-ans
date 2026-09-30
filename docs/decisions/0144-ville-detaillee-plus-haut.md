# 0144 — Ville détaillée plus haut, une seule teinte de toits de loin

Date : 2026-09-30. Statut : acceptée. Amende l'ADR 0138.

## Contexte
Retour du joueur sur Paris (vue rapprochée) : la ville détaillée 1:1 ne s'affichait que sous
`max_rig_distance` = 16 et ses blocs sous `block_range` = 14 (unités, 1 u ≈ 719 m). À une distance
intermédiaire, on voyait donc la partie proche en blocs à tuiles rouges et le reste en maillage
lointain brun foncé (teinte « masse de toits » moyennée sur tuile, chaume, ardoise, plomb). Le
joueur trouve ce contraste gênant et veut la ville détaillée plus haut.

## Décision
- `max_rig_distance` 16 → 45, `block_range` 14 → 42, `stream_max` 18 → 48
  (`resources/town_render.tres` et valeurs par défaut de `TownRenderProfile`). L'enfoncement du
  lointain suit (`block_range` × qualité × `sink_factor`). Les facteurs de qualité sont inchangés
  (bas : blocs jusqu'à 25).
- Toits des blocs simples (`lod_mode` 2 de `town_building.gdshader`) : fondu vers la teinte
  « masse de toits » (`roofscape.gdshaderinc`, celle du lointain) de `roofscape_near` à
  `roofscape_far` (1,2 → 3,5 u). Essai écarté : aligner le lointain sur la tuile des blocs
  (× 0,8 ; 0,58 ; 0,5) donnait de loin des taches rouge sang.

## Conséquences
- Plus de villes chargées en 1:1 aux hauteurs moyennes (rayon de chargement jusqu'à 48 u) : coût
  mesuré dans `docs/wip/vt.md` (banc d = 30 et 40).
- De près (< 1,2 u), les blocs gardent leur tuile ; au-delà, blocs, sol bâti et lointain ont la
  même teinte.
