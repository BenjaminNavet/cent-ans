# ADR 0066 — Marqueurs de carte : un seul langage (DA3)

Date : 2026-09-26. Lot DA3 de la direction artistique (bible `docs/design/2026-09-25-bible-da.md`
§ 7-8, écart n° 3 du § 10).

## Contexte

Sur la carte de campagne, les lieux parlaient plusieurs langues (captures
`docs/img/da3/*_avant.jpg`) :

- `SettlementLayer` dessinait au palier moyen des formes abstraites (SDF) remplies de la **couleur
  de faction** : disques crénelés roses en Angleterre (« rosaces »), jaunes en Flandre
  (« soleils »), bleus en France, écus crénelés pour les cités, carrés, disques à croix ;
- au palier Europe (parchemin, CM2), `ParchmentOverlay` redessinait une vignette à l'encre par
  cité, drapeau coloré pour les capitales ; les autres lieux disparaissaient ;
- la légende (UX1) recopiait les formes SDF.

La bible impose : forme = type, écu = propriétaire, taille = rang, aucune couleur de marqueur hors
teintes héraldiques.

## Décision

1. **Pictogrammes peints générés**, un par type et rang : capitale, grande cité, cité, ville close,
   ville, bourg, forteresse, château, tour, grande abbaye, abbaye, plus un insigne de port (nef).
   12 images `openai/gpt-5-image-mini` (registre enluminé de la planche validée, prompt construit
   depuis `data/map/settlement_markers.json` + bloc `STYLE` de `map_markers.py`), puis un
   post-traitement **procédural** commun (`build_atlas`) : détourage (inondation depuis le bord
   arrêtée par le trait d'encre, le lavis de fond n'est jamais uni), cadrage sur une ligne de base
   commune, **liseré d'encre et halo vélin de même épaisseur** pour tous. Coût : 0,59 $.
   - Pourquoi pas tout procédural (Pillow, comme `ui_illumination.py`) : à 30-50 px, un
     pictogramme vectoriel se lit bien mais reste un « pictogramme plat », que la bible § 8 écarte
     parmi les miniatures ; la sonde de 3 images a montré des silhouettes lisibles à 32 px, dans la
     main de la planche (bouton cloche). Le procédural garde ce qu'il fait mieux : l'unité du trait
     et du halo, le cadrage, l'atlas (gratuit, rejouable : `--atlas-only`).
2. **Un atlas + un shader d'instance**, pas de nœud par marqueur : le `MultiMesh` existant garde un
   quad par lieu ; `settlement_icon.gdshader` échantillonne l'atlas des pictogrammes et compose à
   l'exécution l'**écu du détenteur** (atlas d'écus bâti une fois depuis
   `PortraitLoader.heraldry_texture`, recomposé seulement si une faction nouvelle apparaît) et
   l'insigne de port. Données par instance : case, surbrillance, taille écran, distance de retrait
   (`INSTANCE_CUSTOM`) ; écu et port (`COLOR`). L'écu montre le **contrôleur** (qui tient la
   place) : c'est ce que le joueur doit lire en guerre ; le propriétaire de droit reste au panneau.
3. **Taille = rang** : rang 1-4 par règles de données (liste des capitales et grandes cités,
   fortification, poids), tailles écran par rang dans le catalogue.
4. **Dé-encombrement par les données, sans CPU par image** : chaque lieu reçoit la distance caméra
   jusqu'à laquelle il reste affiché (paliers de `visibility` : tout sous 380, rangs 2+ sous 700,
   seules capitales, grandes cités et forteresses au-delà — palier Europe) ; le shader fond puis
   replie le quad. Au palier près, les marqueurs cèdent la place aux maquettes 3D (comme avant).
   Le parchemin ne dessine plus ses vignettes de villes (`ParchmentOverlay.draw_towns`).
5. La légende dessine les mêmes pictogrammes et écus (`LegendSample.draw_marker`).

Correction au passage : le décalage écran des quads utilisait `PROJECTION_MATRIX[1][1]` signé ; la
projection est retournée en Y dans ce rendu, les quads étaient donc tournés de 180° (invisible
sur les anciennes formes symétriques). `abs()` rétablit le sens.

## Conséquences

- Plus aucune couleur de faction sur les marqueurs de lieux : la couleur vient des armoiries.
  Les bannières des maquettes 3D (palier près) restent teintes à la couleur de faction.
