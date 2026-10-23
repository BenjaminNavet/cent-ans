# Lot EAU : mer, côtes, rivières, neige (campagne)

Source : `out/juge/iter-02-art.md` constats 8, 9, 10. ADR 0750 (`docs/decisions/0750-eau-cotes-neige-de-la-campagne.md`).

## Fait (commits wip sur la branche du worktree)
- Mer (`sea.wesl`, `data/fx/sea_life.json` bloc `eau`, `sea_life.rs::apply_eau`) : couleurs de profondeur, 4,5 m/px, fonds irréguliers, houle supplémentaire, moutons doux à la place des crêtes (les « vermicelles » venaient de `sea_crests`), écume irrégulière. `CENT_ANS_EAU_V2=0` coupe le tout.
- Côte : B-spline 4 lectures (sans nouvel échantillonneur) dans `sea.wesl` et `terrain.wesl::coast_px_smooth`.
- Rivières : bord adouci, reflet, teinte de saison + glace de berge (`water/season.rs`, `river_display.json`). Rivières fines (`river_fine.wesl`) et lacs `fresh` : non faits pour la saison.
- Neige : exposition/altitude, rampe large, givre forestier (terrain) ; givre des feuillus d'hiver (`vegetation/billboard.wesl`, une ligne) ; ombres de nuages plus petites et douces (`data/ui/campaign_map.json`).

## Testé
- `cargo check --features dev` OK ; shaders validés au rendu (2 captures hors écran : côte de Cherbourg, Paris en hiver d=200). Forêts d'hiver givrées, ombres de nuages adoucies, turquoise côtier visible.
- NON fait : clippy complet, tests, smoke, bench fps (`ab.sh` avec `CENT_ANS_EAU_V2=0/1`), test des valeurs `cumulus_*` éventuellement figées dans `look.rs`. À faire une fois au merge.

## Points ouverts
- Fps : mesurer d30/d60 en alternance (coût attendu : 4 lectures près du rivage seulement).
- Bande brune entre le turquoise et la terre à la côte (estran/plage du lot sol ?) à juger ; mer au large encore sombre (LUM).
- Worktree : liens symboliques non commités vers données hors dépôt (`data/map/pyramid`).
