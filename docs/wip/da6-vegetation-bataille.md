# Lot DA6 — Végétation de bataille (direction artistique)

Branche `feat/da6-vegetation-bataille` (worktree `.claude/worktrees/agent-abf402aacfc74bffd`).
Bible : `docs/design/2026-09-25-bible-da.md` § 3.3, § 6 « Végétation », § 10 ligne 6.
Suivi DA : `docs/wip/da-direction-artistique.md`. ADR 0067. Rendu seulement (aucun Rust).

## État : terminé, à fusionner par l'orchestrateur
- [x] Drapeau `BattleTerrain.da6` (`--no-da6` : ancien rendu, captures « avant »).
- [x] Lisières douces : `bt_edge_warp` / `bt_field_info_soft` / `bt_decor_edge_soft`
  (`battle_common.gdshaderinc`), sol et herbe d'accord ; rampe 9 m (procédurales), 8 m (décor
  EP6, `_stamp_decor`) ; touffes mêlées au bord (seuil par touffe).
- [x] Touffes en volume (`_clump_mesh_da6`, 4 cartes cintrées/évasées, normales arrondies, pied
  assombri, cartes par la tranche amincies) ; `grass_blades.png` ; hauteurs variées.
- [x] Arbres `battle_trees.gd` : chêne, hêtre, frêne, peuplier noir, saule têtard (bord de
  l'eau), fruitier (vergers, courtils), buisson (haies) ; hiver nu (chêne marcescent).
- [x] LOD par instance (120 m × qualité / 300 m, fondu tramé) et imposteurs cuits au lancement.
- [x] Sol de près : détail de la couche dominante (luminance + normale) fondu 45-117 m.
- [x] Décor sourd : `decor_saturation` 0,6 (automne 0,5) sol, herbe, feuillage, imposteurs.
- [x] Banc A/B dans un processus (`--bench-ab=da6,no-da6`, `ab_mean`).
- [x] Captures `docs/img/da6/` (avant/après), planches `planche_essences*.jpg` + atlas.
- [x] pytest 601 OK, ruff OK, smoke Godot exit 0.

## Mesures (Mac M4 Pro, 1600 × 900, vsync coupée, `--bench-repeat=4`, moyenne `ab_mean`)
Deux passes (la 2e pendant un pytest : même charge pour A et B, bascule toutes les 30 images).

| Bataille | DA6 ms | sans DA6 ms | écart |
|---|---|---|---|
| standard (1 160 soldats) | 9,54 · 9,22 | 9,38 · 8,92 | +1,7 % · +3,5 % |
| `--closeup` | 9,57 · 9,47 | 9,40 · 9,31 | +1,8 % · +1,7 % |
| bocage `--closeup` (haies, pire cas) | 9,41 · 9,84 | 9,18 · 9,69 | +2,4 % · +1,5 % |
| forêt | 9,16 · 9,35 | 8,98 · 9,22 | +1,9 % · +1,4 % |
| hiver `--closeup` | 9,31 · 9,29 | 9,12 · 9,15 | +2,1 % · +1,6 % |
| épique `--units=63 --bench-at=90` | 31,24 · 35,34 | 31,54 · 35,73 | −0,9 % · −1,1 % |

Budget ≤ 5 % tenu. Avant allègement des buissons (haies), le bocage rapproché coûtait +26 %.

Saturation moyenne HSV (zone 3D sous l'horizon, `avant` → `après`) : gros plan 38 → 29 %,
vue haute 53 → 33 %, ligne 33 → 26 %, hiver 23 → 15 %, bocage 67 → 41 %, automne 53 → 44 %.

## Limites / suites
- Automne et bocage restent > 35 % à la mesure : la lumière et le brouillard dorés d'automne
  (`BattleAtmosphere`) et les ombres sombres (saturation HSV gonflée) pèsent ; hors lot.
- Blé et chaume ne suivent pas la saison (blé « doré » au printemps, hérité) : à faire par saison.
- Imposteurs : 4 angles, sans ombre, cylindriques (vus de très haut, un peu plats).
- Arbres d'un seul modèle par essence (variété par lacet, échelle, teinte) ; deux variantes par
  essence doubleraient les tuiles.
- `--standard-shot=foot` cadre un mur de pont (cadrage EP5, identique avant/après).
- La bible § 10 ligne 6 reste à marquer « fait » par l'orchestrateur.

## Journal
- 26/09 01 h : worktree avancé sur main (4c627f4a), branche, dylib, import, captures avant.
- 26/09 ~02 h 30 : herbe, lisières, sol, arbres, imposteurs, désaturation.
- 26/09 ~04 h : buissons allégés, LOD1 sans ombre, brindilles d'hiver épaissies, saturation 0,6,
  captures finales, banc final, ADR 0067.
