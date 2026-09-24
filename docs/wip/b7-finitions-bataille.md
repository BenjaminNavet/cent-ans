# Lot B7 — Finitions visuelles de bataille

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Suites de B1/B2/B4/B5.
Godot uniquement (`game/`), aucune règle.

## État
| Point | État |
|---|---|
| 1. Fanions de lance couchée | **fait** (shader, sans régénérer les .glb) |
| 2. Repères d'unité regroupés de loin | **fait** (smoke : contrôle ajouté) |
| 3. Neige en relief | à faire |
| 4. Éclaboussures de gué vérifiées | à faire |
| Mesures perf avant/après | à faire |

## 1. Fanions
Cause : le fanion est modélisé perpendiculaire à la hampe (flottant vers l'arrière, lance droite) et
tourne rigidement avec elle : lance couchée (+1,5 rad), il se dressait à la verticale.
Correction (`battle_soldier.gdshader`, branche `P_WEAPON` en mode lance) : les sommets du fanion
(derrière la hampe, à plus de 2,7 m du poing) sont rabattus le long de la hampe d'un angle
`0,8 × weapon_total` avant la rotation de l'arme ; ondulation au galop. Les figures `--legacy-figures`
(fanion vers l'avant) ne sont pas concernées (condition `d < 0`).
Captures : `docs/img/b7/before_lance_pennon.png`, `after_lance_pennon.png`
(`godot --path game --resolution 1600x900 --script res://tests/b4_figures_shot.gd -- --out=<png> --shot=charge`).

## 2. Repères regroupés
`battle_unit_markers.gd` : `update(..., camera_distance)` ; au-delà de 480 m (retour sous 420 m,
hystérésis), les repères d'un même camp dont l'ancre écran est à moins de 70 px du centre d'un
groupe fusionnent (glouton, ordre des ids) en une pastille : disque du camp sur une petite pile,
nombre de régiments, étoile si le général en est, barres d'effectif et de moral cumulées, clignote
en rouge si un membre fuit. Clic : sélectionne tout le groupe ; survol : « N régiments — M hommes ».
Un régiment isolé garde sa bannière. De près : inchangé. `battle_scene.gd` passe `camera_rig.distance`.
Smoke : vue lointaine simulée (toutes les ancres au même point) → 2 pastilles, clic = tout le camp.
Captures : `before_markers_far.png` / `after_markers_far.png`
(`res://scenes/battle/battle.tscn -- --screenshot=<png> --shot-at=60 --camera=630,300,750,180 --units=12`).

## Mesures « avant » (HEAD 51f95be + fanions, `--disable-vsync --resolution 1600x900`)
| Config (`--benchmark --bench-at=90`) | FPS moy. | Primitives | Appels |
|---|---|---|---|
| démo 14 unités | 58,8 | 1,38 M | 520 |
| `--units=20` (40 unités, 4553 soldats) | 58,8 | 2,87 M | 895 |

## Prochaine étape
Point 3 (neige : `battle_terrain.gd`, `battle_ground.gdshader`).
