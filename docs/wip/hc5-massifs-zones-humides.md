# HC5 — nouveaux massifs, landes et zones humides historiques (vers 1340)

Worktree `../gp-hc5`, branche `feat/hc5`. ADR 0161 (complément du 02/10). Pyramide et
`tools/geo/raw` en liens symboliques vers le checkout principal (lecture seule).

## Mandat
Ajouter ≈ 60-90 massifs / landes et ≈ 30-45 zones humides attestés vers 1340 sur toute la carte
(`data/map/historical_forests.json`, `data/map/wetlands.json`), sourcés, puis recuire
`landcover` → `navgrid` → `colormap` → `horizon` et mesurer l'écart de règles (grille de
déplacement, couvert). Objectif : forêt globale ≤ ≈ 45 % des terres, aucun lieu isolé, aucune
liaison renchérie de plus de 30 % sans correction.

## État (reprise du 02/10 au soir, après arbitrage du joueur : corriger les massifs vides)
- [x] Schémas étendus à toute la carte (lon −11..61, lat 28..66).
- [x] Données : 91 massifs, landes et parcours (55 → 146), 45 zones humides (32 → 77). Titres des
      notices Wikipédia vérifiés par l'API (existence de la page) ; contenu des notices et
      ouvrages cités de mémoire non relus en séance.
