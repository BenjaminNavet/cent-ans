# 0829 — DATA-GARDE : manifeste de schémas, chargement parallèle, textes du core

Statut : accepté (lot L-F du juge technique, itération 3).

## Contexte
Le juge technique constatait l'absence de contrôle « donnée ↔ schéma » exhaustif. À l'état du dépôt, `tools/tests/test_schemas.py` couvrait déjà 2 430 des 2 494 fichiers JSON/YAML de `data/` par une table Python ; 64 fichiers (`art/dn_catalog_*`, `art/tx_*_pack`, `fx`, `map/*_px`…) n'avaient aucun schéma, et rien n'obligeait un nouveau fichier à en avoir un.

## Décision
- `data/schemas/manifest.json` est la table explicite `glob de données → schéma`, plus une liste `exceptions` (glob + raison). La table Python est supprimée.
- `test_schemas.py` échoue si un fichier de `data/` (hors `schemas/`) n'est ni couvert ni excepté, si une entrée ne correspond à aucun fichier, si une exception n'a pas de raison, ou si un schéma n'est utilisé nulle part (hors `EXEMPT_SCHEMAS` justifiés).
- Les 27 schémas ajoutés sont dérivés des données existantes : types, clés requises, clés inconnues refusées. Ils sont volontairement minimaux ; les bornes et énumérations s'ajoutent au fil des retouches.
- `GameData::load` charge ses familles de fichiers en parallèle (`std::thread::scope`).
- Les chaînes françaises de `core/` migrent vers les textes de `data/ui/` par grappes.

## Conséquences
Un fichier de données sans schéma casse le test Python. Le périmètre client (panier, réglages, touches) reste hors de ce lot.
