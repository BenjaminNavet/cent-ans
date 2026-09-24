# Lot B8b — suites visuelles de bataille (minicarte, boue, gué, pastilles)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Partie visuelle
(Godot uniquement) des points 2-4 relevés par `docs/wip/b8-suites-bataille.md` (partie IA, branche
`worktree-agent-a7845880a6ea35c48`, fusionnée à part). Aucune règle (`core/`) touchée sauf nécessité.

## État
| Point | État |
|---|---|
| 1. Minicarte : haies/fossés/clôtures, village, côte | **fait** |
| 2. Boue (piétinement réutilisé, sol détrempé/pluie) | **fait** |
| 3. Gué : sillage d'écume + gerbes renforcées | en cours |
| 4. Infobulle des pastilles regroupées | à faire |
| Captures avant/après, banc perf | à faire |

## 1. Minicarte
`battle_minimap.gd` `_draw()` : lit `_terrain.obstacles[{a,b,kind}]` (haie = vert foncé, fossé =
brun, clôture = beige clair, `draw_line`), `_terrain.village{x,z,radius,houses[...]}` (disque
d'emprise + un point par maison), `_terrain.coast{shore_x}` (trait vertical le long du rivage,
en x = shore_x sur toute la profondeur). Données déjà exposées par `get_terrain()` côté pont
(`core/crates/godot-bridge/src/battle_sim.rs`, non modifié).

## 2. Boue
Le piétinement (B7, `battle_terrain.gd`) était activé seulement `snowy()`. Ajout `muddy()`
(pluie ou site de sol boueux) ; `_setup_trample()` construit la carte L8 aussi dans ce cas
(coût nul si ni neige ni boue). Le shader (`battle_ground.gdshader`) traitait déjà la branche
« piétinement hors neige » (assombrissement `vec3(0.7,0.62,0.55)`, ligne existante depuis B7,
jamais activée en pratique car `trample_on` restait à 0 hors neige) : renommé `mud_trample`,
inchangé dans le calcul de teinte. Ajout : la boue piétinée est plus luisante (roughness réduite
via `mud_gloss = wet + mud_trample*0.5`, avant `wet` seul) pour lire « traces sombres et
luisantes » plutôt qu'un simple assombrissement mat.

## Prochaine étape
3. Gué (`battle_effects.gd`, fonction `_wet_span`/émetteurs de gerbes B7) : ajouter un sillage
   d'écume traînant derrière chaque cheval dans l'eau, et renforcer l'émission quand une charge
   entre dans l'eau. Cas de test fourni : `--shot-at=56 --camera=718,372,60,165`.
4. Pastilles (`battle_unit_markers.gd`) : l'infobulle actuelle dit juste « N régiments —
   M hommes » ; lister les régiments (nom, effectif, moral) — la liste est déjà disponible au
   moment du regroupement glouton.
5. Captures `docs/img/b8/` avant/après pour 1-4 ; banc `--benchmark --bench-at=90`
   (et `--units=20`) avant/après.

## Tests
`godot --headless --path game --script res://tests/smoke.gd` à relancer après chaque point
(compter « smoke OK », 23 attendues, aucune `SCRIPT ERROR`). Pas de build core nécessaire pour
les points 1-2 (aucun fichier `core/` touché).
