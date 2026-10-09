# Animateur — état des lieux (09/10, lecture seule, rien vu en jeu)

## 1. État actuel
Ni `AnimationPlayer` ni `Skeleton3D` : poses cuites en texture d'os, shader sur MultiMesh. Rig humain 70 clips (`tools/blender_scripts/battle_skinned.py` l. 317-394), cheval ~23 clips + allures AS8b ; choix dans `battle_skinned.gd` (`STYLES`, l. 959).

| Unité | Attente | Marche | Charge | Combat | Tir | Victoire | Déroute | Mort |
|---|---|---|---|---|---|---|---|---|
| Épée | 6 var. | walk/run | run ×1,05 | 8 clips en cycle | — | 2 | flee ×2 | 4 + renversé + 3 blessés |
| Pique | pike_idle | pike_walk | pike_level_walk | estoc, `brace` | — | victory_pike | flee | idem |
| Arc | 4 | **walk générique** | run | mêlée | bow_shoot | 2 | flee | idem |
| Arbalète | 3 | **walk générique** | run | mêlée | xbow_shoot | 1 | flee | idem |
| Lance montée | c_idle | c_walk/trot/gallop + 4 virages | charge + trébuchement | c_thrust, c_rear | — | c_victory | galop | 3 |
| Archer monté | oui | oui | galop | oui | c_bow_shoot / javelot | oui | galop | idem |

Plus : rôles (étendard, tambour, cor), servants d'engins (`siege_engines.json`), civils (idle/walk/carry, faux/charrue), armées de campagne (idle/marching calé sur la vitesse), bêtes en shader (AS8c), imposteurs 4 jeux × 8 angles × 4 images.

## 2. Forces
- Foule désynchronisée sans CPU (graine, phase, vitesse ±7 %, tirage de mêlée par cycle).
- Fondus 0,35 s / 0,2 s (ADR 0129), surcoût ≤ 3 %.
- Cadence calée sur la vitesse ; trot/galop Muybridge (34→5 %, 30→12 %).
- Mouvement secondaire AN1a ~1 % ; drapeaux A/B et tests par lot.

## 3. Faiblesses
1. **AS7 (jugement en jeu) jamais fait** ; validation sur planches fixes seulement.
2. `c_walk` patine à 49 % (allure la plus vue) ; `c_fall` keyframé.
3. **`bow_walk`/`xbow_walk` cuits et réglés mais non branchés** dans `STYLES`.
4. Mêlée hétérogène (NT14, FA3, keyframé), lame d'estoc décalée, pas de clips appariés.
5. Pas de pas de côté, recul, rotation sur place : les pivots glissent.
6. Charge à pied = `run`.
7. Secondaire par régiment, continue en pause ; crinière repérée par heuristique de teinte.
8. Figurines de carte : 2 états seulement.
9. Kit grossier non recuit.
10. Renversés qui glissent, armes à plat sur pente ; moutons, bovins, servants de bombarde manquants.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| P1 | Brancher `bow_walk`/`xbow_walk` | Moyen, permanent | S | 0 | — |
| P1 | Session AS7 avec check-list et A/B | Fort | S | 0 | joueur, DA |
| P1 | Tournage AS6 (taille, impacts, chute, marche, pas de côté, recul) | Fort | M | 0 | joueur |
| P2 | Pas du cheval < 15 % de patinage | Fort | M | 0 | tech |
| P2 | Pivot/pas de côté de l'infanterie (`turn_l/r`) | Moyen | M | 0 | IA formation |
| P2 | Clip `charge` à pied + poussée à l'impact | Moyen-fort | M | 0 | combat, audio |
| P2 | Secondaire par soldat, figé en pause | Faible-moyen | S-M | < 1 % | shader |
| P3 | Recuire le kit grossier | Moyen | M | 0 | tech |
| P3 | Clips appariés de duel | Moyen | L | 0 | cinématique |
| P3 | États de campagne (camp, siège, déroute) | Moyen | M | 0 | carte |
| P3 | Masque de crinière cuit, renversés posés sur le relief | Faible | S-M | 0 | terrain |
| P4 | Moutons, bovins, servants de bombarde | Faible | M | 0 | historien |
| P4 | Imposteurs `routing`, `dead` | Faible-moyen | M | atlas | tech |
