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
- ADR réservés : 0079 (PB3a profil de build), 0080 (PB3b MetalFX), 0081 (PB3d fin de tour en fil).
- Mesures : machine partagée et très chargée → A/B alternés dans le même créneau, médianes de 3.
- Coordination : la revue de code (vague 2, `../gp-review`) corrige sim-battle, pont, GDScript
  bataille et carte ; PB3c reprend son item reporté « get_soldier_buffers groupés ». Fusionner
  `main` avant de rendre. SZ6 (pics scripts, `qt_update`) : PB3g (quadtree en Rust) attend sa fusion.

## Lots
| Lot | Contenu | Vague | État |
|---|---|---|---|
| PB3a | Profil release (LTO fat, codegen-units 1, line-tables), `godot-bridge` opt 2 en dev, dylib release reconstruite, mesures fin de tour/bataille dev vs release | 1 | **fusionné** a07458c5 (ADR 0079) : LTO thin (fat 32 min de build contre 10), codegen-units 1, pont en opt 2 en dev (build dev 4 m 40 → 5 m 53 sur machine chargée). Mesures d'effet en jeu à faire (pb1_turns, banc bataille) |
| PB3b | Mise à l'échelle MetalFX (spatiale/temporelle) dans `RenderQuality`, A/B GPU + captures (traînées végétation, `qt_vertex`) | 1 | **fusionné** 8955224f (ADR 0080) : spatial 0,75 en Moyenne/Haute, 0,67 en Basse, natif en Ultra ; temporel écarté (pluie effacée, fantômes des moulins : pas de vecteurs de mouvement pour les sommets animés par TIME). Gain d=150 32,1 → 23,4 ms, d=491 32,7 → 27,1, bataille 20,3 → 18,1 |
| PB3c | Bataille : cache des poses entre pas de sim, tampons par régiment pré-dimensionnés, un seul `get_units` par image, `get_siege` mutualisé | 1 | lancé |
| PB3d | Campagne : `end_turn` hors du fil principal ; `refresh_all` par appels groupés Packed*Array | 1 | lancé |
| PB3e | Bataille : pas de sim N+1 dans un fil, piétinement en Rust | 2 | après PB3c |
| PB3f | rayon : pré-calcul IA en lecture seule en parallèle, phases par province, déterminisme gardé | 2 | après PB3d |
| PB3g | Quadtree de relief (sélection) en Rust, hameaux/rivières/maillages de tuiles en Rust | 2 | après SZ6 |

## Journal
- 26/09 : audit, plan, vague 1 lancée (4 agents).
- 26/09 : PB3b fusionné (smoke, pb3b/rl1/pf1 OK après fusion de main). À refaire : mesures en plein écran Retina.
- 26/09 : PB3a fusionné ; dylib release reconstruite depuis main.

## Prochaine étape
Suivre la vague 1, fusionner lot par lot (build.sh + smoke + cargo test), puis vague 2.
