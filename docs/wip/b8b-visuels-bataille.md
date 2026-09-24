# Lot B8b — suites visuelles de bataille (minicarte, boue, gué, pastilles) — terminé, non fusionné

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Partie visuelle
(Godot uniquement) des points 2-4 relevés par `docs/wip/b8-suites-bataille.md` (partie IA, branche
`worktree-agent-a7845880a6ea35c48`, fusionnée à part). Aucun fichier `core/` touché.

## État
| Point | État |
|---|---|
| 1. Minicarte : haies/fossés/clôtures, village, côte | **fait** |
| 2. Boue (piétinement réutilisé, sol détrempé/pluie) | **fait** |
| 3. Gué : sillage d'écume + gerbes renforcées | **fait** |
| 4. Infobulle des pastilles regroupées | **fait** (sans capture : survol de souris) |
| Captures avant/après, banc perf | **faits** |

## 1. Minicarte
`battle_minimap.gd` `_draw()` : lit `obstacles[{a,b,kind}]` (haie vert foncé 2 px, fossé brun
1,5 px, clôture beige 1 px), `village{x,z,radius,houses[...]}` (disque d'emprise + un point par
maison), `coast{shore_x}` (trait bleu vertical en x = shore_x). Clés vérifiées dans
`core/crates/godot-bridge/src/battle_sim.rs` (`get_terrain`, non modifié). Retournement (`flipped`)
respecté (tout passe par `to_map`).

## 2. Boue
`battle_terrain.gd` : `muddy()` (pluie, ou sol de site `muddy`) ; `_setup_trample()` crée la carte
L8 si neige **ou** boue (sol sec : aucune texture, `trample_on` = 0, coût nul). En boue
(`_trample_snow` = false) seule la marche imprime (`add = 3 × vitesse − 2`, plafonné à 16 ; à
l'arrêt rien) : sans cela la zone de déploiement devenait un bourbier en une minute.
`battle_ground.gdshader` : hors neige, le piétinement fait apparaître la couche de boue
(`w_mud = max(w_mud, smoothstep(0.3, 0.9, trampled) × …)`), assombrit en brun
(`mud_trample`) et rend le sol plus luisant (`mud_gloss = wet + 0,7 × mud_trample`, rugosité
jusqu'à 0,38 au lieu de 0,42). Branche neige inchangée.

## 3. Gué
`battle_effects.gd` : pool `_wake` (`WAKE_EMITTERS` = 6) posé au bord arrière de la partie
mouillée d'un régiment **monté** (`_assign_wake`, réutilise `_wet_span`) ; particules `Wake*`
émises vers l'arrière et sur les côtés, 1,6 s de vie. Gerbe d'entrée : `_track[id]["wet"]`
(remis à faux chaque image) ; à la transition sec → mouillé d'une troupe montée rapide,
`burst(…, "ford", 2,2 en charge sinon 1,4)`, nouveau pool de rafales `Ford*` (écume blanche,
plus large et plus vive qu'une gerbe). Une fois par entrée dans l'eau.

## 4. Infobulle des pastilles
`battle_unit_markers.gd` `_draw_name()` : pour un groupe, en-tête « N régiments — M hommes »
puis une ligne par régiment « Nom — effectif (moral N %) », 8 lignes au plus (« … K autres »).
Troupe isolée : inchangé.

## Captures (`docs/img/b8/`, fenêtré 1600×900)
« Avant » = fichiers de `19a8873` remis en place le temps de la capture.
- Minicarte : `minimap_before_after.png` (recadrage agrandi ×3, avant à gauche : la ferme et ses
  haies apparaissent en haut à gauche), `before_minimap_full.png` / `after_minimap_full.png`
  (`res://scenes/battle/battle.tscn -- --screenshot=<png> --shot-at=40`). La démo n'a ni côte ni
  village au sens large (ferme), la côte n'est pas illustrée (`--coast` possible).
- Gué : `before_ford.png` / `after_ford.png` (`--shot-at=56 --camera=718,372,60,165`) : écume
  derrière les chevaliers (moitié droite). La gerbe d'entrée a lieu avant 56 s (non capturée).
- Boue : `before_mud.png` / `after_mud.png` (`--ground=muddy --weather=clear --shot-at=110
  --camera=580,300,90,165`) : traces sombres de la marche française vers la rivière.
- Infobulle : non capturée (survol de souris non simulé par les options de capture).

## Mesures (`--disable-vsync --resolution 1600x900 -- --benchmark --bench-at=90`)
Écran plafonné à 60 Hz (58,5-58,9 FPS partout) : on compare primitives et appels.

| Config | Avant : primitives / appels | Après : primitives / appels |
|---|---|---|
| démo 14 unités | 1,38 M / 521 | 1,38 M / 527 |
| `--units=20` (40 unités) | 2,87 M / 896 | 2,87 M / 905 |
| `--weather=rain` (boue active) | 1,47 M / 520 | 1,47 M / 526 |

Lecture : +6 à +9 appels (émetteurs de sillage/gerbes d'entrée actifs), primitives inchangées ;
la boue n'ajoute aucun appel (une texture L8 500×400 de plus, envoyée toutes les 0,5 s).
Mesure « après » faite avant le dernier réglage de la boue (seuils du shader, taux d'impression),
sans effet sur les appels.

## Tests
Smoke final (après tous les réglages) : 23 « smoke OK », 0 « SCRIPT ERROR ».
Pas de Rust modifié.

## Pistes
- Minicarte : les haies de la ferme sont petites à cette échelle ; épaissir si besoin, ou
  n'afficher que les obstacles longs.
- Gerbe d'entrée au gué : capturer vers 50-52 s pour l'illustrer.
- Boue : teinte violacée de la couche boue sous forte humidité ; flaques (normales) possibles.
- Infobulle : capture via un test qui force `hovered`.
