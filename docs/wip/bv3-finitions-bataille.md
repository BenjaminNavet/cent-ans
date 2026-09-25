# Lot BV3 — Bataille vivante (3) : finitions

Branche `worktree-agent-a4161494b338ea201` (main fusionné + BV1 `worktree-agent-a3fb69eecbaf4f8a1`).
Backlog : `docs/audit/backlog-tw.md` § Bataille. ADR éventuelle : 0024.
Coordination : SG1 (sièges), Q1 (recette) ; HUD de bataille (UB1) intouché sauf sous-titre du discours.

## État
| Lot | État |
|---|---|
| 1. Herbe couchée (piétinement, corps, mêlée) + sang lisible en prairie | **fait** |
| 2. Pavois du dos masqué quand la rangée est plantée | **fait** |
| 3. Imposteurs lointains (Ultra) | à faire |
| 4. Bannières au vent, porte-étendards, étendard du général | à faire |
| 5. Duels appariés (cosmétique, déterministe) | à faire |
| 6. Discours du général | à faire |

## Prochaine étape
Lot 3 : imposteurs lointains (Ultra).

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
