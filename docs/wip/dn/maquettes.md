# DN-MAQ : maquettes de villes -> modèles générés

État : fait (ADR 0214). Cause du défaut : chemins de la table sans suffixe `_lod`, glb jamais chargé.

- Table : 78 modèles, 21 sous-familles (`data/art/dn_campaign_models.json`), code `dn_campaign_models.gd`
  (`subfamily_for`, `lookup_keys`, `model_name`, `brighten`), `town_maquette_data.gd`, `town_maquette_layer.gd`.
- Tests : `dn_campaign_models_test.gd` (sous-familles, lod). Captures à 150 : paris, camargue, lues (OK).
- Aucune génération fal (tous les modèles de lieux existaient déjà) : 0 $. Pas de repli local utilisé.

## Pour une session locale
Rien à générer. Restes éventuels : abbayes `isl` hors Maghreb (repli abbey_isl_anatolia), `city` sans
variante `west_alp` (repli `city_west`).

## Points ouverts
- Fermes, moulins, ponts (`farm_*`, `econ_*`) : couches séparées (hameaux 1:1 près des villes), non branchés ici.
- Perf du rendu des glb à toute distance : à mesurer sur machine calme.
- Équilibrage de l'albédo (gain 2,4) selon la lumière du soir/brume.
