# WIP orchestrateur — session 7 (nuit du 24/09) : Cent Ans moderne et de grande qualité

Mandat : autonomie complète toute la nuit. Inspiration principale Total War. Tous les aspects : graphismes, assets, UI, mécaniques, équilibre, animations, modèles 3D, physique, audio, performance.
- Orchestrateur unique : reprend la session 6 (TW, `docs/wip/tw.md`, vague 6 C4/C5/B8) et le mouvement libre (`docs/wip/mouvement-libre.md`, M1-M5). Les autres sessions sont fermées par le joueur.
- Budget : **nouvelle enveloppe de 50 $** propre à cette session (section « Session 7 » de `docs/budget.md`).
- Latitude totale sur le design (refontes, rupture de sauvegarde permises, ADR à chaque fois).
- Autorisé : push auto de main après chaque vague vérifiée, Blender MCP, fenêtres Godot, assets CC0/CC-BY crédités (recherche web d'abord).

## Vagues

| Vague | Lots | État |
|---|---|---|
| 0 | A1 audit visuel/3D/animation ; A2 audit mécaniques/équilibre (simulations IA) ; A3 audit UI/UX ; A4 recherche d'assets libres ; A5 audit technique (rendu, perf, audio) | en cours |

## Reprise
1. Lire les rapports `docs/audit/*.md` (vague 0) et la feuille de route `docs/audit/feuille-de-route.md` (à écrire après la vague 0).
2. Lots hérités : reprendre depuis les fichiers wip dans les worktrees (`git worktree list`) : C4, C5, B8 (session 6) ; M1, M2 (mouvement libre).
3. Fusion : procédure de `docs/wip/tw.md` § Reprise 2 (worktree `../gp-tw-merge`, ff-only dans main, commit avec chemins explicites).
