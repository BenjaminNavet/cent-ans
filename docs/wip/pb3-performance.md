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
| PB3e | Bataille : pas de sim N+1 dans un fil, piétinement en Rust, soldiers.update/étendards (ADR 0090) | 2 | lancé |
| PB3f | rayon : pré-calcul IA en lecture seule en parallèle, phases par province, déterminisme gardé (ADR 0091) | 2 | lancé |
| PB3g | Quadtree de relief (pas 9-12 ms, SZ6) en Rust, `request_reground` sans copie (70 ms), hameaux en lot (ADR 0092) | 2 | lancé |

## Journal
- 26/09 : audit, plan, vague 1 lancée (4 agents).
- 26/09 : PB3b fusionné (smoke, pb3b/rl1/pf1 OK après fusion de main). À refaire : mesures en plein écran Retina.
- 26/09 : PB3a fusionné ; dylib release reconstruite depuis main.
- 26/09 : PB3d puis PB3c fusionnés après fusion de la revue de code (conflits campaign_map.gd, battle_soldiers.gd résolus : garde de sauvegarde + compte dessiné `_drawn` avec condition `on_field` de la revue). Vague 2 lancée (PB3e, PB3f, PB3g).

- 26/09 ~09:45 : `main` recompilé (dylibs debug + release avec PB3a-d, smoke OK) après nettoyage de
  la crate `vegetation` contaminée dans `core/target` partagé (champ `detail` d'un worktree SZ).
- 26/09 : **PAUSE demandée par le joueur** pendant la vague 2 ; agents priés de commiter leur wip.

## Reprise (vague 2 en pause)
| Lot | Worktree | Branche (dernier commit) | État à la pause |
|---|---|---|---|
| PB3e | `.claude/worktrees/agent-aacdb076cb5ce2218` | `worktree-agent-aacdb076cb5ce2218` (9cd37691) | Pipeline du pas N+1 écrit (`sim/pipeline.rs`, `battle_step_job.rs`, tests bit-à-bit), **jamais compilé**. Reste : brancher `set_step_thread(true)`, piétinement/herbe en Rust (`StampMap` ; pas de mise à jour partielle de texture en Godot 4.7), `mm.buffer` sans réaffectation, p99 au banc, ADR 0090. Wip : `docs/wip/pb3e-bataille-fil.md` |
| PB3f | `.claude/worktrees/agent-a27afc73c1888787a` | `perf/pb3f-rayon` (84c1fda1) | Profil fait, aucun code : cœur 120-167 ms/tour dont `ai::plan_turn` ~80 % (`plan_economy` 66 ms, `Context::new` 28, `plan_armies` 10) ; `resolve_*` ~13 ms seulement → paralléliser dans `plan_turn`, pas les provinces. Instrumentation temporaire non commitée (`docs/wip/pb3f-profil-temporaire.patch`, à retirer). Wip : `docs/wip/pb3f-rayon.md` |
| PB3g | `.claude/worktrees/agent-a390841202fac1ad2` | `perf/pb3g-native-quadtree` (66f7ffb8) | Crate `relief-lod` (sélection, pages LRU, diff) compile, 1 test corrigé non relancé ; pont `ReliefLod` + pages partagées végétation (`Arc`) écrits, **jamais compilés** ; GDScript pas commencé. Wip : `docs/wip/pb3g-quadtree-natif.md` |

Pour reprendre : lire le fichier wip du lot DANS son worktree (pas dans main), relancer un agent
`cent-ans-dev` sur ce worktree/branche (il peut être verrouillé : `git worktree unlock`) avec la
consigne d'origine (ci-dessus, section Lots) et CARGO_TARGET_DIR privé `<worktree>/core/target`.
Après fusion de `main` dans une branche : `godot --headless --path game --import` avant les tests.
Intégration : fusion de `main` dans la branche, cargo fmt/clippy/test, smoke + tests du lot, ff-only.

## Prochaine étape
Reprendre PB3e, PB3f, PB3g (voir « Reprise ») ; puis mesures en plein écran Retina (PB3b) et
mesure en jeu de l'effet de PB3a (pb1_turns, banc bataille, dylib release).
