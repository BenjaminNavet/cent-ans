# Orchestration : mouvement libre des armées

Spec : `docs/design/2026-09-24-mouvement-libre.md`. ADR : `docs/decisions/0010-free-army-movement.md`. Conception validée par le joueur le 2026-09-24.

| Lot | État | Notes |
|---|---|---|
| M1 grille de navigation | **fusionné** | 79 ponts, 17 gués/bacs, 12 cols ; 97,9 % de la terre franchissable ; notes `docs/wip/m1-navgrid.md` |
| M2 cœur | **fusionné** (8d8a1a6) | API et limites : `docs/wip/m2-core-movement.md` ; `c5_settlements_ui_test.gd` échoue (2 points, exécution immédiate) → à adapter en M4 |
| M3 tour séquentiel + IA | **fusionné** (e8056c2d) | batailles de campagne et débarquements anglais rétablis ; IA 2,4 ms/faction en moyenne (p99 15 ms) ; notes `docs/wip/m3-tour-ia.md` |
| M4 pont + UI | **fusionné** (99606cf) | bulle, chemin 2 couleurs, clic droit, animation, cercle ZdC ; captures `docs/img/m4/` ; notes `docs/wip/m4-pont-ui.md` |
| M5 vision, équilibrage, docs | M5b **fusionné** (99db2366) ; M5a (vision) en cours | M5b : débandades rétablies (1,8/graine), croisements route/fleuve 362→77, Saône et Somme, manuel/codex/tutoriel ; notes `docs/wip/m5b-equilibrage-docs.md` |

## Décisions prises en cours de route

- 2026-09-24 : le joueur valide la spec telle quelle (« fais le »).
- 2026-09-24 : la session 6 (TW) est prévenue que son lot C2 « zone de contrôle » (vague 6) recoupe M2 ; proposition de le suspendre jusqu'à la fusion de M2. C2 est suspendu.
- 2026-09-24 : l'ADR est renumérotée en 0010 (0006 à 0009 déjà pris).
- M1 : Saône non tracée en aval de Chalon (ponts sans effet) ; 397 croisements voies romaines/fleuves ouvrent beaucoup de passages, à surveiller en M5.
- M2 : `movement_left` en coûts de grille (≈ 1 460 au printemps) ; siège ouvert en cours de tour progresse à la fin du tour suivant ; repli neutre → `Retreat::Fallback` ; agents C6 inchangés (graphe, sans zone de contrôle).
- M4 : l'aperçu du chemin ignore les zones de contrôle (la marche réelle peut s'arrêter avant) ; armées IA non animées pendant le tour IA ; M4 a touché `march.rs`/`orders.rs` (`StopReason::Engaged`), M3 doit l'intégrer.
- M3 : débandades à 0 depuis M2 (le recul de 15 km réussit presque toujours) ; Écosse fragile (banqueroutes) ; l'évitement de ZdC porte sur les colonies du graphe, pas sur le segment de grille ; `resolve_trade` (C5) doit aller dans `resolve_end_of_turn` et `ai_choose_edicts` (C4) dans `plan_turn`.
- M5b : batailles 89/graine (59 avant M2) — goulets des ponts + tour séquentiel ; ponts ni coupables ni gardables ; Écosse = problème d'économie, hors périmètre.
- À la fusion de M2 (fait), vérifier : `agents.rs` (C6, Dijkstra sur le graphe des colonies, s'appuie sur `movement.rs` ; les agents ne sont pas arrêtés par la zone de contrôle) et la ligne C7 dans `apply_outcome`.

## Prochaine étape

M1 et M2 tournent en parallèle dans des worktrees. Ensuite : fusion de M1, puis de M2 (dans un worktree séparé, ff-only dans main). Puis lancement de M3 et M4 en parallèle, et enfin de M5.
