# HC5 — nouveaux massifs, landes et zones humides historiques (vers 1340)

Worktree `../gp-hc5`, branche `feat/hc5`. ADR 0161 (complément du 02/10). Pyramide et
`tools/geo/raw` en liens symboliques vers le checkout principal (lecture seule).

## Mandat
Ajouter ≈ 60-90 massifs / landes et ≈ 30-45 zones humides attestés vers 1340 sur toute la carte
(`data/map/historical_forests.json`, `data/map/wetlands.json`), sourcés, puis recuire
`landcover` → `navgrid` → `colormap` → `horizon` et mesurer l'écart de règles (grille de
déplacement, couvert). Objectif : forêt globale ≤ ≈ 45 % des terres, aucun lieu isolé, aucune
liaison renchérie de plus de 30 % sans correction.

## État
- [x] Squelette : note, schémas étendus à toute la carte (lon −11..61, lat 28..66 ; ils
      s'arrêtaient à lon 16 / lat 35-60).
- [x] Données : 91 massifs, landes et parcours (55 → 146), 45 zones humides (32 → 77). Titres des
      notices Wikipédia vérifiés par l'API (existence de la page, redirections suivies) ; le
      contenu des notices et les ouvrages cités de mémoire n'ont pas été relus en séance.
- [x] Cuisson de référence sans les nouvelles entrées : forêt 39,7 %, défriché KK10 31,9 % (le
      cache KK10 est donc bien lu ; sans lui le défrichement vaudrait 0,6 partout). Dérive par
      rapport au `splat.png` commité : 443 pixels de forêt sur 44 M (lieux déplacés depuis) :
      négligeable, la cuisson est reproductible.
- [x] Trois cuissons `landcover` + `navgrid` avec mesure ; emprises et densités réduites après les
      deux premières (voir « Corrections »). Sorties de la troisième commitées (f336110c6).
- [x] `colormap` + `horizon` (sorties commitées à part).
- [x] Tests : voir « Tests ».
- [x] `docs/geo.md` (jeux de données, effets sur les règles, limite de l'allocation).

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

## Mesure (avant = rasters commités avant HC5 ; après = cuisson 3)
Script hors dépôt (scratchpad `hc5/measure.py`, `compare.py`, `areas.py`) : parts par région
(boîtes lon/lat grossières), grille de déplacement, coût du plus court chemin sur `navgrid.png`
le long de chaque arête terrestre de `settlement_graph.json` (fenêtre de 130 km autour des deux
lieux). Forêt = `splat.png` B > 127 ; marais et étangs « de règle » = `wetlands.png` R ou G > 127 ;
cases = `navgrid.png` (18 forêt, 25 marais) parmi les cases franchissables.

| Région | Forêt (terres) | Marais + étangs | Prés humides | Cases franchissables | Coût moyen | Cases de forêt | Cases de marais |
|---|---|---|---|---|---|---|---|
| Carte entière | 39,8 % → 40,3 % | 0,08 % → 0,24 % | 0,04 % → 0,56 % | 92,3 % → 92,3 % | 15,94 → 15,97 | 33,7 % → 34,0 % | 0,40 % → 0,54 % |
| France et marges | 35,1 % → 35,3 % | 0,72 % → 0,83 % | 0,06 % → 1,42 % | 95,7 % → 95,7 % | 14,33 → 14,37 | 28,4 % → 28,5 % | 0,72 % → 0,82 % |
| Îles Britanniques | 36,8 % → 36,8 % | 1,04 % → 1,06 % | 0,69 % → 0,71 % | 74,6 % → 74,6 % | 14,12 → 14,23 | 34,2 % → 34,2 % | 2,25 % → 2,28 % |
| Ibérie | 35,7 % → 37,4 % | 0,01 % → 0,36 % | 0,00 % → 0,00 % | 98,9 % → 98,9 % | 16,85 → 16,93 | 26,6 % → 28,0 % | 0,59 % → 0,89 % |
| Italie | 29,4 % → 31,2 % | 0,24 % → 0,92 % | 0,00 % → 0,00 % | 95,6 % → 95,6 % | 16,25 → 16,28 | 19,3 % → 20,7 % | 1,88 % → 2,22 % |
| Empire et Europe centrale | 37,1 % → 37,9 % | 0,08 % → 0,55 % | 0,26 % → 0,45 % | 96,0 % → 96,0 % | 15,08 → 15,19 | 31,2 % → 32,0 % | 2,01 % → 2,44 % |
| Est, Balkans, Nord | 52,6 % → 52,8 % | 0,00 % → 0,17 % | 0,00 % → 0,87 % | 93,8 % → 93,8 % | 15,13 → 15,16 | 47,2 % → 47,3 % | 0,09 % → 0,26 % |
| Maghreb et Orient | 20,2 % → 20,8 % | 0,00 % → 0,03 % | 0,00 % → 0,00 % | 88,9 % → 88,9 % | 17,90 → 17,91 | 12,3 % → 12,6 % | 0,24 % → 0,26 % |

- Lieux : 2 147, aucun sur une case infranchissable, aucune composante connexe scindée ;
  `geo navgrid` (mode strict) passe.
- Liaisons : 5 066 arêtes terrestres comparées, coût moyen +0,21 % ; **aucune au-delà de +30 %** ;
  14 au-delà de +15 % (maximum : Buda–Visegrád +28 %, Buda–Esztergom +25 %, Millau–Pézenas +22 %).
- La forêt globale ne monte que de 0,5 point : la plupart des grands massifs ajoutés (Carpates,
  Dinarides, Bohême, Pontique) étaient déjà boisés par KK10 ; l'apport est local (Gargano 0 → 79 %,
  Châtillonnais 3 → 80 %, Reichswald de Nuremberg 9 → 94 %, Sierra Morena 26 → 66 %, Moyen Atlas
  15 → 58 %, Kroumirie 12 → 60 %).

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

## Défaut constaté de l'allocation (non corrigé ici)
`compute_forest` tire la forêt là où un bruit régional (≈ 30 km) dépasse `1 − cible` ; un massif
nommé situé dans un creux de ce bruit reste vide quelle que soit sa densité. Parts boisées
mesurées dans l'emprise :
- massifs du lot R1 (inchangés) : Bière / Fontainebleau 0 %, Sherwood 0 %, Inglewood 1 %,
  Yveline 5 %, Soignes 12 %, Perche 13 %, Laye-Marly 17 %, Arden 28 % ;
- nouveaux : Reichswald de Clèves 5 %, Cantref Mawr 14 %, Strandja 14 %, Maures 17 %, Crimée
  23 %, Solling-Reinhardswald 43 %, Białowieża 44 %, Kampinos 45 %.
Correctif possible (à arbitrer, car il boiserait d'un coup les massifs R1 du cœur de carte et
renchérirait leurs liaisons bien au-delà de 30 %) : dans le masque des massifs nommés, remplacer
le score régional par un score tiré d'un bruit fin propre (graine distincte, rang calculé parmi
les seuls pixels des massifs), l'ouverture morphologique restant en place. Aucun paramètre de
`landcover.py` n'a été modifié dans ce lot.

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
(à compléter)

## Prochaine étape
Relecture visuelle par la session principale (aucune capture faite ici), arbitrage du défaut
d'allocation, `geo anchors-fine` après fusion, puis fusion de `feat/hc5`.

## Points ouverts
- Lacs historiques absents du raster (lac Fucin, Copaïs en eau, Amouq, Karla, Prile) : rendus ici
  comme marais ou omis ; un vrai plan d'eau demanderait un lot `lakes.json` à part (HC2).
- Dépression caspienne (< 0 m) : hors « terres » pour `landcover.py`, donc pas de delta de la
  Volga.
- Marais du Pripiat sans effet sur le déplacement (densité 0,45) : en faire un obstacle est un
  choix d'équilibrage (les trois lieux de Polésie sont dedans).
- Maghreb : aucune zone humide ajoutée (merjas du Gharb écartées faute de source précise).
- `fine_anchors.json` (chaussées des routes en zone humide) non régénéré : voir plus haut.
