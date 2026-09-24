# Lot BV1 — Bataille vivante (1) : volées, sang au sol, poussière, taille des unités

Branche `worktree-agent-a3fb69eecbaf4f8a1`. Backlog : `docs/audit/backlog-tw.md` § Bataille (idées J).
Coordination : V2 (soldats VAT) possède le maillage et le shader des soldats ; V3 (feu, fumée,
lumière) possède l'environnement global. BV1 ne touche ni l'un ni l'autre.

## État
| Lot | État |
|---|---|
| 0. Cœur : événements de tir (`ShotEvent`, `get_shots`) + figurines par homme (`figure_positions`) | **fait**, testé (`tests/bv1.rs`) |
| 1. Volées massives (shader, flèches plantées, carreaux, feu, sifflement) | à faire |
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

## Prochaine étape
Lot 1 : `game/scripts/battle/battle_volleys.gd` + `game/shaders/battle_volley.gdshader`.
