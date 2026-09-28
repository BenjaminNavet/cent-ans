# FE4a — données dérivées après F4a (9 provinces, 7 factions)

## État : TERMINÉ
`uv run --project tools pytest -q` : 840 passed, 2 skipped (skips pré-existants, sans rapport).
`cd core && cargo test -p data-model` : tout au vert.

## Ce qui a été fait
- Armes de la maison d'Albret ajoutées à `data/heraldry/houses.json` (de gueules plain, historique, attesté,
  source : fr.wikipedia Maison d'Albret).
- Bug du générateur corrigé dans `tools/cent_ans_tools/heraldry.py` : la branche `elif blazon.has("hermine")`
  de `_draw_charges` ne dessinait jamais le lambel, d'où le doublon d'écus Bretagne/Penthièvre (35/36 distincts).
  Le lambel de gueules de Penthièvre est maintenant rendu.
- 9 fichiers `data/settlements/prov_*.json` complétés à 3 colonies chacun (ville/château/abbaye/village
  historiques réels, avec sources) : albret, alencon, armagnac, blois, charolais, evreux, foix, penthievre, valois.
  Deux premiers choix (Beaugency, Domfront) entraient en collision d'id avec des colonies déjà présentes dans
  d'autres provinces (prov_orleanais, prov_normandie_ouest) : remplacés par Chaumont-sur-Loire et Bellême.
- `data/ui/front_end.json` : 7 cartes de présentation (tagline/intro/forces/faiblesses/difficulté) pour
  fac_albret, fac_alencon, fac_armagnac, fac_blois, fac_bourbon, fac_foix_bearn, fac_penthievre. Pas
  d'illustration (champ optionnel du schéma) : aucune génération d'image payante utilisée.
- `data/portraits/archetypes.json` : les 7 nouvelles factions ajoutées au bucket culturel "france".
- Régénéré via les outils existants (aucune édition à la main de fichier généré) :
  `cent-ans geo horizon --province ...` (9 tuiles), `cent-ans geo settlements`, `cent-ans geo hamlets`,
  `cent-ans geo navgrid` → `data/map/{hamlets,settlement_graph,settlements_px,settlement_edge_paths}.json`,
  `data/map/navgrid.png`, `docs/img/{navgrid,settlements}-preview.png`,
  `game/assets/horizon/relief/{index.json,prov_<new>.bin}`.

## Points ouverts
- Les difficultés de jeu attribuées aux 7 nouvelles factions (2 ou 3 sur l'échelle Facile..Très difficile) sont
  un jugement éditorial de ma part (taille du territoire/armée), pas une valeur testée ni documentée ailleurs :
  à revoir si un autre choix de conception est souhaité.
