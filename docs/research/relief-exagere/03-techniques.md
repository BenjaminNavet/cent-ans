# 03 — Techniques de généralisation et d'exagération du relief

Date : 2026-10-13. Recherche web (budget 0 $). Convention : **[Source]** = fait trouvé dans la source citée ;
**[Inférence]** = déduction de l'auteur de cette note, non vérifiée par une source.
Limites : les PDF de Jenny et al. n'ont pas pu être lus en texte (binaire) ; leurs contenus viennent des résumés de recherche et des pages d'abstract. Les licences non confirmées par une page sont marquées « à vérifier ».

Contexte : heightfield Europe 7168×6144 px, 719 m/px, 16 bits, exagération actuelle ×4,3, caméra de 130 à 900 km de large. Cible : style Total War (plaines plates, chaînes plus hautes et plus étroites, crêtes vives, cols, rivières en fond de vallée, plateformes sous les villes).

## 1. Ce que dit la cartographie

### 1.1 Terrain Sculptor (Leonowicz, Jenny, Hurni 2010)
- **[Source]** Logiciel freeware qui généralise un MNT pour l'ombrage de relief ; suite d'opérations raster, coefficients de courbure pour détecter et accentuer les formes importantes ; les filtres raster standards adoucissent crêtes et bords de vallée, d'où l'approche. https://cartographicperspectives.org/index.php/journal/article/view/cp67-leonowicz-et-al
- **[Source]** Méthode sous-jacente (article Geographia Technica 2010) : lissage passe-bas des détails mineurs, puis deux grilles dérivées du MNT lissé : l'une exagère les crêtes (zones de montagne), l'autre creuse les fonds de vallée (zones basses) ; combinaison pondérée par la courbure. https://technicalgeography.org/index.php/special-issue-2010/40-12-anna-m-leonowicz-bernhard-jenny-automated-small-scale-relief-shading-a-new-method
- **[Source]** Application Java, GPL v2, code source sur GitHub, E/S en ASCII grid, mode batch ; mémoire ≈ lignes×colonnes/15 000 Mo (5000×5000 → 1,65 Go). https://www.terraincartography.colororacle.org/terrainsculptor/index.html
- **[Inférence]** Le schéma « lisser, puis exagérer/creuser selon la courbure » est directement réimplémentable en numpy (Gaussienne + Laplacien/courbure) ; pas besoin d'utiliser le code GPL. Pour éviter toute contamination GPL, réécrire à partir de la description publiée, ne pas copier le Java.

### 1.2 Terrain Equalizer : pyramides multi-échelles contraintes par la courbure (Jenny, Jenny, Hurni 2011)
- **[Source]** Le MNT est traité comme un signal 2D décomposé par **pyramide laplacienne** en bandes de fréquences ; l'utilisateur pondère chaque bande comme un égaliseur ; des indices de courbure protègent crêtes et vallées. Implémentation : application « Terrain Equalizer ». Les bandes avant-plan/arrière-plan peuvent être réglées séparément pour des cartes en perspective 3D. Résumé : https://research.monash.edu/en/publications/terrain-generalization-with-multi-scale-pyramids-constrained-by-c-2/ ; PDF : https://mail.colororacle.org/berniejenny/pdf/2011_Jenny_etal_Terrain_Generalization.pdf
- **[Source]** Autre travail du groupe : U-nets entraînés sur des reliefs ombrés dessinés à la main pour supprimer le détail inutile et accentuer les grandes formes (Jenny et al., AutoCarto 2022) ; revue « Small-scale and multi-scale relief mapping » (Samsonov & Jenny 2024). https://cartogis.org/docs/autocarto/2022/docs/abstracts/Session3_Jenny_0293.pdf ; https://research-repository.rmit.edu.au/articles/chapter/Small-scale_and_multi-scale_relief_mapping/27361440
- **[Inférence]** La pyramide laplacienne est la brique la plus utile pour nous : bandes basses (> ~100 km) = chaînes, bandes moyennes (10-100 km) = massifs et vallées, bandes hautes (< 5 km) = bruit à atténuer à l'échelle campagne. Le « masque de courbure » sert à ne pas amplifier les bandes dans les plaines.

