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
| HV1 | France, Pays-Bas, îles Britanniques (factions, titres, personnages, provinces, colonies) | fusionné (96) |
| HV2 | Empire, Europe centrale, Baltique, Scandinavie, Hongrie | fusionné (200) |
| HV3 | Ibérie, Italie, Maghreb, Égypte, Levant, Chypre | fusionné (127) |
| HV4 | Balkans, Grèce, Anatolie, Horde, Caucase, Rus', Mésopotamie | fusionné (151) |
| HV5 | Codex, événements, rencontres, missions, discours, écrans, ordres de chevalerie | fusionné (27 + 15 fiches) |
| HV6 | Unités, techs, bâtiments, traits, compétences, religions, noms, édits, ressources, villes 1:1 v2 | fusionné (169) |
| HV7 | Décisions « à décider » de HV3/4/6 (18 points) | lancé |
| HV8 | Décisions de HV1/2/5 (12 points) | lancé |
| HV9 | Textes d'UI, crédits musicaux, chaînes GDScript, manuel | lancé |
| HV10 | 12-18 événements historiques pour l'Est et le Sud (1337-1360) | lancé |

Prochaine étape : attendre les lots, fusionner dans feat/hv, trancher les points « à décider », cargo test + pytest, ff main.

Reste à faire après HV1/HV2 : décisions HV5 (chr_umur_bey chef d'Aydın dès 1334, mort printemps 1348 ; chr_ivan_kalita note grand-princé partagé depuis 1328 ; chr_otto_le_doux titre Brunswick-Wolfenbüttel depuis 1318) + décisions HV1/HV2.
