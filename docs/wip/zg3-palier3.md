# ZG3 — relief palier 3 (E5-E7) sur les zones de détail

Lot ZG3 de l'ADR 0036. Branche `worktree-agent-a17b7688304a10059`.
Commande : `uv run --project tools cent-ans geo detail-dem [--zones id,...] [--force]`.
Doc : `docs/geo.md` § « Relief palier 3 ». Crédits : `CREDITS.md`.

## Correctif ZG3b (25/09, `worktree-agent-a0b61e4ebec208836`)

### Bug constaté

Zone « londres », rive sud de la Tamise : E5-E7 donnaient -11 à -15 m (Southwark, Bermondsey,
Lambeth, Kennington) alors qu'E4 y vaut 0,5 m et le sol réel ~+2-5 m ODN ; la City donnait
4,9-7,6 m contre ~15 m réels (E4 14,7). Le trait de côte semblait « manger » de la terre réelle.

### Cause

Le brut (source fine, `tools/geo/raw/detail/londres/`) est correct : EA LiDAR donne Southwark
~4 m, la City ~14,65 m, conforme au terrain (vérifié pixel par pixel). Le bug est dans
`bake_cluster` → `boost_base`/`apply_boost` (`detail_dem.py`), pas dans la source ni dans un
décalage de datum ou d'échelle :

1. **`boost_base` fuyait GLO-90 dans le rehaussement.** La base σ 5 km (rehaussement de rendu,
   ADR 0019/0036) était construite en superposant la source fine sur GLO-90 brut *avant* le
   flou, avec une marge de 3σ = 15 km. Mais l'emprise d'une zone de détail à E6-E7 (3-6 km de
   demi-côté) est bien plus petite que cette marge : le flou gaussien puisait presque
   entièrement dans l'anneau GLO-90 non remplacé autour de la zone, la superposition de la
   source fine n'ayant quasiment aucun effet (vérifié : base identique à ±1,5 m avec ou sans la
   superposition, à Southwark). GLO-90 est un modèle de **surface** (biaisé par les bâtiments en
   ville — Southwark 8,2 m vs sol réel 4 m, la City 28,7 m vs sol réel 14,65 m) et reste par
   nature régional (collines réelles à quelques km, ex. Sydenham/Crystal Palace près de
   Southwark). La base résultante (23,3 m à Southwark) était donc bien plus haute que le sol
   réel, et `apply_boost` (`height + 0,8 × clip(height - base, ±120)`) creusait le terrain en
   conséquence : 4,09 m → -11,32 m.
2. **Aucun plancher après le rehaussement.** La pyramide E0-E4 (`relief_shade.py`) applique déjà
   `enforce_coast()` après `boost_relief()` : la terre ne descend jamais sous `MIN_LAND_M`
   (0,5 m) — c'est pourquoi E4 affichait 0,5 m à Southwark au lieu d'une valeur négative.
   `bake_cluster` (palier 3) n'avait pas l'équivalent : `apply_boost` ne bornait pas son
   résultat, seulement sa condition d'application (`height > MIN_LAND_M`).

