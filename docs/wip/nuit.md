# WIP orchestrateur — session 7 (nuit du 24/09) : Cent Ans moderne et de grande qualité

Mandat : autonomie complète toute la nuit. Inspiration principale Total War. Tous les aspects : graphismes, assets, UI, mécaniques, équilibre, animations, modèles 3D, physique, audio, performance.
- Orchestrateur unique : reprend la session 6 (TW, `docs/wip/tw.md`, vague 6 C4/C5/B8) et le mouvement libre (`docs/wip/mouvement-libre.md`, M1-M5). Les autres sessions sont fermées par le joueur.
- Budget : **nouvelle enveloppe de 50 $** propre à cette session (section « Session 7 » de `docs/budget.md`).
- Latitude totale sur le design (refontes, rupture de sauvegarde permises, ADR à chaque fois).
- Autorisé : push auto de main après chaque vague vérifiée, Blender MCP, fenêtres Godot, assets CC0/CC-BY crédités (recherche web d'abord).

## Vagues

| Vague | Lots | État |
|---|---|---|
| 0 | A1 audit visuel/3D/animation ; A2 audit mécaniques/équilibre (simulations IA) ; A3 audit UI/UX ; A4 recherche d'assets libres ; A5 audit technique (rendu, perf, audio) | A1-A5 **faits** (`docs/audit/`) |
| 1 | V1 **fusionné** 3638774 ; D0 (**fusionné** 40a39c9 : 13/15 assets, Knight Pack Quaternius en échec, quota Google Drive) téléchargement du top 15 d'assets libres (`game/assets/third_party/`, wip `d0-assets.md`) ; V1 correctifs visuels rapides A1-01/02/03/04/06 (wip `v1-correctifs-visuels.md`) | en cours (worktrees) |
| 6 | AU1 audio (banque CC0, bataille spatialisée, ambiances, bus ; wip `au1-audio.md`) ; T2 perf (relief adaptatif, banc fiable, test portraits ; wip `t2-perf.md`) ; T1 profil dev optimisé **fusionné** 560652d | en cours |
| 5 | L1 Paris emblématique (ADR 0015, wip `l1-paris.md`) ; BV1 flèches, sang au sol, poussière, taille des unités (ADR 0016, wip `bv1-bataille-vivante.md`) ; CV1 saisons, terroirs, colonies qui grandissent, dévastation (wip `cv1-campagne-vivante.md`) | en cours |
| 4 | V3 lumière/ciels/feu A1-05/14/13 (wip `v3-atmosphere.md`) ; V4 fleuves, ponts, forêts A1-11/10 (wip `v4-fleuves-forets.md`) | en cours |
| 3 | V2 soldats et chevaux animés (A1-18, VAT sur base Quaternius, ADR 0014, wip `v2-soldats-animes.md`) | en cours |
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
- 25/09 : le joueur ajoute des idées (sang, démembrements, chutes, collisions de cavalerie, plus de modèles par unité, plus de flèches) et demande aussi d'améliorer la carte de campagne : backlog complet dans `docs/audit/backlog-tw.md`. Vague « bataille vivante » après V2 (dépend des figurines VAT) ; vague « campagne vivante » après V4.
- 25/09 : le joueur demande un Paris réaliste (Seine, île de la Cité, Notre-Dame). Lot L1 « villes emblématiques » (Paris d'abord) dès qu'une place se libère ; coordonné avec V4 (la Seine qui traverse Paris).
- 25/09 : le joueur autorise exceptionnellement jusqu'à 10 agents en parallèle pour cette session. ADR réservées : 0013 G1, 0014 V2, 0015 L1, 0016 BV1, 0017 V3, 0018 V4. M2 (mouvement libre) fusionné par l'autre session (8d8a1a6) ; M3/M4 y tournent encore.
- 25/09 (autre session, « relief ») : le joueur trouve carte de campagne et champs de bataille trop plats. Une session séparée lance R1 relief et occupation du sol réalistes de la campagne (DEM Copernicus plus fin, forêts historiques vers 1340, lacs/étangs/marais ; données hors ligne + crochet `#include` d'une ligne dans `terrain.gdshader`, coordonné avec V4/CV1 ; ADR 0019, wip `r1-relief-campagne.md`) et R2 relief des champs de bataille (`field.rs` flux dérivé + `battle_terrain.gd` ; ADR 0020, wip `r2-relief-bataille.md`). Fusion par cette session via `../gp-relief-merge`.
- 25/09 (session séparée « bâtiments ») : le joueur trouve les bâtiments de campagne et de bataille pas assez réalistes. Lot **BR1 bâtiments réalistes** (ADR **0021** réservée, wip `docs/wip/br1-batiments.md`, worktree `../gp-br1`, branche `br1-buildings`) : kit Blender de bâtiments détaillés texturés (reprend A1-12 et A1-19 hors monuments), branché dans `battle_village.gd`, `battle_siege.gd` (maisons) et les maquettes `settlements.py`/`models.py` (mêmes noms de GLB, donc sans impact sur CV1). Ne touche pas aux monuments de L1 ni à `terrain.gdshader`.
