# Lot BV1 — Bataille vivante (1) : volées, sang au sol, poussière, taille des unités

Branche `worktree-agent-a3fb69eecbaf4f8a1`. Backlog : `docs/audit/backlog-tw.md` § Bataille (idées J).
Coordination : V2 (soldats VAT) possède le maillage et le shader des soldats ; V3 (feu, fumée,
lumière) possède l'environnement global. BV1 ne touche ni l'un ni l'autre.

## État
| Lot | État |
|---|---|
| 0. Cœur : événements de tir (`ShotEvent`, `get_shots`) + figurines par homme (`figure_positions`) | **fait**, testé (`tests/bv1.rs`) |
| 1. Volées massives (shader, flèches plantées, carreaux, feu, sifflement) | **en cours** : volées, fichées, pieux/pavois, feu, crochet son faits ; captures à finir |
| 2. Sang au sol (gerbes, flaques, traînées, réglage) | à faire |
| 3. Poussière et mottes selon sol et météo | à faire |
| 4. Taille des unités (réglage, ADR 0016, FPS Ultra) | cœur fait ; réglage et mesures à faire |
| Mesures FPS avant/après, captures `docs/audit/captures/bv1/` | à faire |

## Choix
- Événements de tir venus du cœur : `BattleSim::take_shots()` (tir sur régiment ou sur muraille :
  tireur, cible, départ, visée, projectiles lâchés, tués, sorte, feu, couvert pavois/pieux/mur),
  file bornée à 1 024. Pont : `BattleSim.get_shots()`.
- Flèches enflammées : tireur assiégeant dont le type peut allumer un feu (`data/rules/siege_fire.json`,
  `ignition`) — `shoots_fire` dans `sim/fire.rs`, rendu seulement.
- Taille des unités : multiplicateur **visuel** (ADR 0016) : `BattleSim.set_figure_scale(k)`,
  `get_soldier_buffer` rend `round(soldats × k)` figurines serrées dans le rectangle simulé,
  `get_units()[i].figures`.

## Mesures « avant » (25/09, `--resolution 1600x900 -- --benchmark --bench-at=90`)
Le banc coupe désormais la synchro verticale (l'autoload `Settings` la réimposait : 60 Hz partout
dans les lots précédents).
| Config | FPS moyen | médiane | primitives / appels |
|---|---|---|---|
| `--units=20` (40 unités, 4 621 soldats) | 98,7 / 99,1 / 84,0 | 102,9 / 105,0 / 95,7 | 2,86 M / 828 |
| démo 14 unités | 122,2 / 121,5 | 132 / 132 | 1,37 M / 500 |

## Lot 1 (volées)
- `battle_volleys.gd` (`BattleVolleys`, enfant de `BattleEffects`) : paquets de 256 traits
  (`battle_volley.gdshader` + `.gdshaderinc`), 3 traits par homme simulé et par volée × taille
  d'unité, 4 096 au plus ; un sur 5 (≤ 96 par volée) confié à la couche plantée
  (`battle_stuck_arrow.gdshader`, 30 000 max puis remplacement au hasard) ; les autres restent
  fichés 8 s. Même hachage entier (PCG) en GLSL et GDScript (`arrow_landing`).
- Pieux (`stakes`) et pavois (`pavise_cover`, clé ajoutée au pont) plantés à l'avant.
- Son : signal `sound_event` → `BattleScene._on_sound_event` → `BattleAudio.play_at` (AU1) si présent.
- `BattleEffects.update(..., shots)` : les tirs du cœur remplacent la baisse des munitions.

## Prochaine étape
Lot 2 : `battle_blood.gd` (gerbes GPU, flaques/traînées en MultiMesh au sol), réglage `battle/blood`.
