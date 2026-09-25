# SG1 — Batailles de siège à la Total War

Branche `worktree-agent-a806226fd4b8b61a9`. Backlog `docs/audit/backlog-tw.md` (Bataille : sièges).

## Objectif
Assauts spectaculaires et lisibles : échelles dressées et escaladées, beffrois accostés (pont-levis
abaissé), bélier sous manteau qui frappe en rythme, trébuchets et bombardes (projectiles, traînée,
éclats de pierre), défenseurs sur le chemin de ronde, combat sur le mur, huile bouillante, porte qui
cède, garnison qui se replie sur la place.

## Architecture
- Cœur (`core/crates/sim-battle`) :
  - `src/siege_fx.rs` : `SiegeFx { time, kind }`, événements pour le rendu (tir d'engin avec point
    d'impact, coup de bélier, volée d'une tour, échelles dressées, beffroi accosté/décroché, prise du
    rempart, huile bouillante, porte enfoncée, brèche, repli de la garnison). Aucun tirage aléatoire
    (hachage entier) : déterminisme intact.
  - `src/sim/siege_assault.rs` : file d'événements, coups du bélier toutes les `RAM_PERIOD` = 3 s
    (même dégât moyen qu'avant), **huile bouillante** (nouvelle règle : un pot toutes les 20 s si un
    défenseur garde la porte, sur les assaillants à moins de 10 m de sa face ; armure à moitié
    efficace ; bélier protégé par ses peaux), transitions (porte, brèche, beffroi).
  - `ai.rs` (`plan_siege_defence`) : au plus 2 régiments par ouverture la bouchent, le reste de
    l'infanterie de la garnison se regroupe sur la place.
- Pont : `BattleSim.get_siege_events()`, `get_siege().ram_period / oil_period`.
- Godot : fichier séparé `game/scripts/battle/siege_assault_fx.gd` (ne touche pas aux maisons de
  BR1 dans `battle_siege.gd`).

## État : terminé (non fusionné)
- [x] Cœur : `siege_fx.rs` (événements), `sim/siege_assault.rs` (bélier par coups, huile, transitions,
  échelles, poses des grimpeurs), `ai.rs` (repli sur la place), pont `get_siege_events`,
  `debug_set_piece_hp`, `get_units().climbers_shown / ladder_lines`. Tests `tests/sg1.rs` (7).
  Sonde escalade `SEEDS=20 probe -- siege 0 ""` : 9/20 → 7/20 (huile réglée à 30 s / 3 hommes /
  −4 moral ; à 20 s / 4 / −8 on tombait à 5/20) ; `siege 0` et `siege 60` : 6/6 inchangé.
- [x] Clip `climb` (pipeline V2, `human.bones.bin` régénéré, seuls ce fichier et le manifeste
  changent) ; shader `M_SPLIT` + `split_count` ; `STYLES.*.climbing`.
- [x] Rendu `siege_assault_fx.gd` : échelles dressées (1,4 s) sur `ladder_lines`, laissées au mur
  jusqu'à la chute du pan ; bélier calé face à la porte, poutre qui recule puis frappe en rythme
  (`ram_period`), échardes et poussière à chaque coup, vantaux qui tremblent ; beffroi habillé
  (poteaux, lisses, créneaux) et pont-levis articulé abaissé à l'accostage ; pierres et boulets avec
  traînée vers le point d'impact, éclats de pierre, poussière, son d'impact à l'arrivée ; carreaux
  des tours ; huile (coulée + vapeur, cris) ; porte enfoncée (planches projetées, vantail arraché).
  Tours de la porte écartées au rendu (`gatehouse_tower`) : la porte est enfin visible.
- [x] Captures `docs/audit/captures/sg1/` (`game/tests/sg1_siege_shot.gd`) et planches de séquences
  (`*_sequence.png`, 6 images à 0,25 s).
- [x] ADR 0023, section SG1 de `docs/design/m8-sieges.md`.

## Mesures FPS (assaut de la Guyenne, `--siege --units=16 --benchmark --bench-at=130
--camera=600,395,110,205`, 1600×900, dylib release copiée sous le nom debug dans les deux arbres)
| Arbre | FPS moyen (plafond écran 60 Hz) | CPU rendu / image |
|---|---|---|
| main a8e1cc7a | 58,6 · 58,6 | 0,76 · 0,88 ms |
| SG1 | 58,6 | 0,84 ms |
| SG1 + 4 engins (`--siege-engines=`) | 58,5 | 0,92 ms |
Pas de perte mesurable (l'écran plafonne à 60 Hz même avec `--disable-vsync --max-fps 0`).

## Points ouverts
- Le trébuchet lui-même n'est pas animé (verge qui bascule) : figurine d'engin B1 inchangée.
- Le point d'impact est choisi au tir (le cœur applique les dégâts au tir) : l'effondrement S1
  d'un pan peut précéder de 1-3 s l'arrivée de la dernière pierre.
- Garnison réduite (démo Guyenne, 3 régiments) : rien à replier sur la place ; visible avec
  `--units=10`.
- Pas de son dédié pour l'huile (cri `death_groan` réutilisé) ni pour la porte qui éclate (le son
  `wall_collapse` d'AU1 joue quand la porte cède).
- L'IA n'emploie pas encore l'huile comme critère (elle ne garde la porte que par son déploiement).
