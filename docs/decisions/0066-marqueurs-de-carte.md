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
  dé-encombrement écran, seulement par rang).
