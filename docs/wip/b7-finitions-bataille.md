# Lot B7 — Finitions visuelles de bataille (terminé, non fusionné)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Suites de B1/B2/B4/B5.
Godot uniquement (`game/`), aucune règle.

## État
| Point | État |
|---|---|
| 1. Fanions de lance couchée | **fait** (shader, sans régénérer les .glb) |
| 2. Repères d'unité regroupés de loin | **fait** (smoke : contrôle ajouté) |
| 3. Neige en relief | **fait** (congères, pentes bleutées, neige piétinée) |
| 4. Éclaboussures de gué vérifiées | **fait** (quasi invisibles avant : corrigé) |
| Mesures perf avant/après | **faites** |

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

## 3. Neige
`battle_ground.gdshader` : quand la couche neige domine, congères étirées par le vent (bruit
anisotrope à deux échelles, 0,84-1,06 de blancheur), neige plus mince et bleutée sur les pentes
(ombre de lecture), grain fin. Essayé puis retiré : ondulations dans la carte de normales (le sol
ressemblait à une mer agitée). **Neige piétinée** : `battle_terrain.gd` tient une carte L8
(4 m le texel, rectangle des splatmaps) où chaque régiment présent imprime son emprise orientée
toutes les 0,5 s de bataille (plus fort en marche) ; le shader la lit (`trample_map`) : neige tassée
grise, terre qui affleure par taches là où c'est saturé. Aucun coût hors neige au sol (pas de carte).
Appel : `terrain.update_trample(units, dt)` au début de `_update_effects` (`battle_scene.gd`).
Captures : `before_snow.png` / `after_snow.png` (`--season=winter --ground=snowy --terrain=hills
--weather=clear`), `before_snow_weather.png` / `after_snow_weather.png` (`--weather=snow`).

## 4. Éclaboussures
Cas fabriqué : `game/tests/b7_ford_probe.gd` (bataille de démo sans rendu, même graine que la scène
autonome) liste les régiments en mouvement dont l'emprise touche la rivière. Dans la démo, les
chevaliers français la passent au galop vers 50-60 s en (726, 390) (hors des deux gués, x 338 et
868 : ils traversent à gué-rivière), l'infanterie vers 100-160 s en (577, 390) et (617, 390).
Constat (`before_ford_splash.png`, `--shot-at=56 --camera=718,372,60,165`) : quelques bouffées
pâles à peine visibles. Causes : (1) l'émetteur naissait au lit de la rivière, ~0,5 m **sous**
la surface de l'eau ; (2) seul le centre du régiment comptait (mouillé 1 à 2 s sur une rivière
de 18 m) et la boîte d'émission couvrait toute l'emprise, rives comprises ; (3) 64 particules
pâles pour 60 cavaliers. Corrections (`battle_effects.gd`) : partie mouillée de l'emprise
(5 points de l'arrière à l'avant, `_wet_span`), émetteur limité à cette partie et posé à la
surface ; 160 particules, plus opaques, couleur pleine (non assombrie) ; force minimale 0,6
(cavaliers) / 0,4 (piétons) dans l'eau. Captures : `after_ford_splash.png` (charge), 
`after_ford_infantry.png` (`--shot-at=106 --camera=598,366,70,165`).

## Mesures (24/09, `--disable-vsync --resolution 1600x900 -- --benchmark --bench-at=90`)
« Avant » = fichiers de `51f95be` remis en place le temps de la mesure. Écran plafonné à 60 Hz :
les FPS ne départagent pas (58,7-58,8 partout) ; on compare primitives et appels de dessin.

| Config | Avant : primitives / appels | Après : primitives / appels |
|---|---|---|
| démo 14 unités | 1,38 M / 520 | 1,38 M / 520 |
| `--units=20` (40 unités) | 2,87 M / 895 | 2,87 M / 892 |
| `--units=20 --camera=630,300,750,180` (vue lointaine) | 2,77 M / 831 | 2,77 M / **617** |
| neige (`--season=winter --ground=snowy --terrain=hills --weather=clear`) | 1,46 M / 549 | 1,46 M / 549 |

Lecture : aucun coût GPU mesurable ; les pastilles de groupe **retirent** ~210 appels de dessin 2D
en vue lointaine (moins de repères). Coût CPU du piétinement : une impression toutes les 0,5 s
de bataille (≈ 40 régiments × quelques centaines de texels) et un envoi de texture 500 × 400 L8 ;
non mesurable en FPS. Fanions : quelques instructions du shader de sommets (lances seulement).

## Tests
- Smoke `godot --headless --path game --script res://tests/smoke.gd` : 22 « smoke OK », aucune
  `SCRIPT ERROR` (contrôle B7 des pastilles ajouté dans `_check_battle_markers_b2`).
- `--legacy-figures` : capture OK (fanion ancien vers l'avant, non touché).
- Pas de Rust modifié (cargo non relancé).

## Pistes
- Neige piétinée étendable à la boue (sol détrempé, pluie) : même carte, teinte brune.
- Éclaboussures : gerbes ponctuelles au choc d'une charge dans l'eau ; sillage (écume) derrière
  les chevaux ; les unités traversent souvent la rivière hors des gués (règle de la sim, à voir).
- Pastilles : regroupement par ligne de bataille (avant-garde/bataille) plutôt que par proximité
  écran ; infobulle listant les régiments du groupe.
