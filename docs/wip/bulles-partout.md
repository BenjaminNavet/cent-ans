# WIP — Bulles partout (session historien, 25/09)

Conception : `docs/design/2026-09-25-bulles-partout.md`.

| Lot | État | Branche | Notes |
|---|---|---|---|
| Squelette (schéma, catégories, validateur) | fait | main | |
| B1 Infra T universel | fini, fusionné dans integration/historien (1a87dcd5), tests en cours | feat/b1-bulles-infra | |
| B2 Mécaniques campagne | fini (27 fiches cdx_jeu_*, 12 gameplay), fusionné dans integration | worktree-agent-a1391ebbb72a033ca | |
| B3 Mécaniques bataille/siège/naval | fini (32 fiches), fusionné dans integration | worktree-agent-a8fac39c8d71471d7 | supprimer `data/codex/_b3_links.md` après B4/B5 ; B6 doit relire Breteuil/Romorantin 1356 |
| B4 Bâtiments + ressources | lancé (worktree agent) | | |
| B5 Unités, navires, techniques | lancé (worktree agent) | | |
| B6 Audit historique récent | lancé (worktree agent) | | |

Fusion : worktree `../gp-historien-merge` (branche `integration/historien`) puis ff-only dans main.
Commits avec chemins explicites (index partagé avec d'autres sessions).

## Journal
- 2026-09-25 : vague 1 lancée, 6 agents B1-B6 en worktrees. En cas de coupure : relancer le lot depuis son `docs/wip/bN-*.md` dans sa branche worktree.

## Incohérences code ↔ interface relevées par B2 (lot correctif B7 à prévoir)
- Coût de la cour : aide F1 dit 3 % au-delà de 8 saisons ; code 20 % au-delà de 6 (`economy.rs` OPULENCE_*).
- Dette : infobulles disent « se débandent » ; code : −10 moral/saison seulement.
- Ravitaillement : infobulle cite le pays « dévasté » ; code ne regarde qu'ami/non ami.
- Deux mécontentements : panneau lit `ProvinceState.unrest`, les révoltes lisent la moyenne des classes.
- Carte du mécontentement toujours rouge (ratio non divisé par 100).
- Données non lues : piété des édits Paix de Dieu/Carême strict, vitesse de construction (Bâtisseur, Urbaniste), `recruit_time_turns`, coûts en ressources des bâtiments, `vision_army_km`/`vision_settlement_km`.
