# CP — carte peinte

## CP0 (infrastructure) : FAIT
Masques de retrait `_clear`, `compose/paint.py` + commande `compose paint [--preview out.png]`, `masks/derived/`,
recompilation complète de l'éditeur, calque `/api/cover/`, schéma `compose_paint.schema.json`, ADR 0361,
tests `test_compose_paint.py` + `test_compose_c4a.py`. Détail : `docs/editeur-carte.md`.

## Prochaine étape (CP1)
4 agents peignent chacun une région dans `data/map/compose/paint/<region>.json` : `ouest`, `mediterranee`, `est`, `orient`
(chaque trait avec une `note` en français). Contrôle : `uv run --project tools cent-ans compose paint --preview x.png`.
Puis, hors agents : `compose paint` + `geo compose-layers --only landcover,colormap` (bake complète, pas dans les tests).

## Points ouverts
- `agri_regions` ne lit pas les masques de champs.
- Bake réelle non lancée en CP0 (mémoire) : à vérifier au premier lot régional.

## Contrôle CP1
`compose paint --check` (4 fichiers, 180 traits) : 0 sommet hors domaine, 0 trait ignoré, 0 chevauchement add/clear entre fichiers
régionaux (ouest/mediterranee dans le Languedoc compris). Intra-fichier (volontaire, retrait dans un ajout) : forest
mediterranee 31 cases, fields orient 4, rock ouest 4. `compose paint --preview` OK (le NaN initial venait de sommets désormais
corrigés ; la rastérisation ignore désormais tout sommet hors domaine avec avertissement). Masques dérivés commités :
`masks/derived/*.png` (forest 20 traits, forest_clear 36, fields 61, fields_clear 7, rock 12, rock_clear 5, marsh 39).
Reste : bake `geo compose-layers --only landcover,colormap`.

## Diagnostic de cause racine (forêt de l'Est vide, Anatolie/Grèce pleines)
**Cause** : `landcover.open_plains` (paramètre `plain_open` = 0,9, lot CC CD, conçu pour la Beauce et la Flandre) rend
« ouverte » toute terre plate et basse, sans regarder le défrichement réel. Sur la carte composée (Europe de l'Est comprise),
la plaine russe, la Carélie, les pays baltes, la Biélorussie et la Pologne sont plates : fenêtres de 1000 px mesurées,
ouverture ≈ 0,98-1,0 (Carélie, Smolensk, Novgorod), contre 0,0-0,1 en Grèce, Meseta, Anatolie (relief). La cible de forêt
y était donc multipliée par 0,1. Comme `forest_share` (0,3) est une moyenne **globale** (`calibrate_share`), le manque à
l'est était reporté sur les terres vallonnées du Sud : facteur de calibrage gonflé, d'où Grèce, Meseta, Maghreb et plateau
anatolien (malgré `dryness`) sur-boisés. Ni projection (lon/lat → px vérifiés au 1e-4 degré près), ni emprise de source
(KK10 couvre -37..69 E, 24..71 N ; Carélie 0,08, Konya 0,37 : correct), ni canal inversé.
**Correction** : `open_plains(..., cleared)` module l'ouverture par `smoothstep(0,35 ; 0,70 ; cleared)` (part défrichée
KK10 vers 1340) ; la plaine n'est ouverte que si KK10 la dit cultivée. Test `test_open_plains_follow_the_kk10_cleared_share`.
**Vérification** (bake complète indépendante, hors dépôt, pic 5,2 Go, 2 min 30) : forêt/terre 0,29 inchangé ;
Carélie 0,00 -> 0,52, Smolensk 0 -> 0,36, Novgorod -> 0,14, Konya 0,65+ -> 0,10 ; moyenne par neuvième : bande nord-est
0,03 -> 0,3-0,55, Sud 0,5-0,6 -> 0,3-0,4. Reste bas (0,05-0,15) : Pologne centrale, Allemagne du Nord, Madrid (KK10 élevé) :
là, la peinture reste utile.
**Bake à lancer** (le checkout principal a le cache KK10 `tools/geo/raw/kk10/`, absent d'un worktree : en copier ou lier
`tools/geo/raw` d'abord, sinon `cleared` vaut 0,6 partout) :
`cd tools && uv run cent-ans compose paint && uv run cent-ans geo compose-layers --only landcover,colormap`
(pic ≈ 5 Go ; ne pas lancer avec une autre suite lourde).
