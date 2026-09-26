# Lot DA6 — Végétation de bataille (direction artistique)

Branche `feat/da6-vegetation-bataille` (worktree `.claude/worktrees/agent-abf402aacfc74bffd`).
Bible : `docs/design/2026-09-25-bible-da.md` § 3.3, § 6 « Végétation », § 10 ligne 6.
Suivi DA : `docs/wip/da-direction-artistique.md`. Rendu seulement (aucun Rust prévu).

## Problème (capture `docs/img/da/etat_2509_battle_close.jpg`, avant : `docs/img/da6/avant_*`)
1. Herbe en cartes plates, parcelles à lisière rectiligne, touffes lues comme des plans.
2. Arbres « sucette » (boule sur bâton) au loin comme de près.
3. Sol flou de près.

## Plan
- [x] Squelette : drapeau `BattleTerrain.da6` (`--no-da6`), wip, captures avant.
- [x] Lisières douces : déformation (bruit) des bords de parcelles procédurales et du décor EP6,
  rampe plus large, touffes mêlées au bord (tirage par touffe) — `battle_common.gdshaderinc`,
  `battle_ground.gdshader`, `battle_grass.gdshader`, `_stamp_decor`.
- [x] Touffes en volume : 4 cartes cintrées et évasées, normales arrondies, pied assombri,
  cartes vues par la tranche effacées ; nouvelle texture de touffe (pied resserré) ; variation
  de hauteur et de teinte.
- [x] Arbres : `battle_trees.gd` (`BattleTrees`), feuillus ramifiés procéduraux par essence
  (chêne, hêtre, frêne, peuplier noir, saule têtard au bord de l'eau, fruitier, buisson), LOD1
  issu du même squelette, hiver = branches nues (chêne : feuilles sèches marcescentes).
- [x] Imposteurs d'arbres au-delà de 300 m (atlas cuit au lancement, comme ADR 0024).
- [x] Sol de près : couche de détail (luminance) fondue avec la distance.
- [ ] Saturation ≤ 35 % (mesure sur captures), saisons (hiver, automne).
- [ ] Banc A/B `game/tests/da6_perf.gd` (un seul processus, alternance), budget ≤ 5 %.
- [ ] Captures après `docs/img/da6/`, ADR, docs.

## Journal
- 26/09 01 h : worktree avancé sur main (4c627f4a), branche créée, dylib construite, import,
  captures avant (`avant_closeup`, `avant_foot` — plan de mur, cadrage EP5 —, `avant_haute`,
  `avant_hiver`, `avant_automne`).

- 26/09 ~02 h 30 : herbe, lisières, sol, arbres (`battle_trees.gd`), imposteurs, désaturation
  (0,66 ; automne 0,55) commités. Banc A/B en un processus : `--bench-ab=da6,no-da6` (moyenne
  `ab_mean` en plus de la médiane : sous Metal la médiane colle aux paliers d'affichage).
  Premières mesures : plaine +0 %, bocage gros plan +5 % avant allègement des buissons, ~+1 %
  après (machine bruitée). Planche des essences : `game/tests/da6_trees_shot.gd`.

## Prochaine étape
Captures après (dont un bois proche), banc final (plaine, bocage, épique), ADR, CREDITS/SOURCE.