### 1.3 Tom Patterson / Imhof / style suisse
- **[Source]** Patterson, « resolution bumping » : technique Photoshop de manipulation de MNT petite échelle (GTOPO30) pour rendre les montagnes accidentées lisibles ; l'exagération verticale est nécessaire à petite échelle mais aggrave le caractère haché des données brutes, d'où un ré-échantillonnage plus grossier préalable. (Résumé de recherche ; la page elle-même a renvoyé 403.) https://shadedrelief.com/bumping/bumping.html
- **[Source]** Patterson (NPS) : substitution topographique, exagération verticale **sélective**, peinture/filtrage, export du MNT modifié vers un moteur de rendu (Bryce). Voir aussi Prisma Shaded Relief (généralisation SRTM dans Pyramid Shader). https://www.naturalearthdata.com/?p=5025 ; https://carto.geogr.msu.ru/cmc/publications/papers/cartographica/carto_38-1_10_nationalpark.pdf
- **[Source]** Imhof, *Cartographic Relief Presentation* : perspective aérienne — contraste maximal sur les plus hauts sommets, moindre en plaine (imite la brume) ; éclairage du nord-ouest pour éviter l'inversion crêtes/vallées. Relief ombré suisse = ombrage + courbes de niveau + teinte légèrement modulée par altitude et orientation. https://en.wikipedia.org/wiki/Terrain_cartography ; https://mail.colororacle.org/berniejenny/pdf/2006_JennyHurniSwissStyleShading.pdf (URL du résultat de recherche : `2006_JennyHurni_SwissStyleShading.pdf`)
- **[Source]** Terrain texture shading (Leland Brown), présenté sur shadedrelief.com : page non lisible (403). https://www.shadedrelief.com/texture_shading/ ; **[Inférence]** (connaissance générale non vérifiée ici) : il s'agit d'un filtre fractionnel de type Laplacien qui fait ressortir la structure des crêtes/vallées sans éclairage directionnel — utile comme texture de contrôle visuel de nos résultats.

### 1.4 Perception de l'exagération verticale
- **[Source]** Aucune étude trouvée donnant un facteur d'exagération « optimal » en relief ombré. Trouvé : définition (rapport échelle verticale/horizontale, Thurrell 1953, https://www.asprs.org/wp-content/uploads/pers/1953journal/sep/1953_sep_579-588.pdf) ; illustration Penn State de 5×, 10×, 20×, 40× https://courses.ems.psu.edu/natureofgeoinfo/print/c7_p11.html ; les observateurs surestiment systématiquement les pentes (10° perçus ≈ 30°, Proffitt) https://www.sciencedaily.com/releases/1999/10/991028071258.htm.
- **[Inférence]** Les pentes perçues sont déjà gonflées, donc le besoin d'exagération vient surtout de la **taille apparente** des reliefs à 130-900 km de large, pas d'une pente réelle forte. Choisir l'exagération par essai visuel (§ 8) plutôt que par une valeur littérature. Ordre de grandeur Total War : non documenté (recherche sur les cartes de Rome II/Warhammer III infructueuse ; seul fait : l'éditeur Terry génère un impassable au-delà de 45° de pente, https://wiki.totalwar.com/w/TWWAKT_Editing_the_Heightmap).

## 2. Algorithmes : remappage d'altitude

Tout ce qui suit est **[Inférence]** sauf mention. Notations : `Z` MNT réel (m), `B` altitude de base locale.

