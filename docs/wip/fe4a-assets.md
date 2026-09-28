# FE4a — données dérivées après F4a (9 provinces, 7 factions)

## État
- Armes de la maison d'Albret ajoutées à `data/heraldry/houses.json` (de gueules plain, historique, attesté).
- Bug génériciteur corrigé : `tools/cent_ans_tools/heraldry.py` ne dessinait pas le lambel sur un champ d'hermine
  (branche `elif blazon.has("hermine")` de `_draw_charges`), d'où le doublon Bretagne/Penthièvre (36 écus).
- 9 fichiers `data/settlements/prov_*.json` complétés à 3 colonies chacun (historiques, sources en commentaire `sources`) :
  albret, alencon, armagnac, blois, charolais, evreux, foix, penthievre, valois.

## Prochaine étape
- Regénérer front_end.json (7 nouvelles factions jouables : albret, alencon, armagnac, blois, bourbon, foix_bearn, penthievre).
- Régénérer geo horizon / navgrid / settlement_graph / hamlets avec `uv run --project tools cent-ans geo ...`.
- Vérifier portrait_archetypes et cargo test -p data-model.

## Points ouverts
- Aucun pour l'instant.
