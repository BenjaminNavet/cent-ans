# WIP — B2 Mécaniques de campagne (fiches Codex `cdx_jeu_*`)

Spec : `docs/design/2026-09-25-bulles-partout.md` (lot B2). Branche : `worktree-agent-a1391ebbb72a033ca`.

## État
- [x] Collecte des chiffres (code `core/crates/sim-campaign`, `data/`)
- [x] Fiches `cdx_jeu_*` (27)
- [x] `gameplay` ajouté aux fiches existantes (zone de contrôle, chevauchée, rançon, mutations, taille, gabelle, aides, Carême, jours maigres, Peste noire, places fortes, ponts et gués) + see_also vers les fiches jeu
- [x] Liens `[[cdx_jeu_*]]` dans les données (6 édits, 4 événements : disette, révolte fiscale, épidémie locale, Peste noire)
- [x] Validateur Codex vert (262 fiches, 1975 liens), pytest 418 verts ; `_b2_links.md` supprimé

## Fiches écrites
cdx_jeu_tresor, cdx_jeu_impot, cdx_jeu_entretien, cdx_jeu_edits, cdx_jeu_commerce, cdx_jeu_ordre_public, cdx_jeu_population, cdx_jeu_devastation, cdx_jeu_famine, cdx_jeu_table, cdx_jeu_saisons, cdx_jeu_meteo, cdx_jeu_mouvement, cdx_jeu_ravitaillement, cdx_jeu_vision, cdx_jeu_terrain, cdx_jeu_traversee, cdx_jeu_colonies, cdx_jeu_construction, cdx_jeu_recrutement, cdx_jeu_suite, cdx_jeu_personnages, cdx_jeu_agents, cdx_jeu_diplomatie, cdx_jeu_chronique, cdx_jeu_technologies, cdx_jeu_victoire ; gameplay ajouté à cdx_ponts_gues (plus de fiche cdx_jeu_fleuves)

## Prochaine étape
Lot terminé ; reste la fusion (orchestrateur). Points ouverts :
- Écarts code/UI relevés : aide F1 (`help_controller.gd`) annonce « 3 % de l'excédent au-delà de huit saisons » alors que le code prend 20 % au-delà de six ; `rich_tooltip.gd` et `encyclopedia.gd` disent que les troupes « se débandent » en dette (le code : −10 de moral) et que le ravitaillement baisse en pays « dévasté » (le code : seulement « non ami ») ; la ligne « Mécontentement » du panneau de province lit `ProvinceState.unrest`, champ qu'aucune règle ne lit (prises, chevauchées, régence, bonus de province complète n'agissent que sur lui) ; la piété des édits n'est lue nulle part ; `ConstructionSpeed` et `recruit_time_turns` ne sont pas lus ; la carte du mécontentement sature au rouge (ratio non divisé par 100).
- Alias susceptibles de doublonner avec B3/B4/B5 : garnison, commandement, forêt, marais, collines, montagnes, col, construction, chantier, écuyer, expérience, météo, traversée, débarquement.

## Validateur
`uv run --project tools python -c "from pathlib import Path; from cent_ans_tools.codex import validate_codex; r=validate_codex(Path('data')); print(len(r.entries)); print('\n'.join(r.errors))"`
ou `uv run --project tools pytest tools/tests/test_codex.py`.