1. **Séparation de fréquences** : `L_k = G_k - G_{k+1}` (Gaussiennes, `scipy.ndimage.gaussian_filter`) ou pyramide laplacienne (`skimage.transform.pyramid_laplacian`, BSD). Sortie = `base + Σ w_k·L_k`. Plage de w : bandes > 100 km ×1-1,5 ; 20-100 km ×1,5-2,5 ; < 5 km ×0,3-0,8 (dans les montagnes, plus bas en plaine).
2. **Altitude au-dessus du niveau de base local** : `B = ouverture/min filtré` (`scipy.ndimage.minimum_filter` puis Gauss, rayon 30-80 km) ou plus propre : surface passant par les fonds de vallée (altitude des cellules de rivière interpolée). `H = Z - B`. Puis remapper `H' = f(H)`.
3. **Courbe de remappage `f`** : nulle ou compressive en dessous de ~250-400 m (plaines plates : `H' = H·0,3`), puis convexe (puissance `H^p`, p 1,2-1,6, ou sigmoïde/smoothstep par morceaux) pour que les montagnes gagnent beaucoup plus que le piémont. Garder `f` monotone et C1 pour éviter les terrasses. Faire `f` fonction du **masque de montagne lissé** (rugosité locale = écart-type de `Z` sur 20-40 km) plutôt que de l'altitude seule : sinon un plateau (Meseta, Massif central) devient une chaîne.
4. **Chaînes plus étroites** : (a) érosion morphologique « douce » : remplacer `H` par `H - α·(H - opening_σ(H))` n'est pas recommandé (terrasses) ; préférer (b) une **déformation de pente de flanc** : `H' = H · g(d)` où `d` = distance (EDT, `scipy.ndimage.distance_transform_edt`) au masque de crête/ligne de partage, `g` décroît vite avec `d` (profil en lame). (c) Ou un remappage de `H` par puissance plus grande (p) : à altitude égale seuls les sommets survivent, la chaîne paraît plus étroite sans toucher la géométrie en plan. Option (c) est la plus simple et la plus stable ; (b) pour compléter.
5. **Éviter les terrasses** : travailler en flottant 32 bits (jamais en uint8/uint16 intermédiaires) ; remappages continus ; terminer par un léger lissage (σ 0,5-1 px) + dithering ±0,5 LSB avant quantification 16 bits ; contrôler l'histogramme d'altitudes et la dérivée de `f`. Les terrasses viennent surtout des quantifications répétées et des `f` en marches d'escalier.
6. **Côtes** : **[Inférence]** garder les cellules marines intactes (masque terre/mer), et fondre `f` vers l'identité sur 10-20 px près de la côte (smoothstep sur la distance à la mer) ; sinon les falaises décalent la ligne de rivage et créent des murs. Dans les zones où la montagne touche la mer (Norvège, Dalmatie, Alpes-Maritimes) accepter la falaise mais limiter la pente (voir § 7).

## 3. Accentuation des crêtes et vallées

- **[Source]** Terrain Sculptor : grille « crêtes exagérées » + grille « vallées creusées », mixées par courbure (§ 1.1) ; Terrain Equalizer : courbure comme contrainte (§ 1.2).
- **[Inférence]** Recette numpy : `curv = laplacian(gaussian(Z, σ))` (σ 2-6 px ≈ 1,5-4 km) ; crêtes `curv < 0` (convexe), vallées `curv > 0`. Ajouter `Z' = Z - k·clip(curv_normalisée)·masque_montagne` (k donne un effet « unsharp mask » orienté). Un masque de crête binaire (skeleton de `curv < -seuil` par `skimage.morphology.skeletonize`) peut servir d'axe pour le profil en lame (§ 2.4b).
- **Bruit « ridged » guidé** : **[Source]** les procédures « ridged multifractal » (Musgrave) et les cartes d'uplift avec érosion fluviale (Cordonnier 2016, stream power law, https://diglib.eg.org/handle/10.1111/cgf12820 ; version analytique rapide 2024 https://diglib.eg.org/handle/10.1111/cgf15033) existent. **[Inférence]** Pour nous : ajouter du ridged noise (`1 - |noise|`) seulement comme **détail haute fréquence** modulé par `masque_montagne × pente`, avec orientation alignée sur le gradient réel ; jamais comme forme principale (on perdrait la géographie réelle des chaînes — le joueur reconnaît les Alpes, les Pyrénées).
- **[Inférence]** Pitfall : amplifier la courbure amplifie aussi le bruit DEM (artefacts Copernicus : bâtiments, forêts, bandes). Toujours lisser (σ ≥ 1 px à 719 m) avant le calcul et masquer la mer et les lacs.

## 4. Vallées, rivières et hydrologie

