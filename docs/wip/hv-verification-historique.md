# WIP — HV : vérification historique (nuit du 30 septembre 2026)

**État : terminé, fusionné dans main** (5eeb5f7b4 puis 39de6fd1e, d9e206429). Synthèse :
`docs/histoire/audit-2026-09-30.md` ; rapports par lot `docs/histoire/audit-2026-09-30-hv1.md` à `-hv10.md`.

| Lot | Périmètre | Bilan |
|---|---|---|
| HV1 | France, Pays-Bas, îles Britanniques | 96 corrections |
| HV2 | Empire, Europe centrale, Baltique, Scandinavie, Hongrie | 200 |
| HV3 | Ibérie, Italie, Maghreb, Égypte, Levant, Chypre | 127 |
| HV4 | Balkans, Grèce, Anatolie, Horde, Caucase, Rus' | 151 |
| HV5 | Codex, événements, rencontres, missions | 27 + 15 fiches |
| HV6 | Unités, techs, bâtiments, religions, prénoms | 169 |
| HV7 / HV8 | Décisions sur les champs mécaniques | 24 décisions |
| HV9 | UI, tutoriel, illustrations, crédits musicaux | 13 + 6 morceaux datés |
| HV10 | 18 événements de l'Est et du Sud (1337-1354) | ajout |

Intégration : générateur d'écus (croix pisane, fleurdelisée, griffon, échiqueté), écus/bannières
régénérés, `setup_1337` (morts plus tard en 1337 = vivants au départ), Montpellier à Majorque
(objectif Achaïe), Abu Bakr mamelouk, towns_1340 régénéré dans le checkout principal.
Validation : clippy, cargo test (workspace), pytest (hors 2 tests d'icônes sans images dans les
worktrees), smoke Godot.

Restes : section « Restes » de la synthèse.
