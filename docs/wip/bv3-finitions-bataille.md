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
| 5. Duels appariés (cosmétique, déterministe) | **fait** |
| 6. Discours du général | **fait** |

## Prochaine étape
Mesures finales (`--units=50`, Normale et Ultra, A/B `--no-bv3`), fusion de `main`,
vérifications (fmt, clippy, test, build.sh, import, smoke), rapport.

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

## Lot 5 — duels appariés (cosmétiques)
- Le cœur ne modélise pas de duel : `battle_duels.gd` (`BattleDuels`) crée un événement
  purement visuel et déterministe (aucun tirage : figurines, moment et perdant sont des
  fonctions des ids et du temps), sans effet sur les règles, les pertes ou le moral.
- Déclenchement : régiment du général, de chevaliers ou d'hommes d'armes (`enabled_for`) en
  mêlée contre sa cible depuis 1,5 s, caméra à moins de 160 m, 4 duels au plus, 25 s entre deux
  duels d'un même régiment. Les deux figurines au contact quittent la formation
  (`BattleSoldiers.hide_figure`, mécanisme des renversés de BV2) et combattent au milieu.
- Passes synchronisées (0,9 s) de clips existants, en mode CUSTOM : attaque / parade, parade /
  estoc, taille / coup reçu, estoc / renversé (le perdant se relève, clip `knockdown`) ; à cheval :
  estocs de lance alternés. Pas de nouveaux clips appariés cuits dans Blender (écart à la
  demande : les clips existants synchronisés suffisent à la lecture ; piste ouverte).
- Captures `duel_passe_1` / `duel_passe_4` (`tests/bv3_shot.gd --shot=duel`).

## Lot 6 — discours du général
- Données : `data/speeches/battle_speeches.json` (+ schéma, `tools/tests/test_battle_speeches_schema.py`) :
  ouverture par faction, phrase du général ({general}), rapport de forces (fort ≥ 1,3, faible
  ≤ 0,77), terrain, météo, conclusion par faction ; cri de guerre repris de
  `data/battle_orders/order_war_cry.json`. Choix déterministes (graine de la bataille).
- `battle_speech.gd` (`BattleSpeech`) : sous-titre dans une couche à part (le HUD d'UB1 n'est
  pas modifié), travelling de trois quarts le long des régiments du joueur (4,6 s par phrase),
  cri de guerre en grand, caméra qui recule, son `war_cry` d'AU1 ; Échap / Entrée / espace /
  clic : passer ; caméra rendue ensuite. Joué à l'ouverture du déploiement (simulation figée) ;
  sans déploiement, la bataille est mise en pause pendant le discours.
- Jamais en `--autoplay` (bancs, captures, smoke) ni avec `--no-speech` / `--no-bv3` ;
  `--speech-shot=<png> --speech-at=<s>` force le discours et capture.
- Captures `discours_travelling` (6 s), `discours_cri` (25 s).

## Mesures (`--disable-vsync --resolution 1600x900 -- --benchmark --bench-at=90 --units=50`, 11 800 soldiers)
A/B alterné `--no-bv3` / BV3, machine partagée avec d'autres agents (bruit ±30 % entre passes
identiques ; l'écran plafonne souvent à 60 Hz malgré `--disable-vsync`).
| Config | `--no-bv3` (FPS moyens) | BV3 | Primitives |
|---|---|---|---|
| Normale (5 paires) | 48,8 · 79,1 · 58,6 · 59,4 · 43,8 (moy. 57,9) | 43,6 · 58,6 · 58,3 · 65,6 · 50,9 (moy. 55,4) | 2,19-2,24 M → 1,29-1,37 M |
| Ultra ×2,5 (2 paires) | 61,1 · 58,3 | 62,4 · 56,5 | 3,81 M → 1,42-1,63 M |
| Ultra, imposteurs seuls (`--no-impostors`, 1 paire non plafonnée) | 41,4 | 54,6 (+32 %) | 3,79 M → 1,42 M |
- Normale : −4 % en moyenne, dans le bruit (seuil −10 % tenu) ; les imposteurs retirent 40 %
  des primitives, ce qui compense le coût de l'herbe couchée, des étendards et des duels.
- Ultra : la baisse de 20-25 % mesurée par BV1 est rattrapée (primitives ÷2,3 à 2,7).
