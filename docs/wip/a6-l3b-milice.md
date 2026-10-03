# A6-L3b — Milice et armées de départ en données

Branche `a6-l3b` (depuis `a6-merge`). ADR : addendum à `docs/decisions/0179-economie-a-l-echelle.md`.

## État
- Problème 1 (milice) : mécanisme `share_caps` dans `data/ai/doctrines.json` (schéma `ai_doctrine.schema.json`), appliqué par `ai::doctrine::pick_recruit` ; la composition comptée inclut les garnisons (`ai/src/campaign.rs`). Réglage retenu : unit_urban_militia max_share 0,7 à partir de 4 régiments.
- Cause trouvée : les factions sans armée de campagne recrutaient des milices dans leurs garnisons à chaque tour (la composition ne voyait que les armées) ; un plafond sur les seules armées n'avait aucun effet.
- Problème 2 (armées de départ en données) : à faire.

## Prochaine étape
Problème 2 : `data/rules/starting_armies.json` + schéma + chargeur data-model + `setup_1337.rs`, puis finition (fmt, clippy, tests, sonde, ADR, supprimer core/target).