- [x] `main` fusionné dans `feat/hc5` (9d20f69c6, GC compris ; aucun intrant de cuisson modifié).
- [x] Allocation corrigée dans `landcover.py` (voir « Correction de l'allocation »), deux tests
      unitaires.
- [x] Recuisson complète `landcover` → `navgrid` → `colormap` → `horizon`, sorties commitées.
- [x] Écart de règles remesuré : aucun lieu isolé, aucune liaison au-delà de +30 %.
- [x] Tests : voir « Cuissons et tests ».
- [x] `docs/geo.md`.

## Ce qui dépend des données ajoutées
- `splat.png` (canal B), `forest_kind.png`, `wetlands.png`, `wetlands_bc1_*.bin`,
  `map.json.wetlands_gpu` : `geo landcover`.
- `navgrid.png` : `geo navgrid` (forêt = 18, marais = 25 pour R ou G ≥ 0,5 ; les prés humides ne
  ralentissent pas).
- `colormap_bc1_*.bin`, `colormap_preview.jpg` : `geo colormap` ; `game/assets/horizon/relief/` :
  `geo horizon` (lit le canal B de `splat.png`).
- `settlement_graph.json` / `settlement_edge_paths.json` : **indépendants** (coûts = km × terrain
  de province, `settlements.py`), non régénérés.
- `geo anchors-fine` : `fine_anchors.py` lit `wetlands.png` pour le tracé fin des routes (coût des
  fonds humides en zone cœur, drapeau « chaussée » partout). Ses sorties dépendent donc des
  nouveaux marais, mais `fine_anchors.json` est modifié par une autre session dans le checkout
  principal : **non relancé ici**, à relancer par la session principale après fusion.

## Mesure (avant = rasters commités avant HC5 ; après = cuisson finale avec allocation corrigée)
Script hors dépôt (scratchpad `hc5/measure.py`, `compare.py`, `areas.py`) : parts par région
(boîtes lon/lat grossières), grille de déplacement, coût du plus court chemin sur `navgrid.png`
le long de chaque arête terrestre de `settlement_graph.json` (fenêtre de 130 km autour des deux
lieux). Forêt = `splat.png` B > 127 ; marais et étangs « de règle » = `wetlands.png` R ou G > 127 ;
cases = `navgrid.png` (18 forêt, 25 marais) parmi les cases franchissables.

| Région | Forêt (terres) | Marais + étangs | Prés humides | Cases franchissables | Coût moyen | Cases de forêt | Cases de marais |
|---|---|---|---|---|---|---|---|
| Carte entière | 39,8 % → 40,0 % | 0,08 % → 0,24 % | 0,04 % → 0,56 % | 92,3 % → 92,3 % | 15,94 → 15,97 | 33,7 % → 33,9 % | 0,40 % → 0,55 % |
| France et marges | 35,1 % → 35,3 % | 0,72 % → 0,83 % | 0,06 % → 1,42 % | 95,7 % → 95,7 % | 14,33 → 14,39 | 28,4 % → 28,5 % | 0,72 % → 0,82 % |
| Îles Britanniques | 36,8 % → 36,2 % | 1,04 % → 1,06 % | 0,69 % → 0,71 % | 74,6 % → 74,6 % | 14,12 → 14,21 | 34,2 % → 33,4 % | 2,25 % → 2,29 % |
| Ibérie | 35,7 % → 36,5 % | 0,01 % → 0,36 % | 0,00 % → 0,00 % | 98,9 % → 98,9 % | 16,85 → 16,92 | 26,6 % → 27,4 % | 0,59 % → 0,90 % |
| Italie | 29,4 % → 30,4 % | 0,24 % → 0,92 % | 0,00 % → 0,00 % | 95,6 % → 95,6 % | 16,25 → 16,28 | 19,3 % → 20,1 % | 1,88 % → 2,23 % |
| Empire et Europe centrale | 37,1 % → 37,6 % | 0,08 % → 0,55 % | 0,26 % → 0,45 % | 96,0 % → 96,0 % | 15,08 → 15,18 | 31,2 % → 31,8 % | 2,01 % → 2,45 % |
| Est, Balkans, Nord | 52,6 % → 52,6 % | 0,00 % → 0,17 % | 0,00 % → 0,87 % | 93,8 % → 93,8 % | 15,13 → 15,16 | 47,2 % → 47,2 % | 0,09 % → 0,26 % |
| Maghreb et Orient | 20,2 % → 20,6 % | 0,00 % → 0,03 % | 0,00 % → 0,00 % | 88,9 % → 88,9 % | 17,90 → 17,91 | 12,3 % → 12,5 % | 0,24 % → 0,26 % |

- Lieux : 2 147, aucun sur une case infranchissable, aucune composante connexe scindée ;
  `geo navgrid` (mode strict) passe.
- Liaisons : 5 066 arêtes terrestres comparées, coût moyen +0,20 % ; **aucune au-delà de +30 %** ;
  18 au-delà de +15 % (Calcar–Clèves +29 %, Henley-in-Arden–Maxstoke +27 %, Varsovie–Wyszogród
  +26 %, Millau–Pézenas +22 %, Niort–Tonnay-Charente +22 %, Bruxelles–Villers +21 %,
  Arundel–Steyning +20 %) ; 5 liaisons baissent de plus de 10 % (massifs qui dépassaient leur
  densité dans une bosse du bruit).
- La forêt globale reste à 40,0 % (plafond indicatif 45 %) : la correction déplace la forêt vers
  les massifs nommés plus qu'elle n'en ajoute (les massifs en bosse de bruit redescendent à leur
  densité, ceux en creux y montent).

## Corrections après mesure
- Cuisson 1 : 19 liaisons au-delà de +30 %. Marais du Pripiat ramené à 0,45 (sous le seuil de
  ralentissement : à 0,6, Pinsk–Tourov–Davyd-Haradok passaient à +110-144 %) ; Empordà, delta du
  Danube, Valli Veronesi, baltas du Danube, Polist-Lovat, Scarpe rétrécis ou décalés ; étangs du
  Forez à 0,4 ; Fucecchio sans le lac de Bientina ; Menzaleh seul (sans Bourlos ni Damiette) ;
  Pilis limité au sud du Danube ; Écouves décalée à l'ouest de la route Alençon–Sées ; Chilterns
  0,6 → 0,4 ; Grands Causses 0,6 → 0,4 (une lande sur colline classe la case en montagne, coût 30).
- Cuisson 2 : 1 liaison (Douai–Marchiennes +35 %) : marais de la Scarpe recentré sur
  Marchiennes–Saint-Amand ; Chilterns encore réduits. Forêt d'Arsouf retirée (la bande aride au
  sud de 32,5° N lui donne un potentiel nul : entrée sans effet).
