# WIP — session historien (histoire et savoir)

Conception : `docs/design/2026-09-23-histoire-et-savoir.md`.

| Lot | État | Où | Notes |
|---|---|---|---|
| H1 Audit historique | **fusionné** (fcbcb01) | — | données seulement ; rapport `docs/histoire/audit-2026-09-23.md` |
| H2 Codex et bulles (infra + amorce) | **fusionné** (9ffd82f) | — | nouveaux fichiers game/ ; hooks minimaux |
| H3+H4 Table et médecine (core + données) | **fusionné** (453f2e6) | — | API : `docs/design/h3-h4-api.md` ; reste UI, libellés d'effets, genres table/medicine dans season_report, herbier |
| H7 Événements historiques manquants | **fusionné** (50ccf58 + correctif 44c48b0) | — | 20 événements + 3 personnages ; doublons avec F7b retirés |
| H9 UI Table + Médecine + herbier + rapport de saison | **fusionné** (7e9d0e6) | — | captures `docs/img/table-section.png`, `tech-medicine.png` ; reste : journal et solde HUD dans map_ui.gd (session ui-tw), icônes des régimes |
| H8 Rédaction codex partie 1 | **fusionné** (41 fiches, af53cdf) | — | |
| H8b Codex partie 2 (~70 fiches) + liens + onglets | en cours | worktree agent | hors cuisine/médecine/monnaie ; corrige les onglets masqués |
| H10 Codex Table, plantes, médecine (~55 fiches) | en cours | worktree agent | |
| H5 Monnaie + H6 Chevalerie et rançons (core + données) | vague 2 | — | |
| UI monnaie, rançons, ordres ; événements éducatifs ; relecture | vague 3 | — | |
| Liaison Encyclopédie F8 (touche L) ↔ Codex (K) via `entity` | vague 3 | — | boutons croisés « Fiche historique » / « Voir dans l'encyclopédie » |

Coordination : sessions parallèles orchestrateur (2b, F1-F9), ui-tw (e3), visual (76).
Ne stager que ses propres chemins. F8 (encyclopédie) doit s'appuyer sur le Codex.

Vague 1 lancée (3 agents worktree : H1, H2, H3+H4). Prochaine étape : fusionner leurs branches, puis vague 2.

Reste après H2 : bouton Codex dans le bandeau (`map_ui.gd`, session ui-tw) ; T = épingler seulement si une infobulle est visible (sinon arbre des techs).

## Journal des coupures
- 2026-09-23 ~23h : limite de quota (reset 0h10) → H7, H8 coupés ; relancés.
- 2026-09-24 : H8 bloqué (watchdog), puis limite (reset 5h10) → H7, H8, H9 coupés ; relancés tous trois avec leur contexte.

Méthode de fusion (depuis l'incident 50ccf58) : worktree `../gp-historien-merge` sur la branche `integration/historien`, fusion et tests là-bas, puis `git merge --ff-only integration/historien` dans main.
