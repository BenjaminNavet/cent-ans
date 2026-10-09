# TX campagne (lot T2b3 + T2b4) — branche tx-campaign

Spec : docs/superpowers/specs/2026-10-09-textures-regionales-design.md § 2b. ADR 0239.

## État (2026-10-09)
- Fond par biome x rôle (bloc `regional` de `campaign_terrain_textures.json`), fondu de frontière
  10 km (cartes `geo biome-blend`), micro-grain (`micro`), drapeau `parcels_source` hb|tx (défaut hb)
  + `tx_overrides`, `--legacy-textures`, réglage « Qualité des textures », test `tx_campaign_test.gd`.
- Captures : `tx_campaign_shot.gd` -> ~/dev/cent-ans-raw/textures/planches/campaign_*.png ;
  planche champs : `tx_fields_compare_shot.gd` -> fields_compare.png.
- Règle d2d289623 (.import des tableaux) cherry-picked.

## Points ouverts
- Le fond régional change peu l'image : la couleur vient de la carte de couleur (teintes par rôle) ;
  `hue_keep` (0,6) ajoute la teinte propre des couches. L'augmenter pour plus de contraste régional.
- Variante 2k jamais chargée en jeu (`pack --size 2048` non lancé : disque).
- Planche champs : la caméra ne descend pas sous ~25 unités (profil de caméra proche), vue lointaine.
- Bascule du défaut `parcels_source` à `tx` après jugement de la planche.
