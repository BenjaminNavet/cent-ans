# WIP — session historien (histoire et savoir)

Conception : `docs/design/2026-09-23-histoire-et-savoir.md`.

| Lot | État | Où | Notes |
|---|---|---|---|
| H1 Audit historique | **fusionné** (fcbcb01) | — | données seulement ; rapport `docs/histoire/audit-2026-09-23.md` |
| H2 Codex et bulles (infra + amorce) | **fusionné** (9ffd82f) | — | nouveaux fichiers game/ ; hooks minimaux |
| H3+H4 Table et médecine (core + données) | **fusionné** (453f2e6) | — | API : `docs/design/h3-h4-api.md` ; reste UI, libellés d'effets, genres table/medicine dans season_report, herbier |
| H7 Événements historiques manquants (≈ 22, d’après l’audit) | relancé après quota (finalisation) | worktree agent | data/events, data/characters |
| H9 UI Table + Médecine + herbier + rapport de saison | en cours | worktree agent | hook minimal dans province_panel.gd |
| H8 Rédaction codex (~120 fiches) + liens | relancé après quota | worktree agent | hors cuisine/médecine/monnaie ; corrige les onglets masqués |
| H5 Monnaie + H6 Chevalerie et rançons (core + données) | vague 2 | — | |
| UI monnaie, rançons, ordres ; événements éducatifs ; relecture | vague 3 | — | |

Coordination : sessions parallèles orchestrateur (2b, F1-F9), ui-tw (e3), visual (76).
Ne stager que ses propres chemins. F8 (encyclopédie) doit s'appuyer sur le Codex.

Vague 1 lancée (3 agents worktree : H1, H2, H3+H4). Prochaine étape : fusionner leurs branches, puis vague 2.

Reste après H2 : bouton Codex dans le bandeau (`map_ui.gd`, session ui-tw) ; T = épingler seulement si une infobulle est visible (sinon arbre des techs).
