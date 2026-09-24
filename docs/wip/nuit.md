# WIP orchestrateur — session 7 (nuit du 24/09) : Cent Ans moderne et de grande qualité

Mandat : autonomie complète toute la nuit. Inspiration principale Total War. Tous les aspects : graphismes, assets, UI, mécaniques, équilibre, animations, modèles 3D, physique, audio, performance.
- Orchestrateur unique : reprend la session 6 (TW, `docs/wip/tw.md`, vague 6 C4/C5/B8) et le mouvement libre (`docs/wip/mouvement-libre.md`, M1-M5). Les autres sessions sont fermées par le joueur.
- Budget : **nouvelle enveloppe de 50 $** propre à cette session (section « Session 7 » de `docs/budget.md`).
- Latitude totale sur le design (refontes, rupture de sauvegarde permises, ADR à chaque fois).
- Autorisé : push auto de main après chaque vague vérifiée, Blender MCP, fenêtres Godot, assets CC0/CC-BY crédités (recherche web d'abord).

## Vagues

| Vague | Lots | État |
|---|---|---|
| 0 | A1 audit visuel/3D/animation ; A2 audit mécaniques/équilibre (simulations IA) ; A3 audit UI/UX ; A4 recherche d'assets libres ; A5 audit technique (rendu, perf, audio) | A1, A2, A3, A4 **faits** (`docs/audit/`) ; A5 en cours |
| 1 | D0 téléchargement du top 15 d'assets libres (`game/assets/third_party/`, wip `d0-assets.md`) ; V1 correctifs visuels rapides A1-01/02/03/04/06 (wip `v1-correctifs-visuels.md`) | en cours (worktrees) |
| 2 | G1 sonde O1 + auto-résolution N1 + doctrines IA E1 (wip `g1-equilibre.md`) ; U1 bogues UI U0/U2 (wip `u1-bogues-ui.md`) | en cours (worktrees) |

## Reprise
1. Lire les rapports `docs/audit/*.md` (vague 0) et la feuille de route `docs/audit/feuille-de-route.md` (à écrire après la vague 0).
2. Lots hérités : reprendre depuis les fichiers wip dans les worktrees (`git worktree list`) : C4, C5, B8 (session 6) ; M1, M2 (mouvement libre).
3. Fusion : procédure de `docs/wip/tw.md` § Reprise 2 (worktree `../gp-tw-merge`, ff-only dans main, commit avec chemins explicites).

## Décisions
- Les agents des autres sessions (C4, C5, B8, M1, M2) tournaient encore au démarrage : on ne touche ni à movement.rs ni à economy.rs tant qu'ils ne sont pas fusionnés.
- Chemin critique visuel : A1-18 (soldats squelettiques + animations cuites en texture, VAT) après D0 (soldats et chevaux riggés Quaternius CC0).
- A2 : milice = 99,8 % des recrutements IA ; auto-résolution contredit la 3D ; carte figée puis boule de neige ; aucune tension (ordre public). Priorités O1, N1, E1 puis E2, E4, E3, N2, N3, E8.
- A3 : après G1/U1, vague UI U1 fenêtres + U3 économie + U4 échelle ensemble (tous dans map_ui.gd).
- 24/09 après le crash : l'orchestrateur TW (session 6, relancé) finit et fusionne C4, C5 et B8b via ../gp-tw-merge ; la session du mouvement libre garde M2-M5. La nuit garde D0, V1, G1, U1 et A5, et fusionne via ../gp-night-merge (integration/night). On se prévient mutuellement à chaque push de main.
