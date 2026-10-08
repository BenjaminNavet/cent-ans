# DN ui-codex : images des fiches Codex

Branche `dn/ui-codex`. Rendu des fiches : `game/scripts/codex/codex_window.gd` (`art_path_of`), pas `encyclopedia.gd` (qui ne gère que les fiches de règles ; non touché).

## Constat
L'audit (« 87 personnages sans image ») est périmé : 61 fiches personnage avaient déjà un portrait via `entity: chr_*`. Restaient 26 personnages sans image.

## Changement
`art_path_of` : repli supplémentaire (1) `icons/entity/<entity>.png` (ressources `res_*`, 128 px, rendu centré comme les portraits), (2) `portraits/chr_<slug>.png` quand `cdx_<slug>` correspond exactement (4 fiches dont l'`entity` est une province ou vide : adolphe_de_la_marck, baudouin_de_luxembourg, jean_le_bel, taddeo_pepoli). Aucune correspondance approximative.

## Comptes (fiches sans image, 476 fiches)
Avant : 161 (personnage 26, mecanique 63, plante 18, economie 10, societe 5, lieu 5, guerre 5, religion 5, unite 4, vie_quotidienne 4, bataille 3, institution 3, dynastie 2, ingredient 2, cuisine 2, medecine 2, evenement 1, batiment 1).
Après : 149 (personnage 22, economie 2, le reste inchangé). Liste : `codex-sans-image.txt` (file de génération).
Pas de correspondance sûre par identifiant pour unités/bâtiments/lieux (les `unit_*`/`bld_*` liés par `entity` étaient déjà branchés).
