# Lot BV3 — Bataille vivante (3) : finitions

Branche `worktree-agent-a4161494b338ea201` (main fusionné + BV1 `worktree-agent-a3fb69eecbaf4f8a1`).
Backlog : `docs/audit/backlog-tw.md` § Bataille. ADR éventuelle : 0024.
Coordination : SG1 (sièges), Q1 (recette) ; HUD de bataille (UB1) intouché sauf sous-titre du discours.

## État
| Lot | État |
|---|---|
| 1. Herbe couchée (piétinement, corps, mêlée) + sang lisible en prairie | **fait** |
| 2. Pavois du dos masqué quand la rangée est plantée | **fait** |
| 3. Imposteurs lointains (Ultra) | **fait** (ADR 0024) |
| 4. Bannières au vent, porte-étendards, étendard du général | **fait** |
| 5. Duels appariés (cosmétique, déterministe) | à faire |
| 6. Discours du général | à faire |

## Prochaine étape
Lot 5 : duels appariés cosmétiques (`battle_duels.gd`, section `duels` de
`data/fx/battle_finish.json`, déjà écrite et validée).

## Lot 1 — herbe couchée, sang en prairie
- `battle_grass_flatten.gd` (`BattleGrassFlatten`) : carte RG8 à 1 m sur (0, −100)-(1200, 900),
  R = herbe couchée, G = sang sur l'herbe. Sources : régiments en marche (plafond 150/255 : herbe
  foulée), à l'arrêt (70/255), front de mêlée (bande de 6 m, couchée net), corps (`corpse_fallen`,
  disque 1 m, 1,6 m pour un cheval ; sang selon le réglage). Envoi GPU au plus toutes les 0,5 s
  de bataille, seulement si la carte a changé.
- La carte B7/B8 (`BattleTerrain.update_trample`, neige ou boue seulement) n'est pas réutilisée :
  elle n'existe pas sur sol sec (le cas de la prairie) et appartient au terrain.
- `battle_grass.gdshader` : touffes couchées (hauteur ×0,22, brins étalés au ras du sol, bords
  irréguliers au bruit), un peu clairsemées (−30 %), jaunies ; sang : brins teintés.
- `BattleVegetation.set_flatten()`, branché dans `BattleScene._setup_grass_flatten()`.
- `--no-bv3` : toutes les finitions BV3 coupées (A/B).
- Captures : `avant_herbe_sang` / `apres_herbe_sang` (`tests/bv3_shot.gd --shot=grass
  --center=640,440`, `--no-bv3` pour l'avant).

## Lot 2 — pavois en double
- Blender : le pavois du dos (`archer_2`) porte le masque `PAVISE_MASK` = 0b1000_0011 (bit 7 =
  drapeau de pièce) ; seul `archer_2` régénéré (`-- --no-rigs --only archer_2`).
- Shader skinné : `hide_pavise` masque les faces au bit 7. `BattleSoldiers.hide_planted_pavise`
  (actif avec BV1) le pose par régiment quand `pavise_cover` et le régiment ne marche ni ne
  charge (la règle de BV1 qui plante la rangée). Les cadavres gardent le pavois au dos.
- Captures : `avant_pavois_dos` / `apres_pavois_dos` (`tests/v2_figures_shot.gd --fig=archer_2
  --state=shooting [--hide-pavise]`).

## Lot 3 — imposteurs lointains (ADR 0024)
- `battle_impostors.gd` (`BattleImpostors`, enfant de `BattleSoldiers`) : atlas cuits en jeu
  depuis les figurines V2 (8 angles × 4 jeux × 4 images), `battle_impostor.gdshader`.
- `BattleSoldiers` : au-delà de 300 m, couche d'imposteurs à la place du LOD2, même tampon.
- `--no-impostors` : imposteurs seuls coupés. `tests/bv3_shot.gd --shot=atlas|impostor
  [--fig=archer_0]` : atlas brut, comparaison maillage / imposteur de près.
- Premier banc Ultra (`--units=50 --unit-size=2.5`, une passe chacun) : sans imposteurs
  41,4 FPS, 3,79 M primitives ; avec 54,6 FPS, 1,42 M primitives.
- Captures `avant_ultra_imposteurs` / `apres_ultra_imposteurs` (`--shot-at=60 --units=20
  --unit-size=2.5 --camera=600,420,420,200`).
- Piège : sans relever l'alpha selon le niveau de mipmap, les régiments lointains disparaissent.

## Lot 4 — vent, bannières, porte-étendards
- Données : `data/fx/battle_finish.json` (+ `data/schemas/battle_finish.schema.json`,
  `tools/tests/test_battle_finish_schema.py`) : vent par météo, étendards, duels (lot 5).
- `battle_standards.gd` (`BattleStandards`) : vent (direction tirée de la graine de bataille,
  force et rafales selon la météo : clair 0,55, pluie 0,85, brouillard 0,12, neige 0,6) ;
  étendard à l'échelle 1 (hampe 4,2 m, étoffe du régiment ×0,55) porté par la figurine du
  milieu du tampon (`figure_frame`), à droite du porteur, plus haut à cheval ; général : hampe
  5,6 m, étendard royal ×0,85. Affichés à moins de 220 m.
- `battle_banner.gdshader` : `wind_strength` / `wind_gust` (vitesse, amplitude, drapeau qui
  pend sans vent, rafales lentes par drapeau).
- `BattleScene` : drapeaux-repères (V4) dans le vent de près (orientés vers la caméra en
  s'éloignant, comme avant), effacés devant l'étendard porté quand leur échelle ≤ 1,05 ;
  herbe dans le même vent (`BattleVegetation.set_wind`).
- Capture `apres_etendards_vent` (`tests/bv3_shot.gd --shot=standards`, décor factice
  `FakeBattle` qui rend les figurines rangées sans simulation).
