# Lot B8b — suites visuelles de bataille (minicarte, boue, gué, pastilles)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Partie visuelle
(Godot uniquement) des points 2-4 relevés par `docs/wip/b8-suites-bataille.md` (partie IA, branche
`worktree-agent-a7845880a6ea35c48`, fusionnée à part). Aucune règle (`core/`) touchée sauf nécessité.

## État
| Point | État |
|---|---|
| 1. Minicarte : haies/fossés/clôtures, village, côte | **fait** |
| 2. Boue (piétinement réutilisé, sol détrempé/pluie) | **fait** |
| 3. Gué : sillage d'écume + gerbes renforcées | **fait** |
| 4. Infobulle des pastilles regroupées | **fait** |
| Captures avant/après, banc perf | à faire |

Smoke intermédiaire (après 1-4, avant captures) : 23 « smoke OK », 0 « SCRIPT ERROR ».

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

## 3. Gué
`battle_effects.gd` : nouveau pool `_wake` (`WAKE_EMITTERS` = 6, réaffecté comme les gerbes)
posé au bord arrière (dans le sens de marche) de la partie mouillée d'un régiment monté
(`_assign_wake`, réutilise `_wet_span` déjà calculé pour les gerbes) ; matériau `Wake*`
(`_process_for`) : émission vers l'arrière et sur les côtés (pas verticale comme une gerbe),
durée de vie plus longue (1,6 s), traîne d'écume. Gerbe renforcée à l'entrée dans l'eau :
`_track[id]["wet"]` mémorise l'état mouillé image par image (reconstruit chaque image, jamais
perimé) ; à la transition sec → mouillé d'une troupe montée en charge/course, `burst(..., "ford",
2.2 si charge sinon 1.4)` — nouveau pool de rafale `_bursts["ford"]` (matériau `Ford*`, écume
blanche, plus large/rapide qu'une gerbe de gué ordinaire), une fois par entrée dans l'eau (pas à
chaque image tant qu'elle y reste). Cas de test fourni : `--shot-at=56 --camera=718,372,60,165`.

## 4. Infobulle des pastilles
`battle_unit_markers.gd` `_draw_name()` : pour un groupe (`entry["units"].size() > 1`), l'infobulle
liste désormais une ligne par régiment (« Nom — effectif (moral N %) »), plafonnée à 8 lignes
(« … K autres » au-delà) sous l'en-tête « N régiments — M hommes » (séparateur fin). Cas d'une
troupe isolée inchangé.

## Prochaine étape
5. Captures `docs/img/b8/` avant/après pour 1-4 ; banc `--benchmark --bench-at=90`
   (et `--units=20`) avant/après.

## Tests
`godot --headless --path game --script res://tests/smoke.gd` à relancer après chaque point
(compter « smoke OK », 23 attendues, aucune `SCRIPT ERROR`). Pas de build core nécessaire pour
les points 1-2 (aucun fichier `core/` touché).