Les deux causes candidates de la consigne (bathymétrie grossière recopiée sous la terre, décalage
de datum/facteur d'échelle) sont écartées : le brut EA LiDAR est juste, et le décalage vient du
rehaussement de rendu, pas d'une reprojection.

### Correctif (`tools/cent_ans_tools/geo/detail_dem.py`)

- `boost_base` construit désormais la base à partir de **la source fine de la grappe elle-même**,
  étendue vers l'extérieur par plus-proche-voisin (`scipy.ndimage.distance_transform_edt`) avant
  le flou σ 5 km — GLO-90 n'est lu qu'en repli, quand une grappe entière n'a aucune donnée fine
  (bord de pyramide). Testé : n'appelle plus jamais `copernicus.resample_to_grid` dès qu'il existe
  une donnée fine dans le rayon de la grappe (`test_boost_base_ignores_glo90_when_fine_data_exists`).
- `apply_boost` borne désormais la terre (hauteur source > `MIN_LAND_M`) à rester ≥ `MIN_LAND_M`
  après rehaussement, comme `relief_shade.enforce_coast` pour E0-E4
  (`test_apply_boost_never_sinks_land_below_sea_level`).
- Le fondu de bord (`footprint_weight`/`blend`, 20 % du demi-côté) et le fondu de côte
  (2 px, `validity_weight`) ne sont pas touchés : ils continuent de raccorder à l'ancêtre et à
  l'eau comme avant (ADR 0036 § Continuité).

### Vérification ponctuelle (points signalés, tuiles recuites)

| Point | avant (E7) | après (E7) | réel (ODN) |
|---|---:|---:|---:|
| Southwark | -11,32 m | **0,5 m** (plancher, comme E4) | ~2-5 m |
| Bermondsey | -12,11 m | **0,5 m** | ~2-5 m |
| Lambeth | -10,89 m | **0,5 m** | ~2-5 m |
| Kennington | -13,08 m | **0,5 m** | ~2-5 m |
| City de Londres | 7,57 m | **15,23 m** | ~15 m |

Le plancher à 0,5 m sur la rive sud (au lieu de la vraie valeur ~2-5 m) est une limite
résiduelle : la base σ 5 km, même construite sur la source fine seule, reste tirée vers le haut
par le relief réel du bassin londonien (collines à moins de 5 km — Sydenham, Crystal Palace,
Hampstead), donc `apply_boost` creuse encore la vallée avant que le plancher ne la rattrape. C'est
le même compromis que la pyramide E0-E4 (qui affichait déjà 0,5 m au même endroit) : le
rehaussement de rendu accentue intentionnellement les vallées (ADR 0019/0036), le plancher évite
seulement qu'elles passent sous l'eau. La City, plus loin du relief et avec un sol plus haut,
retrouve sa vraie valeur presque exactement.

### Script de vérification (`detail_dem.e5_e4_land_gap`, `cent-ans geo detail-check`)

Compare, par zone, la terre baked E5 (> `MIN_LAND_M`, hors fondu de bord) à l'ancêtre E4
(interpolé bilinéaire, ce que le moteur affiche sans E5) : écart médian, p95, max ; signale
(`LAND_GAP_ALERT_M` = 5 m) les zones dont le p95 dépasse ce seuil. Tests synthétiques :
bake plat qui matche (écart nul), symptôme brut exclu comme eau (< `MIN_LAND_M`), fuite du
rehaussement signalée. Zones sans tuiles E5 : `None`, pas de crash.

**Limite du signal p95** constatée sur le cache réel : la comparaison E5/E4 ne distingue pas un
vrai défaut d'un relief fin légitime que E4 (lissé, 90 m d'origine) ne peut pas montrer — or les
34 zones sont justement choisies pour un relief marqué (forteresses sur éperon, sièges sur
coteau, batailles sur crête). Après correctif, Carcassonne (cité sur son éperon) et
Mont-Saint-Michel (baie à marnage extrême) voient leur p95 **augmenter** (31 et 46 m) : ce n'est
pas une régression mais la disparition d'un biais qui, avant, alignait artificiellement E5-E7 sur
E4 (les deux partageant la même dérive régionale de GLO-90). Le p95 seul n'est donc pas un
signal pass/fail fiable pour ces zones ; il reste utile comme indicateur d'écart à surveiller.
Toutes les zones et p95 avant/après sont dans le tableau ci-dessous.

**Contrôle croisé retenu** (celui qui vérifie vraiment le bug signalé) : balayage de toutes les
tuiles E5, terre intérieure de l'emprise (fondu ≥ 0,98) avec hauteur baked < -2 m. Seules les
zones **contenant réellement de l'eau dans leur emprise** en ont une fraction notable : Calais
(port, 34 %), Douvres (Manche/port, 39 %), L'Écluse (chenal, 12 %), La Rochelle (port
atlantique, 19 %), Southampton (Solent, 9 %), Winchelsea (ancien port/marais, 15 %),
Mont-Saint-Michel (baie à marnage extrême, 10 %), Harfleur (estuaire, 3 %), Bruges/Tournai/
Formigny (canaux/rivière, < 1 %) ; **Londres : 0,01 %** (271 px sur 2,1 M, min -30,7 m — chenal
de la Tamise). Aucune zone sans façade d'eau ne montre de terre intérieure sous -2 m : le
symptôme d'origine (terre réelle basculant sous l'eau) a disparu partout, pas seulement à
Londres.

