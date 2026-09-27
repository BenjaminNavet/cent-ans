# PB3 — Rust et M4 Pro : FPS et vitesse d'exécution

Demande du joueur (2026-09-26) : « quelle utilisation efficace de Rust et du M4 pour améliorer les
performances (FPS et rapidité d'exécution) » ; accord sur tout le plan (« ok pour tout »).
Orchestrateur : session PB3 (checkout principal). Coût cloud : 0 $ (calcul local).
Suite de PB1 (`pb1-benchmark-perf.md`) et PB2 (`pb2-vegetation-shader.md`).

## Constat (audit du 26/09)
- Campagne proche : GPU-bound, ~22-24 ms GPU (relief ~12, ombres ~7,5, MSAA ~5,5).
- Bataille : GPU 8-9 ms ; CPU par image gaspillé (poses recalculées, Dictionary).
- Fin de tour : 225-390 ms bloquants dans le fil principal ; sim et IA mono-cœur (10 P-cœurs libres).
- Pas de `[profile.release]` (ni LTO ni codegen-units=1) ; `godot-bridge` en opt-level 0 en dev ;
  dylib release périmée (24/09).

## Conventions
- Worktrees d'agents depuis `main` ; liens symboliques non versionnés `data/map/pyramid`,
  `tools/geo/raw` → dépôt principal.
- Compiler avec `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target` (disque : ~30 Go
  libres), copier la dylib dans le `game/bin` du worktree dans la même commande (rm avant cp).
- Commits `-- chemins` explicites, `wip:` ≤ 15 min, fusion ff-only par l'orchestrateur.
- ADR réservés : 0079 (PB3a), 0080 (PB3b), 0081 (PB3d), 0090 (PB3e), 0091 (PB3f), 0092 (PB3g).
- Cible de build : dossier PRIVÉ par worktree (`CARGO_TARGET_DIR=<worktree>/core/target`), supprimé à la fin : la cible partagée a mélangé les crates de worktrees différents (PB3c, PB3d). Disque : ~229 Go libres.
- Mesures : machine partagée et très chargée → A/B alternés dans le même créneau, médianes de 3.
- Coordination : la revue de code (vague 2, `../gp-review`) corrige sim-battle, pont, GDScript
  bataille et carte ; PB3c reprend son item reporté « get_soldier_buffers groupés ». Fusionner
  `main` avant de rendre. SZ6 (pics scripts, `qt_update`) : PB3g (quadtree en Rust) attend sa fusion.

## Lots
| Lot | Contenu | Vague | État |
|---|---|---|---|
| PB3a | Profil release (LTO fat, codegen-units 1, line-tables), `godot-bridge` opt 2 en dev, dylib release reconstruite, mesures fin de tour/bataille dev vs release | 1 | **fusionné** a07458c5 (ADR 0079) : LTO thin (fat 32 min de build contre 10), codegen-units 1, pont en opt 2 en dev (build dev 4 m 40 → 5 m 53 sur machine chargée). Mesures d'effet en jeu à faire (pb1_turns, banc bataille) |
| PB3b | Mise à l'échelle MetalFX (spatiale/temporelle) dans `RenderQuality`, A/B GPU + captures (traînées végétation, `qt_vertex`) | 1 | **fusionné** 8955224f (ADR 0080) : spatial 0,75 en Moyenne/Haute, 0,67 en Basse, natif en Ultra ; temporel écarté (pluie effacée, fantômes des moulins : pas de vecteurs de mouvement pour les sommets animés par TIME). Gain d=150 32,1 → 23,4 ms, d=491 32,7 → 27,1, bataille 20,3 → 18,1 |
| PB3c | Bataille : cache des poses entre pas de sim, tampons par régiment pré-dimensionnés, un seul `get_units` par image, `get_siege` mutualisé | 1 | **fusionné** 2ccea813 : grosse bataille release 25 → 40 i/s (surtout `terrain.in_water` en tronçons, −15 ms/image) ; siège sans gain mesurable. Reste : soldiers.update 3,3 ms, étendards 1,7, audio 1,3, herbe 1,2 |
| PB3d | Campagne : `end_turn` hors du fil principal ; `refresh_all` par appels groupés Packed*Array | 1 | **fusionné** 3a74df31 (ADR 0081) : pire image fin de tour 229 → 78 ms (release), refresh_all ~59 ms ; fil en QoS USER_INITIATED. Reste : marqueurs d'armée 8-50 ms |
| PB3e | Bataille : pas de sim N+1 dans un fil, piétinement en Rust, soldiers.update/étendards (ADR 0090) | 2 | **fusionné** a56437e1 : p99 image grosse bataille release 33,4 → 24,0 ms, neige 36,4 → 20,8 ; pics venaient des cartes au sol (piétinement/herbe → `StampMap` Rust), tick pire 1,5 → 0,2 ms ; tampons non réaffectés si inchangés ; `--no-pb3e`. Reste : étendards 1,7 ms, audio 1,3, image suivant un pas +3,5 ms (poses dans le fil, get_units groupé) ; toute méthode du pont qui modifie la bataille hors pas doit appeler `touch_poses` |
| PB3f | rayon : pré-calcul IA en lecture seule en parallèle, phases par province, déterminisme gardé (ADR 0091) | 2 | **fusionné** 84023504 : cœur de fin de tour 108 → 69 ms (−36 %, dont −20 % par suppression de calculs répétés dans plan_turn), pb1_turns release 166 → 130 ms ; pool rayon 10 fils USER_INITIATED ; factions toujours séquentielles ; test bit-à-bit 3 graines × 12 tours. Pire image de fin de tour inchangée (~65-110 ms, rafraîchissement GDScript) |
| PB3g | Quadtree de relief (pas 9-12 ms, SZ6) en Rust, `request_reground` sans copie (70 ms), hameaux en lot (ADR 0092) | 2 | **fusionné** 818030e1 : sélection quadtree pire 14,9 → 0,22 ms, `lod/quadtree` pire 24 → 9 ms, images > 50 ms 3 → 0, zoom France↔Paris p50 31,8 → 25,1 / p95 46,9 → 34,7 ; `request_reground` 4,7 → 0,04 ms ; repli `--no-native-quadtree`. Reste : `qt/collect` ≤ 9 ms (écouteurs surface_changed), TownLayer, NextHintController |

## Journal
- 26/09 : audit, plan, vague 1 lancée (4 agents).
- 26/09 : PB3b fusionné (smoke, pb3b/rl1/pf1 OK après fusion de main). À refaire : mesures en plein écran Retina.
- 26/09 : PB3a fusionné ; dylib release reconstruite depuis main.
- 26/09 : PB3d puis PB3c fusionnés après fusion de la revue de code (conflits campaign_map.gd, battle_soldiers.gd résolus : garde de sauvegarde + compte dessiné `_drawn` avec condition `on_field` de la revue). Vague 2 lancée (PB3e, PB3f, PB3g).

- 26/09 ~09:45 : `main` recompilé (dylibs debug + release avec PB3a-d, smoke OK) après nettoyage de
  la crate `vegetation` contaminée dans `core/target` partagé (champ `detail` d'un worktree SZ).
- 26/09 : **PAUSE demandée par le joueur** pendant la vague 2 ; agents priés de commiter leur wip.
- 26/09 : **REPRISE** demandée par le joueur ; PB3e, PB3f, PB3g relancés dans leurs worktrees.
- 26/09 : PB3g fusionné (smoke, pb3g, sz1, zg2, zg8, settlements OK après fusion de main).
- 26/09 : PB3f fusionné (conflit Cargo.lock régénéré ; cargo + smoke, cv1, c5, fr1, ct1, pb3g OK).
- 26/09 : PB3e fusionné (conflit lib.rs : modules `relief_lod_bridge` + `stamp_map`). **PB3 terminé** : 7 lots dans main ; dylibs de main reconstruites.

## Prochaine étape (pistes restantes, non lancées)
- Mesures en plein écran Retina (PB3b) ; effet en jeu de PB3a (pb1_turns, banc bataille, release).
- Fin de tour : pire image ~65-110 ms = rafraîchissement GDScript (marqueurs d'armée 8-50 ms, maquettes de croissance).
- Bataille : poses calculées dans le fil du pas, `get_units` en tableaux groupés, étendards, audio.
- Carte : `qt/collect` ≤ 9 ms (écouteurs `surface_changed`), `TownLayer` ~10 ms, `NextHintController.refresh` 15 ms/s.
