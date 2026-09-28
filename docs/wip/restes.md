# RS — Restes des wip (orchestration, 28/09) — fichier de reprise

Demande du joueur (28/09) : « tu es un orchestrateur et tu vas attaquer les wip restants ».
Tri fait sur `main` 2c475b06 (agent Explore) : 20 points ouverts, faisables sans le joueur.
Exclus : parties pilotes, test sur vrai PC Windows, écoutes, publication du relief, dépenses cloud,
FXAA, retrait de `--coarse-figures` (point 17, choix du joueur).

## Reprendre

Lire ce fichier, `git worktree list`, `git branch --list 'feat/rs-*'`. Chaque lot a son
`docs/wip/rs-<lot>.md`. Fusion : worktree `../gp-rs-merge` (branche `integration/rs`), puis ff-only
vers `main`. ADR réservés : **0098** (lot B), **0099** (lot F).

## Vague 1 (6 agents, fichiers disjoints)

| Lot | Points | Agent | Branche | État |
|---|---|---|---|---|
| A Codex | H10 lots 7-8 (8 fiches), fiche Difficulté | mech | `feat/rs-a-codex` | lancé |
| B Ordre public | constantes économiques → `economy.json` ; pondération `province_effect_percent` (garnison, peste, revenu IA) ; révoltes vers 4-10/partie | dev | `feat/rs-b-order` | lancé |
| D Commerce + tests | chemins commerciaux précalculés ; tests campagne 4/13/14/15 (revue-code) | dev | `feat/rs-d-trade` | lancé |
| E Carte | lectures groupées diplomacy_panel/alerts ; légende frontières ; refresh NextHint sur événement | mech | `feat/rs-e-map` | lancé |
| F Bataille | bouton « incendier » (S2) + règles de feu lues depuis `data/` ; sélecteur de formations repliable | dev | `feat/rs-f-battle` | lancé |
| H Nettoyage | événement `movement` du mock ; bug du grand livre `budget.py` | mech | `feat/rs-h-cleanup` | lancé |

## Vague 2 (après fusion de la vague 1)

- C Diplomatie/IA : plafond du bonus Mariage, démolition par l'IA (banqueroutes) — après B (sondes communes).
- G Villes/fleuves : VH8 (Bordeaux, Avignon, Calais, Bruges), rives et couloirs de fleuve.
- I PO phase 2 (P2a-f) : après E et F (fichiers d'UI communs).
- Perf scripts restants (TownLayer, étendards, audio) ; suites EP (seules, marges minces).