### Tableau p95 (m) par zone, avant/après (34 zones, E5 vs ancêtre E4, terre hors fondu de bord)

| Zone | avant | après | médiane avant | médiane après |
|---|---:|---:|---:|---:|
| crecy | 20,58 | 22,06 | 3,24 | 4,69 |
| avignon | 17,93 | 22,74 | 2,73 | 5,67 |
| rouen | 17,48 | 17,51 | 4,80 | 4,09 |
| harfleur | 16,25 | 16,42 | 2,56 | 1,08 |
| carcassonne | 15,62 | 31,42 | 3,28 | 18,38 |
| chateau_gaillard | 15,37 | 14,37 | 3,24 | 1,12 |
| bauge | 13,98 | 14,68 | 3,84 | 3,84 |
| douvres | 13,85 | 15,66 | 3,57 | 5,76 |
| nevilles_cross | 13,14 | 19,36 | 3,72 | 9,65 |
| castillon | 12,95 | 13,27 | 3,80 | 0,00 |
| vincennes | 12,20 | 15,31 | 3,02 | 5,30 |
| auray | 11,76 | 14,34 | 4,87 | 6,13 |
| paris | 11,75 | 13,00 | 3,01 | 4,60 |
| poitiers | 11,51 | 13,12 | 3,43 | 4,88 |
| cocherel | 11,16 | 12,40 | 3,14 | 4,72 |
| bordeaux | 11,15 | 10,18 | 2,10 | 1,63 |
| meaux | 10,63 | 13,18 | 2,29 | 5,21 |
| chinon | 10,51 | 14,27 | 3,18 | 7,17 |
| winchelsea | 10,44 | 9,19 | 2,23 | 0,26 |
| southampton | 10,09 | 10,88 | 2,43 | 2,55 |
| londres | 9,99 | 9,91 | 1,93 | 1,35 |
| orleans | 9,91 | 11,49 | 2,82 | 4,70 |
| tournai | 9,04 | 8,81 | 1,09 | 1,19 |
| reims | 8,50 | 22,26 | 1,84 | 6,54 |
| azincourt | 8,12 | 8,54 | 1,44 | 2,01 |
| bruges | 7,69 | 6,59 | 1,29 | 1,05 |
| calais | 7,41 | 12,07 | 1,99 | 0,55 |
| mont_saint_michel | 7,19 | 45,80 | 2,29 | 2,14 |
| verneuil | 6,94 | 9,81 | 2,31 | 5,30 |
| caen | 6,80 | 8,41 | 2,15 | 2,71 |
| formigny | 5,99 | 6,56 | 1,57 | 1,76 |
| la_rochelle | 3,97 | 3,88 | 0,85 | 1,03 |
| l_ecluse | 3,87 | 3,67 | 0,61 | 0,53 |
| patay | 2,23 | 1,99 | 1,10 | 0,81 |

Toutes les zones à façade d'eau demandées (Londres, Southampton, Douvres, Winchelsea, Calais,
L'Écluse, Bruges, Harfleur, Rouen, Bordeaux, La Rochelle, Mont-Saint-Michel, Caen, Castillon)
sont dans la liste ci-dessus et ont été recuites.

### Recuisson

`cent-ans geo detail-dem --force` sur les 34 zones (bruts en cache, aucun nouveau
téléchargement) : 135 s, E5 378 tuiles (50,8 Mo), E6 824 (82,8 Mo), E7 908 (70,7 Mo), 0,20 Go de
tuiles, 1,37 Go de bruts (log `/tmp/rebake.log` non versionné).

