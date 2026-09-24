# Lot B7 — Finitions visuelles de bataille

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Suites de B1/B2/B4/B5.
Godot uniquement (`game/`), aucune règle.

## État
| Point | État |
|---|---|
| 1. Fanions de lance couchée | **fait** (shader, sans régénérer les .glb) |
| 2. Repères d'unité regroupés de loin | à faire |
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

## Prochaine étape
Point 2 (`battle_unit_markers.gd`).