- Perf : même nombre de nœuds et d'appels de dessin ; mesures i/s dans
  `docs/wip/da3-marqueurs-carte.md`.
- Ajouter un type ou un rang : une entrée de `pictograms` + `kinds`, puis
  `cent-ans assets map-markers` (génère ce qui manque, reconstruit l'atlas).
- Limites : un écu de maison (DA1) pourra remplacer celui de faction via le même atlas ; les
  marqueurs proches se chevauchent encore dans les régions denses au palier moyen (pas de
  dé-encombrement écran, seulement par rang) — traité par DA7d, ci-dessous.

## DA7d — Dé-encombrement écran par priorité (2026-09-26)

### Constat
Le dé-encombrement par rang et distance laissait les marqueurs se chevaucher dans les régions
denses (Flandre, Île-de-France, Normandie, horizon anglais en vue inclinée), et les noms passaient
sous les pictogrammes voisins. Mesure (`game/tests/da7d_overlap_test.gd -- --measure`, 1600×900,
3 régions × 4 distances 1100 / 600 / 330 / 200) : **664 paires** de rectangles écran visibles
qui se recouvrent (marqueurs et noms, hors paire marqueur / son propre nom), jusqu'à 138 paires
dans une seule vue (Île-de-France à 330). Captures `docs/img/da7d/*_avant.jpg`.

### Décision
1. **Placement glouton par priorité, à la Total War** (`MarkerDeclutter`, grille spatiale à cases
   de la taille du plus grand marqueur) : les colonies sont parcourues dans un ordre fixe (rang
   décroissant, poids, puis ordre des données) ; pour chacune, le marqueur puis le nom. Un
   marqueur qui recouvre un rectangle déjà posé **cède la place** (fondu du shader) et son nom
   avec lui ; un nom qui recouvre quelque chose est masqué, le marqueur reste.
2. **Épinglés** : colonie sélectionnée (affichée même hors de son palier de rang), colonie survolée
   (le marqueur sous la souris ne disparaît pas pendant un zoom à la molette) et capitale du
   joueur (colonie la plus prioritaire de la province capitale) sont posées d'abord et toujours.
3. **Pas de CPU par image** : recalcul seulement quand la caméra bouge nettement (déplacement
   > 1 % de la distance, distance ± 1,5 %, rotation > 0,5°, écran redimensionné), quand le palier
   ou les épinglés changent, au plus toutes les 0,15 s. Le shader reçoit l'état par instance
   (`COLOR.b` affiché / cédé, `COLOR.a` instant de la bascule) et fait le fondu seul ; l'horloge
   n'est transmise que pendant un fondu. Les marqueurs hors de l'écran élargi (20 %) ne sont pas
   départagés. Coût d'un recalcul complet : **~1,2-1,5 ms** (GDScript, build debug, meilleur de 5
   séries), contre ~3,3 ms pour l'ancien dé-encombrement des seuls noms, qui tournait lui toutes
   les 0,15 s même caméra immobile.
4. **Règle de rendu, côté Godot** : la logique dépend de la caméra et des tailles écran, sans
   effet sur la simulation ; elle reste dans `game/` mais isolée dans une classe pure et testée
   (`MarkerDeclutter`), et non dans `core/`.
5. **Paramètres dans les données** : bloc `declutter` de `data/map/settlement_markers.json`
   (emprise opaque du pictogramme 0,8 × 0,94 du quad, marges, opacité d'un marqueur cédé —
   0 = masqué, > 0 = estompé —, durée du fondu, seuils de recalcul, épinglés), validé par le schéma.
6. Les noms montent un peu (`LABEL_LIFT` 0,55 × la police au-dessus du quad) pour ne plus toucher
   les flèches de leur propre pictogramme.

### Résultat
Même mesure, plus stricte (paires marqueur / son propre nom comprises) : **0 paire** aux 12 vues ;
le test échoue au premier chevauchement et si un recalcul dépasse 4 ms. Captures
`docs/img/da7d/*_apres.jpg`.

### Limites
- Les noms de provinces (`CityMarkers`, palier loin) ne participent pas au placement.
- Le haut de l'écran sous la barre supérieure n'est pas soustrait (des marqueurs peuvent y passer
  sous l'interface).
- Pas de regroupement (« +3 ») ni de décalage : un marqueur cède ou reste ; le zoom rapproché
  révèle les lieux masqués.
