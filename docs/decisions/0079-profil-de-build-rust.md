# ADR 0079 — Profil de build Rust (Apple Silicon)

Date : 2026-09-26. Statut : accepté. Lot PB3a (`docs/wip/pb3-performance.md`), suite de
PB1/PB2.

## Contexte

Le crate `core` n'avait pas de `[profile.release]` explicite (LTO désactivé,
`codegen-units` par défaut) et le pont Godot (`godot-bridge`, crate qui construit des
`Dictionary`/`Array` et boucle sur les figurines chaque image via `get_units`,
`get_soldier_buffers`, etc.) n'avait pas d'`opt-level` dédié en dev : il héritait donc du
profil `dev` global (`opt-level = 0`), contrairement aux crates de simulation
(`sim-campaign`, `sim-battle`, `ai`, déjà en `opt-level = 2` depuis l'audit A5/T1) et aux
crates de décodage PNG. `core/build.sh` copiait toujours depuis
`$CORE_DIR/target/<profil>`, ce qui casse dès qu'on partage `CARGO_TARGET_DIR` entre
plusieurs worktrees d'agents (convention PB3).

## Options pour le profil release

- **A. Statu quo** (pas de `[profile.release]`) : build rapide, mais pas de LTO ni de
  fusion des unités de compilation ; le pont Godot (beaucoup de petites fonctions FFI)
  bénéficie particulièrement de l'inlining inter-crates que seul le LTO permet.
- **B. `lto = "fat"` + `codegen-units = 1`** : optimisation maximale (une seule unité de
  compilation par crate, le lieur voit tout le graphe d'appels), mais coûteux à
  compiler.
- **C. `lto = "thin"` + `codegen-units = 1`** : optimisation inter-crates plus légère
  (analyse par lots plutôt que globale), compilation nettement plus rapide.

`target-cpu` n'a volontairement pas été fixé (ni dans `release` ni dans `bench-native`
par défaut) : la cible `aarch64-apple-darwin` par défaut correspond déjà à un baseline
Apple M1, et imposer une cible plus récente (M4) à l'export ferait planter (instruction
illégale) le jeu chez les joueurs sur M1-M3. `panic = "abort"` a été explicitement écarté
car `gdext` (le pont Godot) attrape les panics Rust à la frontière FFI pour ne pas tuer le
process hôte, ce qui suppose le déroulement de pile (unwinding) par défaut.

## Mesures (machine partagée, très chargée par d'autres agents PB3 compilant en
parallèle sur le même `CARGO_TARGET_DIR` — échantillons uniques, pas de médiane de 3 ;
les temps incluent l'attente du verrou de build)

| Mesure | Avant / A | Après / B (fat) | C (thin) |
|---|---|---|---|
| Build release `-p godot-bridge` (compilation propre après changement de profil) | — | 32 min 30 s | **9 min 49 s** |
| Build dev `-p godot-bridge` (`opt-level` 0 → 2) | 4 min 40 s | 5 min 53 s | — |
| `turn_perf` (crate `ai`, 1400 tours de faction), dev vs release | mean 4,22 ms (dev) | mean 4,22 ms (release) | — |

`turn_perf` ne bouge pas entre dev et release : `ai`/`sim-campaign` sont déjà en
`opt-level = 2` en dev, donc ce banc ne montre pas l'effet du profil release (il ne passe
pas par `godot-bridge`). Les bancs sensibles au pont (`pb1_turns.gd`, banc bataille
`--benchmark`) n'ont pas pu être menés à terme dans le temps imparti : ce worktree
n'avait pas de cache d'import Godot (`game/.godot/`), et un premier import complet aurait
dépassé le budget de temps de ce lot. **Point ouvert**, à reprendre par un agent suivant
ou l'orchestrateur.

## Décision

- `[profile.release]` : `lto = "thin"`, `codegen-units = 1`,
  `debug = "line-tables-only"`. `lto = "thin"` retenu plutôt que `"fat"` : le gain de
  `"fat"` sur ce type de code (beaucoup de petites fonctions FFI, mais pas de boucle
  interne monolithique unique qui bénéficierait d'une fusion totale) ne semblait pas
  justifier un temps de build 3,3× plus long sur une machine de développement partagée
  entre plusieurs agents ; à réévaluer si un profilage (Instruments, via
  `debug = "line-tables-only"`) montre un goulot que seul le LTO complet lève.
- `[profile.bench-native]` (hérite de `release`) : profil optionnel pour des mesures
  locales avec `RUSTFLAGS="-C target-cpu=native"`, jamais utilisé pour l'export joueur.
- `[profile.dev.package.godot-bridge] opt-level = 2` : cohérent avec les autres crates
  hors chemin critique de test ; coût de compilation incrémentale mesuré (+ ~1 min 13 s
  sur cette machine chargée), jugé acceptable.
- `core/build.sh` copie désormais depuis `${CARGO_TARGET_DIR:-$CORE_DIR/target}`.

## Conséquences

- Le temps de build release reste élevé sur machine chargée (~10 min pour
  `-p godot-bridge` seul) ; un export complet (tous les binaires) sera plus long. À
  surveiller quand l'export joueur sera mis en place (pas encore le cas).
- Les mesures « fin de tour » et « banc bataille » debug vs release, demandées par le
  mandat de ce lot, restent à finaliser (import Godot manquant dans ce worktree) — voir
  `docs/wip/pb3a-build.md`, section « Prochaine étape ».
- Aucune dépense cloud (calcul local).
