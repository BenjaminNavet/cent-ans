# JR2 — données croisés (état)

Branche `feat/jr-data`, worktree `../gp-jr-data`. Pas de Rust, pas de build cargo.

## Fait
- Étape 1 : faction, chef, maison La Palud, Limassol (propriétaire, `bld_muster_field`), flotte (mer `sea_levantine`), revendications, relations réciproques, archétypes de portrait.
- Étape 2 : arêtes `sea` Limassol ↔ Acre, Gaza, Tripoli (éditées à la main dans `settlement_graph.json`, coût = 100 + km), carte de sélection, armes (meuble « croix potencée » ajouté à `heraldry.py`), bannières, codex.

## Points ouverts
- Les arêtes maritimes sont à la main : `cent-ans geo roads` / `geo settlements` les effacerait (le générateur ne relie que les ports les plus proches de provinces voisines). Limassol ↔ Famagouste existe déjà en arête terrestre : pas de doublon maritime.
- Le test de relief/ledger/icônes échouent déjà sur main (fichiers `docs/img` ignorés, version de bake, ledger).
- Les `.png.import` des nouveaux visuels seront créés par Godot à l'import.
- Pas d'illustration de carte de sélection (champ optionnel).
