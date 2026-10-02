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
- [x] Données : 92 massifs et landes (55 → 147), 45 zones humides (32 → 77). Titres des notices
      Wikipédia vérifiés par l'API (existence de la page, redirections suivies).
- [x] Cuisson de référence sans les nouvelles entrées : forêt 39,7 %, défriché KK10 31,9 % (le
      cache KK10 est donc bien lu ; sans lui le défrichement vaudrait 0,6 partout). Dérive par
      rapport au `splat.png` commité : 443 pixels de forêt sur 44 M (lieux déplacés depuis) :
      négligeable, la cuisson est reproductible.
- [ ] Cuisson 1 (`landcover` + `navgrid`) et mesure avant / après — en cours.
- [ ] Ajustements d'emprise / densité si une liaison est renchérie de plus de 30 %.
- [ ] `colormap` + `horizon`, tests (pytest, Rust `real_data`, Godot import + smoke), `docs/geo.md`.

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

## Mesure
Script hors dépôt (scratchpad `hc5/measure.py`) : parts par région (boîtes lon/lat grossières),
grille de déplacement, coût du plus court chemin sur `navgrid.png` le long de chaque arête terrestre
de `settlement_graph.json`. Résultats : à venir.

## Candidats écartés (volume demandé ≈ 60-90 / 30-45), sourcés, pour un lot ultérieur
Forêts : Andaine, Saint-Gobain, Eawy, Brotonne, Rennes, Gâvre, Moulière, Mervent, Châteauroux,
Cîteaux, Darney, Hardt, Forez-Livradois, Lanvaux, Blois-Boulogne, Vercors, Bouconne, Chinon,
Halatte, Montagne Noire, Corse ; Aherlow, Delamere, Selwood, Lake District, Cheviot, Wyre,
Bowland, Wentwood ; Urbasa, Eume, Gerês, Ports de Beseit, Bardenas, Monegros, Ronda ; Tessin,
Montello, Cansiglio, collines Métallifères, Amiata, Murge, Apennin ligure, Cimins, Tavoliere ;
Dreieich, Taunus, Fichtelgebirge, Schönbuch, Ebersberg, Bienwald, Sachsenwald, Lusace, Drenthe,
Rothaar, Křivoklát, Schorfheide ; Noteć, Sandomierz, Naliboki, Sainte-Croix, Spačva, Ludogorie,
Mátra-Bükk, Codri, Gorski Kotar, Småland, Colchide, Ouarsenis, Ida. Écartés faute de source
solide : forêts de Mourom, forêts mordves, zasseki de Toula (XVIe s.), versant sud des Pyrénées.
Zones humides : Teufelsmoor, Aischgrund, callows du Shannon, Oristano, Narbonnais, Giannitsá ;
Gharb (source trop générale).

## Prochaine étape
Lire la mesure de la cuisson 1, corriger les emprises qui renchérissent une liaison, recuire.

## Points ouverts
- Lacs historiques absents du raster (lac Fucin, Copaïs en eau, Amouq, Karla, Prile) : rendus ici
  comme marais ou omis ; un vrai plan d'eau demanderait un lot `lakes.json` à part (HC2).
- Dépression caspienne (< 0 m) : hors « terres » pour `landcover.py`, donc pas de delta de la
  Volga.