- Lagune de Venise passée en polygone (l'ellipse tombait en mer).

## Correction de l'allocation (`compute_forest`, arbitrage du joueur)
Défaut : la forêt était tirée là où un rang global (bruit régional ≈ 30 km + préférence de
terrain) dépassait `1 − cible` ; un massif nommé tombé dans un creux du bruit restait vide
(Bière / Fontainebleau 0 %, Sherwood 0 %, Yveline 5 %), et un bruit de lisière plus lent que le
massif le rétrécissait ou le gonflait en bloc (Tronçais).
1. `named_forest_score` : dans l'emprise d'un massif (kind ≠ heath), le score est reclassé parmi
   les pixels de terre de l'emprise ; la part boisée vaut donc la cible moyenne de l'emprise
   (`densité × √potentiel`, moins les essarts), avec les mêmes préférences (pentes boisées
   d'abord, clairières autour des lieux et le long des rivières). Hors emprise, score inchangé.
2. `area_mask(..., centred=True)` pour les massifs : le bruit de lisière est recentré sur sa
   moyenne le long du bord ; la lisière reste bruitée, l'aire nominale est conservée. Landes et
   zones humides gardent l'ancien masque.
3. Trouées d'un pixel bouchées dans les massifs (là où la cible ≥ `NAMED_FILL_MIN_TARGET` = 0,3)
   avant l'ouverture morphologique, qui reste en place.
Inchangés : cible `max(cible, densité × √potentiel)`, essarts (villes 0,9 sur ≈ 3 km, hameaux),
limite des arbres, mer, graine. Hors emprises (dilatées de 6 px) : 5 576 pixels de forêt changés
sur 9,64 M, soit 0,058 % (objectif < 1 %).

Résultat : 131 massifs boisés (hors landes) ; sous 70 % de leur densité : 25 avant, 11 après ;
médiane après / densité = 95 %. Retouches de massifs R1 pour tenir le garde-fou des +30 % :
New Forest retiré de la rive de Southampton Water (Beaulieu–Southampton +33 %), Arden 0,65 → 0,6
(Henley-in-Arden–Maxstoke +30 %).

| Massif | Densité | Part boisée avant → après | Après / densité | Sous 70 % : cause |
|---|---|---|---|---|
| orleans | 0,90 | 0,61 → 0,85 | 95 % |  |
| biere | 0,90 | 0,00 → 0,78 | 86 % |  |
| cuise | 0,90 | 0,98 → 0,92 | 102 % |  |
| retz | 0,90 | 0,86 → 0,83 | 93 % |  |
| yveline | 0,85 | 0,05 → 0,79 | 93 % |  |
| laye_marly | 0,80 | 0,17 → 0,48 | 60 % | essarts : 39 % de l'emprise à moins de 3 km d'une ville |
| lyons | 0,90 | 0,42 → 0,87 | 96 % |  |
| rouvray_roumare | 0,80 | 0,55 → 0,78 | 97 % |  |
| eu | 0,85 | 1,00 → 0,94 | 110 % |  |
| perche | 0,75 | 0,13 → 0,69 | 93 % |  |
| berce | 0,85 | 1,00 → 0,89 | 105 % |  |
| broceliande | 0,80 | 0,55 → 0,70 | 87 % |  |
| double | 0,75 | 0,94 → 0,76 | 102 % |  |
| braconne | 0,85 | 0,71 → 0,65 | 76 % |  |
| troncais | 0,85 | 0,50 → 0,79 | 93 % |  |
| sologne | 0,50 | 0,42 → 0,59 | 119 % |  |
| othe | 0,85 | 0,92 → 0,81 | 95 % |  |
| der | 0,80 | 0,68 → 0,75 | 93 % |  |
| argonne | 0,90 | 0,75 → 0,81 | 90 % |  |
| ardenne | 0,80 | 0,84 → 0,71 | 89 % |  |
| charbonniere_soignes | 0,85 | 0,12 → 0,72 | 85 % |  |
| mormal | 0,85 | 0,98 → 0,88 | 103 % |  |
| haye | 0,85 | 0,54 → 0,77 | 91 % |  |
| vosges | 0,85 | 0,80 → 0,83 | 98 % |  |
| palatinat | 0,85 | 0,81 → 0,77 | 90 % |  |
| foret_noire | 0,85 | 0,82 → 0,81 | 95 % |  |
| hunsruck | 0,75 | 0,93 → 0,79 | 105 % |  |
| eifel | 0,60 | 0,94 → 0,74 | 123 % |  |
| chaux | 0,90 | 0,92 → 0,82 | 91 % |  |
| jura | 0,75 | 0,90 → 0,78 | 104 % |  |
| morvan | 0,80 | 0,84 → 0,89 | 111 % |  |
| sherwood | 0,70 | 0,00 → 0,71 | 101 % |  |
| new_forest | 0,60 | 0,45 → 0,65 | 108 % |  |
| dean | 0,90 | 0,87 → 0,80 | 89 % |  |
| weald | 0,70 | 0,47 → 0,58 | 82 % |  |
| arden | 0,60 | 0,28 → 0,54 | 90 % |  |
| epping_waltham | 0,75 | 0,82 → 0,86 | 115 % |  |
| windsor | 0,65 | 0,35 → 0,69 | 106 % |  |
| savernake | 0,80 | 0,96 → 0,87 | 109 % |  |
| wychwood | 0,70 | 1,00 → 0,71 | 102 % |  |
| rockingham | 0,65 | 0,81 → 0,61 | 94 % |  |
| cannock | 0,60 | 0,93 → 0,63 | 105 % |  |
| galtres | 0,65 | 0,96 → 0,71 | 109 % |  |
| inglewood | 0,65 | 0,01 → 0,34 | 53 % | province de terrain « lande » (potentiel 0,55) |
| ettrick | 0,55 | 0,86 → 0,52 | 95 % |  |
| caledonie | 0,40 | 0,58 → 0,26 | 64 % | limite des arbres et hautes terres océaniques |
| chize_argenson | 0,75 | 0,92 → 0,80 | 107 % |  |
| gresigne | 0,90 | 0,70 → 0,83 | 92 % |  |
| iraty | 0,85 | 0,96 → 0,78 | 91 % |  |
| chartreuse | 0,85 | 0,90 → 0,89 | 105 % |  |
| ecouves | 0,85 | 1,00 → 0,94 | 111 % |  |
| crecy | 0,85 | 0,92 → 0,65 | 77 % |  |
| orient | 0,85 | 0,61 → 0,75 | 88 % |  |
| chatillonnais | 0,80 | 0,80 → 0,70 | 87 % |  |
| haguenau | 0,90 | 0,64 → 0,76 | 85 % |  |
| cevennes | 0,70 | 0,83 → 0,79 | 113 % |  |
| maures | 0,75 | 0,17 → 0,48 | 63 % | garrigue (potentiel 0,75) et ouverture morphologique sur un massif étroit |
| glenconkeyne | 0,75 | 0,73 → 0,50 | 67 % | province de terrain « lande » (potentiel 0,55) |
| wicklow_shillelagh | 0,70 | 0,96 → 0,76 | 108 % |  |
| killarney_desmond | 0,60 | 0,83 → 0,57 | 96 % |  |
| cantref_mawr | 0,65 | 0,14 → 0,61 | 93 % |  |
| chilterns | 0,40 | 0,73 → 0,58 | 145 % |  |
| cantabrique | 0,70 | 0,90 → 0,73 | 105 % |  |
| demanda_urbion | 0,75 | 0,99 → 0,81 | 108 % |  |
| systeme_central | 0,65 | 0,78 → 0,69 | 105 % |  |
| monts_tolede | 0,60 | 0,47 → 0,53 | 89 % |  |
| sierra_morena | 0,55 | 0,66 → 0,49 | 89 % |  |
| serrania_cuenca | 0,75 | 0,96 → 0,79 | 105 % |  |
| cazorla_segura | 0,75 | 0,83 → 0,63 | 83 % |  |
| montseny | 0,75 | 0,86 → 0,73 | 97 % |  |
| pinhal_leiria | 0,85 | 0,87 → 0,70 | 82 % |  |
| pineta_classe | 0,85 | 0,61 → 0,48 | 56 % | cordon de 3 px de large sur la côte (dunes, ouverture morphologique) |
| casentino | 0,85 | 1,00 → 0,98 | 116 % |  |
| fiemme | 0,80 | 0,58 → 0,44 | 55 % | 61 % de l'emprise au-dessus de la limite des arbres |
| abruzzes | 0,70 | 0,64 → 0,64 | 91 % |  |
| gargano | 0,80 | 0,79 → 0,73 | 91 % |  |
| pollino | 0,75 | 0,86 → 0,70 | 93 % |  |
| sila | 0,85 | 0,94 → 0,83 | 98 % |  |
| serre_aspromonte | 0,75 | 0,94 → 0,75 | 99 % |  |
| nebrodes | 0,70 | 0,88 → 0,67 | 96 % |  |
| gennargentu | 0,65 | 0,86 → 0,61 | 94 % |  |
| thuringe | 0,85 | 0,98 → 0,90 | 106 % |  |
| harz | 0,85 | 0,76 → 0,76 | 90 % |  |
| spessart | 0,90 | 0,95 → 0,95 | 106 % |  |
| odenwald | 0,80 | 0,99 → 0,92 | 116 % |  |
| solling_reinhardswald | 0,85 | 0,43 → 0,78 | 92 % |  |
| reichswald_cleves | 0,85 | 0,05 → 0,67 | 79 % |  |
| reichswald_nuremberg | 0,90 | 0,94 → 0,92 | 103 % |  |
| foret_boheme | 0,90 | 0,94 → 0,86 | 96 % |  |
| monts_metalliferes | 0,75 | 0,74 → 0,76 | 101 % |  |
| sudetes | 0,80 | 0,97 → 0,84 | 106 % |  |
| wienerwald | 0,85 | 0,74 → 0,74 | 87 % |  |
| bialowieza | 0,95 | 0,44 → 0,70 | 73 % |  |
| grande_wildnis | 0,85 | 0,76 → 0,87 | 102 % |  |
| kampinos | 0,85 | 0,45 → 0,71 | 84 % |  |
| niepolomice | 0,90 | 0,79 → 0,76 | 84 % |  |
| tuchola | 0,80 | 0,88 → 0,77 | 97 % |  |
| rudninkai | 0,90 | 0,90 → 0,93 | 104 % |  |
| samogitie | 0,75 | 0,96 → 0,79 | 106 % |  |
| beskides | 0,85 | 0,91 → 0,84 | 99 % |  |
| carpates_orientales | 0,85 | 0,93 → 0,87 | 103 % |  |
| carpates_meridionales | 0,85 | 0,97 → 0,88 | 103 % |  |
| apuseni | 0,75 | 0,95 → 0,89 | 119 % |  |
| bakony | 0,80 | 0,63 → 0,68 | 85 % |  |
| pilis | 0,80 | 1,00 → 0,86 | 108 % |  |
| codrii_vlasiei | 0,75 | 0,68 → 0,70 | 94 % |  |
| silva_bulgarica | 0,65 | 0,61 → 0,54 | 83 % |  |
| dinariques | 0,75 | 0,93 → 0,77 | 102 % |  |
| grand_balkan | 0,80 | 0,75 → 0,69 | 86 % |  |
| rhodopes | 0,80 | 0,89 → 0,80 | 100 % |  |
| strandja | 0,85 | 0,14 → 0,49 | 58 % | repli steppique de l'Est (potentiel 0,42) |
| pinde | 0,75 | 0,97 → 0,80 | 106 % |  |
| briansk | 0,90 | 0,89 → 0,85 | 95 % |  |
| okovsky | 0,90 | 0,88 → 0,88 | 98 % |  |
| mechtchera | 0,80 | 0,79 → 0,74 | 92 % |  |
| crimee | 0,75 | 0,23 → 0,25 | 33 % | steppe sous 500 m (potentiel 0,21) |
| caucase_ouest | 0,80 | 0,48 → 0,44 | 55 % | limite des arbres et steppe du piémont |
| kolmarden_tiveden | 0,90 | 0,83 → 0,92 | 103 % |  |
| mamora | 0,70 | 0,52 → 0,49 | 70 % |  |
| rif | 0,65 | 0,69 → 0,62 | 96 % |  |
| moyen_atlas | 0,75 | 0,58 → 0,54 | 72 % |  |
| haut_atlas | 0,50 | 0,38 → 0,25 | 50 % | limite des arbres et bande aride |
| kabylie | 0,70 | 0,79 → 0,68 | 97 % |  |
| aures | 0,60 | 0,70 → 0,59 | 98 % |  |
| kroumirie | 0,80 | 0,60 → 0,59 | 74 % |  |
| pontique_ouest | 0,85 | 0,95 → 0,81 | 96 % |  |
| pontique_est | 0,85 | 0,74 → 0,69 | 81 % |  |
| taurus | 0,70 | 0,82 → 0,67 | 95 % |  |
| amanus | 0,75 | 0,86 → 0,73 | 98 % |  |
| liban | 0,50 | 0,57 → 0,45 | 89 % |  |
| troodos | 0,75 | 0,88 → 0,77 | 103 % |  |

## Sources fragiles
- Bois d'Irlande (Glenconkeyne, Wicklow, Desmond) : décrits vers 1600 (McCracken 1971) ; état de
  1340 supposé.
- Grands ensembles sourcés par une notice régionale et non par une attestation médiévale :
  Carpates, Beskides, Dinarides, Grand Balkan, Rhodopes, Pinde, Pontique, Taurus, Rif, Atlas,
  Aurès, Kabylie, Caucase occidental, Crimée, Système central, Jutland (landes), Cévennes.
- Emprises très approchées : Châtillonnais, Silva Bulgarica / Šumadija, Codrii Vlăsiei, Grande
  Solitude, Samogitie, Mechtchera, forêt d'Okov, landes humides de Gascogne, Polésie, marais du
  Polist et de la Lovat (lecture de la chronique de 1238), Uthlande, Hortobágy.
- Pinhal de Leiria : l'attribution des semis à Denis Ier est en partie traditionnelle.
- Ouvrages et sources anciennes cités de mémoire (Jireček 1877, Rowell 1994, Le Roy Ladurie
  1966, McCracken 1971, Dante, Ibn Battûta, al-Idrîsî, Léon l'Africain, Muntaner, chroniques
  russes, Libro de la montería) : références réelles, passages non revérifiés en séance.

## Candidats écartés (volume demandé ≈ 60-90 / 30-45), sourcés, pour un lot ultérieur
Forêts : Andaine, Saint-Gobain, Eawy, Brotonne, Rennes, Gâvre, Moulière, Mervent, Châteauroux,
Cîteaux, Darney, Hardt, Forez-Livradois, Lanvaux, Blois-Boulogne, Vercors, Bouconne, Chinon,
Halatte, Montagne Noire, Corse ; Aherlow, Delamere, Selwood, Lake District, Cheviot, Wyre,
Bowland, Wentwood ; Urbasa, Eume, Gerês, Ports de Beseit, Bardenas, Monegros, Ronda ; Tessin,
Montello, Cansiglio, collines Métallifères, Amiata, Murge, Apennin ligure, Cimins, Tavoliere ;
Dreieich, Taunus, Fichtelgebirge, Schönbuch, Ebersberg, Bienwald, Sachsenwald, Lusace, Drenthe,
Rothaar, Křivoklát, Schorfheide ; Noteć, Sandomierz, Naliboki, Sainte-Croix, Spačva, Ludogorie,
Mátra-Bükk, Codri, Gorski Kotar, Småland, Colchide, Ouarsenis, Ida. Écartés faute de source
solide : forêts de Mourom, forêts mordves, zasseki de Toula (XVIe s.), versant sud des Pyrénées ;
forêt d'Arsouf (sans effet sous la bande aride).
Zones humides : Teufelsmoor, Aischgrund, callows du Shannon, Oristano, Narbonnais, Giannitsá ;
Gharb (source trop générale).

## Cuissons et tests
(à compléter en fin de reprise)

## Prochaine étape
Session principale : relecture visuelle (aucune capture faite ici), `geo anchors-fine` après
fusion, fusion de `feat/hc5`.

## Points ouverts
- Lacs historiques absents du raster (lac Fucin, Copaïs en eau, Amouq, Karla, Prile) : rendus ici
  comme marais ou omis ; un vrai plan d'eau demanderait un lot `lakes.json` à part (HC2).
- Dépression caspienne (< 0 m) : hors « terres » pour `landcover.py`, donc pas de delta de la
  Volga.
- La cible reste `densité × √potentiel` : un massif en steppe, sur la côte, en province de
  terrain « lande » ou au-dessus de la limite des arbres reste sous sa densité (11 cas listés).
- Marais du Pripiat sans effet sur le déplacement (densité 0,45) : en faire un obstacle est un
  choix d'équilibrage (les trois lieux de Polésie sont dedans).
- Maghreb : aucune zone humide ajoutée (merjas du Gharb écartées faute de source précise).
- `fine_anchors.json` (chaussées des routes en zone humide) non régénéré : voir plus haut.