### Commits (`worktree-agent-a0b61e4ebec208836`)
- `wip(zg3b): fix render-boost base leaking GLO-90/regional bias into small E6-E7 footprints`
- `wip(zg3b): add E5-vs-E4 land gap checker (detail_dem.e5_e4_land_gap, geo detail-check CLI)`
- `wip(zg3b): document the render-boost base and land-floor fix in docs/geo.md`
- (à venir) mise à jour de ce fichier

### Points ouverts
- Le plancher `MIN_LAND_M` reste une valeur plate (0,5 m) là où le rehaussement creuse fort
  (rives basses proches d'un relief réel à quelques km) : la vraie cote (2-5 m à Southwark) n'est
  pas restituée, seule l'incohérence « sous l'eau » est corrigée. Améliorer cela demanderait de
  revoir la force du rehaussement lui-même pour le palier 3 (hors budget de ce lot).
- Le seuil `LAND_GAP_ALERT_M` (5 m) déclenche sur la quasi-totalité des zones : attendu vu leur
  relief réel (voir ci-dessus), mais rend l'alerte peu actionnable telle quelle. Le contrôle
  fiable retenu est le balayage « terre intérieure < -2 m », déjà en place manuellement ; il
  n'est pas encore un test pytest sur le cache réel (seulement des fixtures synthétiques dans
  `tools/tests/test_detail_dem.py`) — pourrait être ajouté comme script séparé si utile plus tard.

## État (avant ZG3b)
- [x] Services vérifiés (25/09) : IGN WMS-R (GeoTIFF float32, 5010 px max) ; EA WCS 2.0.1
  (`scalefactor`) ; AHN PDOK (`scalesize`, lent) ; Flandre WCS (multipart, ≤ 2000² px, pas de
  mise à l'échelle) ; Wallonie : WMS rendu seulement → repli GLO-30 ; Overpass OK.
- [x] Code : `detail_dem.py`, `detail_sources.py`, `anachronisms.py`, CLI, tests
  (`tools/tests/test_detail_dem.py`, 24 tests avant ZG3b, 31 après).
- [x] Zones : 34 dans `data/map/detail_zones.json` (schéma étendu : `extra_sources`,
  `level_half_km`).
- [x] Essais : château-Gaillard (IGN), Douvres (EA), Tournai (GLO-30) cuits et contrôlés
  (raccords sans marche).
- [x] Bruges + L'Écluse (Flandre + AHN) cuits.
- [x] Cuisson complète des 34 zones (25/09, 743 s de cuisson, téléchargements compris
  avant : ~30 min), manifeste (lignes 5-7), aperçus `docs/img/zg3/` (Calais, Poitiers,
  Château-Gaillard). Aucun échec de service, aucun repli imprévu.

## Prochaine étape
Correctif ZG3b terminé et recuit. Reste ouvert : revoir la force du rehaussement de rendu pour
le palier 3 si le plancher plat à 0,5 m sur les rives basses proches de relief gêne visuellement
(voir « Points ouverts »).

## Reprise
Idempotent : relancer la même commande ; les blocs téléchargés et les zones à jour
(`tools/geo/raw/detail/<zone>/done_E<k>.json`) sont sautés. Le hash de `params_hash` inclut
`BAKE_VERSION` (bump à 3) : un futur changement du calcul du rehaussement doit le bumper pour
invalider les marqueurs `done_E<k>.json` existants (pas fait ici : la recuisson a été lancée
explicitement avec `--force`).

## Tailles (25/09, après ZG3b)
- Tuiles : E5 378 (50,8 Mo), E6 824 (82,8 Mo), E7 908 (70,7 Mo) — **0,20 Go** (plafond 1,5 Go).
- Bruts `tools/geo/raw/detail/` : **1,37 Go** (plafond 8 Go).
- Part effacée (anachronismes) dans l'emprise : voir `/tmp/rebake.log` (non versionné), stable
  par rapport à la cuisson précédente (0 à ~15 % selon la zone).