- **[Source]** RichDEM : `BreachDepressions` (creusement d'un chenal depuis chaque cuvette vers la sortie) ; WhiteboxTools : `breach_depressions` ; Priority-Flood (Barnes, Lehman, Mulla 2014, O(n) entiers / O(n log n) flottants) pour combler les dépressions. https://richdem.readthedocs.io/en/latest/depression_breaching.html ; https://arxiv.org/pdf/1511.04463 ; https://github.com/r-barnes/Barnes2013-Depressions
- **Licences** : RichDEM GPL-3.0 (à vérifier), WhiteboxTools MIT (à vérifier), pysheds MIT (à vérifier) — non confirmés par les pages lues. Le fichier `Barnes2013-Depressions` : licence non lue. **[Inférence]** Priority-Flood se code en ~40 lignes Python avec `heapq` ; à 7168×6144 (44 M de cellules) c'est lent en Python pur → numba, ou outil externe en lot (whitebox binaire) une seule fois ; ce n'est pas un traitement à rejouer à chaque itération.
- **Rivières existantes** : le dépôt a déjà des rivières (vecteurs). **[Inférence]** Stratégie recommandée : **ne pas recalculer** le réseau depuis le MNT exagéré, mais **graver (stream burning) le MNT final selon les polylignes de rivières de référence** : rasteriser les rivières (largeur 1-3 px), calculer `d` à la rivière (EDT), puis `Z' = min(Z, Z_rivière_lissée + profil(d))` avec profil en V ou en U (largeur de vallée ∝ ordre de Strahler, profondeur 20-150 m équivalents jeu). Garantir l'altitude **monotone décroissante** vers l'aval : propager le minimum cumulatif le long du chemin (`np.minimum.accumulate` sur chaque polyligne ordonnée de l'amont vers l'aval).
- **Cohérence avec l'exagération** : appliquer d'abord le remappage (§ 2), puis la gravure ; sinon le remappage convexe étouffe les vallées. Le fond de vallée doit rester au niveau de base `B` (donc `H' ≈ 0` sur la rivière) ; ainsi la vallée gagne en profondeur relative quand les crêtes montent.
- **Lacs et mers intérieures** : masque d'eau → altitude constante forcée (niveau du lac) ; ne pas laisser l'exagération incliner un lac.
- **Option physique** : **[Source]** Cordonnier et al. partent d'une carte d'uplift et d'un réseau de rivières pour synthétiser la topographie par équation de puissance de l'écoulement. **[Inférence]** Pour nous c'est surdimensionné ; on utilise le MNT réel comme « carte d'uplift » implicite.

## 5. Cols (passes) entre bassins

- **[Inférence, aucune source trouvée]** Objectif : garder des passages jouables et lisibles (ex. Brenner, Saint-Gothard, Roncevaux, Somport, Mont-Cenis, Petit-Saint-Bernard) malgré des crêtes élevées. Méthode :
  1. Définir une liste de cols historiques (lon/lat, `data/`, jamais en dur dans le script) ; l'altitude réelle du col est connue.
  2. Pour chaque col, route de moindre coût (`skimage.graph.route_through_array` avec coût = pente + altitude relative) entre les deux vallées de chaque côté.
  3. **Plafonner** l'altitude le long de la route : `Z' = min(Z', Z_col_cible + c·d_route)` avec profil en selle (EDT de la route, largeur 4-10 px), `Z_col_cible` ≈ `B + 0,25-0,4 · hauteur_crête_locale`.
  4. Lisser la jonction (Gauss σ 1-2 px sur le masque de la selle) pour éviter une tranchée visible ; les bords de la selle doivent remonter en pente douce.
  5. Vérifier par Dijkstra sur la grille de déplacement du jeu (core/) qu'il reste exactement les passages historiques : un col en trop change la stratégie.
- Pitfall : creuser sans plafonner crée des brèches artificielles dans une chaîne ; ne creuser que les cols répertoriés.

## 6. Plateformes sous les villes

- **[Inférence]** Pour chaque ville/place forte (rayon `r` selon taille, 3-8 px ≈ 2-6 km) : calculer l'altitude cible `Z_t` = médiane de `Z'` sur le disque, puis `Z' ← lerp(Z', Z_t, w(d))` avec `w` = smoothstep 1→0 de `r` à `1,8 r`. Pente maximale en bordure ≤ pente de talus choisie. Exécuter **après** gravure des rivières (une ville au bord d'une rivière : coupler `Z_t` à `Z_rivière + marge`). Ne pas aplatir d'un seul trait pour les villes sur versant (Grenoble, Innsbruck) : cibler la médiane de la moitié basse du disque.
- Pitfall : plateformes trop grandes écrasent la vallée ; plafonner `r`.

## 7. Érosion pour rendre le relief exagéré naturel

- **[Source]** `xrspatial.erosion.erode` (xarray-spatial) : érosion hydraulique par particules (numpy/numba, CuPy/Dask en option), paramètres par défaut : inertia 0,05, capacity 4, deposition 0,3, erosion 0,3, evaporation 0,01, gravity 4, min_slope 0,01, radius 3, max_lifetime 30 ; pas d'érosion thermique ; licence non indiquée sur la page (le paquet est publié sous licence à vérifier, **[Inférence]** probablement MIT). https://xarray-spatial.readthedocs.io/en/stable/_modules/xrspatial/erosion.html
- **[Source]** `QuentinWach/terra` : générateur Python avec érosion hydraulique, MIT, immature. https://github.com/QuentinWach/terra
- **[Inférence]** Sur 44 M de cellules l'érosion par particules (≥ 50 000 gouttes par défaut) n'a d'effet que local ; à utiliser par **tuiles** (ex. 1024², recouvrement 64 px) sur les seules zones de montagne, après exagération, avec peu d'itérations (≈ 2-10 gouttes/cellule). Danger : décaler les rivières gravées (§ 4) → graver **après**, ou masquer les cellules de rivière.
- **Érosion thermique** (talus) : à écrire à la main, 15 lignes numpy : pour chaque cellule, si `ΔZ > talus` avec un voisin, déplacer une fraction `c·(ΔZ - talus)/2`. Talus à 719 m/px : tan(θ) × 719 m/px ≈ ΔZ max entre voisins (θ 35-45° jeu ⇒ ΔZ ≈ 500-720 m équivalents, après exagération). Utile pour **adoucir les falaises de côte** et les falaises créées par le remappage. 10-40 itérations, c ≈ 0,3-0,5. Coût vectorisable par décalages (`np.roll`) sur 44 M de float32 : quelques secondes par itération.
- **Pitfall** : l'érosion par gouttes génère des rigoles sub-pixel à 719 m/px qui ne lisent pas à 400 km de large ; son bénéfice se juge à la zone « rapprochée » (130 km) et dans les tuiles fines.

## 8. Lecture visuelle (shading, neige, brume)

- **[Source]** Imhof : éclairage nord-ouest (inversion de relief sinon) ; contraste maximal en haut, atténué en bas (perspective aérienne) ; Terrain Equalizer permet de réduire le détail d'arrière-plan en perspective. Voir § 1.
- **[Inférence]** Pour Godot : (a) lumière directionnelle azimut 300-330° (NO), élévation 35-50° pour que les flancs à pente exagérée ne soient pas entièrement éclairés ou ombrés ; (b) recalculer les **normales** depuis la hauteur **exagérée** et non depuis le MNT original, avec échantillonnage Sobel sur 2-3 px pour éviter l'aliasing ; (c) neige : `snow = smoothstep(h0, h1, H_réel_non_exagéré + bruit) × (1 - smoothstep(35°, 55°, pente))` — baser le seuil sur l'altitude **réelle** (neige au-dessus de ~2200-2800 m aux Alpes selon la latitude) pas sur l'altitude jeu ; roche nue : pente > 40-50° (calculée sur la pente exagérée) ou altitude réelle > treeline ; (d) brume/perspective aérienne : fog dépendant de la distance et de la hauteur (plus dense dans les vallées) ; réduire le contraste des normales avec la distance ; (e) occlusion ambiante de vallée précalculée dans une texture (voir déjà fait : RV/lighting) ; (f) CDLOD : maillage morphé, donc les crêtes fines aliasent à grande distance → conserver des sommets d'au moins 2 px de large en maillage de niveau grossier (voir pitfalls).
- **Vérification** : une vue de contrôle « texture shading » ou Laplacien et une vue hillshade multidirectionnelle (315°, 270°, 360°) pour détecter terrasses et artefacts sans lancer Godot (headless, numpy → PNG).

## 9. Bibliothèques et licences (Python)

| Besoin | Outil | Licence | Statut |
|---|---|---|---|
| Filtres, EDT, morpho | scipy.ndimage | BSD | connue, non revérifiée |
| Pyramide, skeleton, route | scikit-image (`pyramid_laplacian`, `skeletonize`, `graph.route_through_array`) | BSD | connue |
| Dépressions, flux | RichDEM | GPL-3.0 | à vérifier |
| Dépressions, flux | WhiteboxTools / whitebox-python | MIT | à vérifier |
| Flux, bassins | pysheds | MIT | à vérifier |
| Érosion hydraulique | xrspatial.erosion | non indiquée | à vérifier |
| Érosion hydraulique | QuentinWach/terra | MIT | confirmée par la recherche |
| Généralisation de référence | Terrain Sculptor (Java) | GPL v2 | confirmée https://www.terraincartography.colororacle.org/terrainsculptor/index.html |
| Accélération | numba | BSD | connue |

**[Inférence]** GPL : l'utiliser comme **outil externe** de comparaison ne contamine pas le dépôt, mais ne pas copier de code GPL dans `tools/` si le dépôt public est sous une autre licence (cf. mémoire : jamais d'assets NC dans le dépôt).

## 10. Pipeline recommandé (ordre et paramètres, échelle 719 m/px)

Toutes les étapes en float32, une tuile de travail avec recouvrement de ≥ 128 px (≈ 90 km) si la mémoire l'impose ; masques terre/mer/lacs en entrée ; paramètres dans un fichier de données (`data/`), pas en dur.

1. **Chargement + nettoyage** : lire le MNT 16 bits, convertir en mètres réels (float32), figer les masques mer (Z ≤ 0) et lacs. Combler les petites dépressions (Priority-Flood ou whitebox `breach`) hors lacs. Lisser σ = 0,7-1,0 px les artefacts Copernicus.
2. **Base locale `B`** : surface des fonds de vallée = `gaussian(minimum_filter(Z, 61-111 px), 25-40 px)` (≈ 45-80 km). `H = max(Z - B, 0)`.
3. **Masque de montagne `M`** : rugosité (écart-type de `Z` sur 28-56 px ≈ 20-40 km) + `H` ; seuils en smoothstep (rugosité 120-350 m, `H` 300-900 m) ; flou σ 8-15 px pour des transitions piémont douces.
4. **Séparation de fréquences** : 4-5 bandes (σ 1,5 ; 4 ; 10 ; 25 ; 60 px). Poids par bande × `M` : hautes (< 3 km) 0,4-0,8 ; moyennes (3-30 km) 1,2-2,0 ; basses (> 30 km) 1,0-1,5.
5. **Remappage vertical** : `H' = f(H)` avec zone morte plaine (H < 250-400 m compressé à ×0,2-0,4), puis puissance p = 1,3-1,6 normalisée pour que le massif le plus haut atteigne la cible jeu ; facteur global équivalent ×8-12 sur les sommets (contre ×4,3 actuels), ×1-2 en piémont, ×0,3-0,6 en plaine. Valeurs à calibrer par captures (voir budget 5 captures) : **[Inférence]** chiffres de départ, non issus de la littérature.
6. **Accentuation crêtes/vallées** : `curv` sur σ = 2-5 px, `Z -= k·curv_norm·M` avec k équivalent à 5-12 % de la hauteur locale ; profil en lame optionnel autour des axes de crête (largeur ×0,6-0,8).
7. **Cols** : plafonnement en selle sur les routes de moindre coût des cols répertoriés (§ 5), largeur 4-10 px, jonction lissée 1-2 px.
8. **Érosion locale (optionnelle)** : thermique 15-40 itérations (talus ≈ 0,5-0,7 km/px équivalent) pour adoucir les falaises, puis hydraulique par tuiles de 1024² uniquement sur `M > 0,5`, 2-10 gouttes/cellule.
9. **Gravure des rivières** : rasteriser les rivières, profil V/U (demi-largeur 1-4 px selon ordre), monotonie aval par minimum cumulatif, fond de vallée ≈ `B` ; lacs et mers forcés à niveau constant.
10. **Plateformes de villes** : disques r 3-8 px, médiane de la moitié basse, smoothstep jusqu'à 1,8 r.
11. **Côtes** : fondu de `f` vers l'identité sur 10-20 px près du trait de côte ; talus maximal 35-45°.
12. **Sortie** : lissage final σ 0,5-0,8 px, dithering ±0,5 LSB, quantification 16 bits **une seule fois** ; exporter aussi les tuiles fines avec la même fonction de transformation (fonction pure de `Z` réel + masques) pour garantir la continuité aux jonctions de tuiles.
13. **Contrôles automatiques** (sans GUI) : histogramme des altitudes (pas de peignes), pente max par type de zone, hillshade 4 azimuts + texture shading en PNG, test de connectivité des cols (Dijkstra), test de monotonie des rivières, différence vs ancienne carte aux jonctions de tuiles.

## 11. Liste des pièges

1. **Terrasses** : quantification intermédiaire 8/16 bits, `f` en marches, érosion morphologique en paliers. Remède : float32, `f` C1, dithering final.
2. **Plateaux changés en chaînes** : exagérer par altitude absolue. Remède : base locale `B` + masque de rugosité.
3. **Amplifier le bruit DEM** : courbure/ridged sur données non lissées. Remède : lissage préalable, masque de mer/lac.
4. **Côtes déformées** : exagération non fondue près du rivage. Remède : fondu 10-20 px.
5. **Rivières à contre-pente** après remappage ou érosion. Remède : gravure en dernier + minimum cumulatif.
6. **Cols perdus ou inventés** : crêtes surélevées sans plafonnement ; creusement trop large. Remède : liste de cols de données, test Dijkstra.
7. **Jonctions de tuiles** : traitements dépendant de statistiques de tuile (normalisation locale, filtres sans recouvrement). Remède : fonctions pures, recouvrement, coordonnées globales.
8. **CDLOD** : crêtes étroites sous-échantillonnées aux niveaux grossiers → scintillement et « pops » ; hauteurs max d'AABB sous-estimées si on morphe. Remède : minimum/maximum conservés dans la pyramide de hauteurs, ou générer les LOD grossiers en préservant les maxima en montagne ; vérifier les AABB après exagération (culling).
9. **Normales** calculées sur la hauteur originale ou sur un LOD grossier : relief « plat » éclairé. Remède : normal map dérivée de la hauteur exagérée, texturée.
10. **Neige/roche liées à l'altitude du jeu** au lieu de l'altitude réelle : la neige descendrait trop bas. Remède : conserver `Z_réel` dans un canal ou une texture.
11. **Érosion qui détruit les gravures** ou crée des rigoles sub-pixel illisibles à 400 km. Remède : érosion avant gravure, limitée aux zones proches.
12. **Plateformes de villes trop larges** : écrasent les vallées. Remède : rayon plafonné, médiane partielle.
13. **Pentes de jeu** : unités (le pathfinding/déplacement utilise-t-il l'altitude exagérée ?) — **[Inférence]** garder la règle de jeu dans `core/` indépendante de l'exagération visuelle ou documentée dans un ADR.
14. **Licences** : ne pas intégrer de code GPL (Terrain Sculptor, RichDEM) sans décision explicite.
15. **Mémoire / temps** : 44 M de cellules en float32 = 176 Mo par grille ; une pyramide à 5 bandes + masques ≈ 1-2 Go ; érosion Python pur trop lente → numba ou tuiles.
16. **Exagération non calibrée** : aucune étude chiffrée trouvée ; procéder par comparaison visuelle à trois distances (130, 400, 900 km), budget de captures du projet.

## Sources principales
- Terrain Sculptor : https://cartographicperspectives.org/index.php/journal/article/view/cp67-leonowicz-et-al ; https://www.terraincartography.colororacle.org/terrainsculptor/index.html
- Multi-scale pyramids : https://research.monash.edu/en/publications/terrain-generalization-with-multi-scale-pyramids-constrained-by-c-2/
- Patterson : https://shadedrelief.com/bumping/bumping.html ; https://www.shadedrelief.com/texture_shading/
- Imhof/swiss : https://en.wikipedia.org/wiki/Terrain_cartography
- Hydrologie : https://arxiv.org/pdf/1511.04463 ; https://richdem.readthedocs.io/en/latest/depression_breaching.html
- Érosion : https://xarray-spatial.readthedocs.io/en/stable/_modules/xrspatial/erosion.html ; https://github.com/QuentinWach/terra
- Génération par uplift : https://diglib.eg.org/handle/10.1111/cgf12820 ; https://diglib.eg.org/handle/10.1111/cgf15033
- Total War (éditeur) : https://wiki.totalwar.com/w/TWWAKT_Editing_the_Heightmap
