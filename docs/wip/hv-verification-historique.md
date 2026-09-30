# WIP — HV : vérification historique (nuit du 30 septembre 2026)

Mandat : vérifier l'exactitude historique du contenu du jeu, corriger, compléter. Autonomie toute la nuit.
Périmètre : tout ce qui a été ajouté ou réécrit depuis l'audit B6 du 25/09 (`docs/histoire/audit-2026-09-25.md`) :
OM (Oural + Méditerranée, 311 provinces et colonies), FE (225 titres, 148 factions), 158 personnages,
39 fiches du Codex, événements, rencontres, missions, unités de l'Est, religions, noms.

Méthode : celle des audits du 23 et du 25/09 (Erreur corrigée / Approximation assumée / Liberté de jeu).
Textes, noms, dates, maisons, blasons attestés : corrigés directement. Champs mécaniques
(owner, suzerain, relations, stats, listes de provinces, ids) : listés « à décider », tranchés par
l'orchestrateur à l'intégration.

Branche d'intégration : `feat/hv` (worktree `../gp-hv`). Rapports : `docs/histoire/audit-2026-09-30-<lot>.md`.

| Lot | Périmètre | État |
|---|---|---|
| HV1 | France, Pays-Bas, îles Britanniques (factions, titres, personnages, provinces, colonies) | lancé |
| HV2 | Empire, Europe centrale, Baltique, Scandinavie, Hongrie | lancé |
| HV3 | Ibérie, Italie, Maghreb, Égypte, Levant, Chypre | lancé |
| HV4 | Balkans, Grèce, Anatolie, Horde, Caucase, Rus', Mésopotamie | lancé |
| HV5 | Codex, événements, rencontres, missions, discours, écrans, ordres de chevalerie | lancé |
| HV6 | Unités, techs, bâtiments, traits, compétences, religions, noms, édits, ressources, villes 1:1 v2 | lancé |

Prochaine étape : attendre les lots, fusionner dans feat/hv, trancher les points « à décider », cargo test + pytest, ff main.
