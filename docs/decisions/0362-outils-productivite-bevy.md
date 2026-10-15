# 0362 — Outils de productivité du client Bevy : nextest, insta, sccache (optionnel)

Statut : accepté (15/10, lot `bm/devtools1`). Suite de `docs/research/bevy-outils-productivite.md` ; le joueur a décidé d'adopter tous les outils classés « adopt ».

## Contexte
Dizaines de worktrees `../gp-bm-*` en parallèle : chacun recompile Bevy, les tests bloqués ne sont pas bornés, les dumps d'état de l'agent n'ont aucune régression automatique. Disque de la machine presque plein (16 Go libres au moment de la mesure).

## Décision
1. **cargo-nextest** (0.9.148) : `client/.config/nextest.toml` (le client est son propre workspace). Profils `default` (60 s ×3 par test, une reprise pour `smoke` et `audio_playback`), `ci` (2 reprises, JUnit `target/nextest/ci/junit.xml`), `smoke`. `client/tests/test.sh` l'utilise et se replie sur `cargo test --features dev` s'il manque ; `smoke.sh` l'appelle avec `SMOKE_UNIT=1` ; nouveau workflow `.github/workflows/client-bevy.yml` (nextest profil `ci`, JUnit en artefact). `cargo test --features dev` reste valable et reste la commande du brief de fusion.
2. **insta** (1.49, dev-dependency `json yaml redactions filters`) : `client/tests/snapshots.rs`, 3 tests : dump d'agent intégré (compteur d'images et nombre d'entités masqués), scénarios de fumée analysés (chemin du dépôt remplacé par `[repo]`), ligne de commande type. Flux d'agent : `cargo insta test --features dev --test snapshots` puis `cargo insta review` (`accept` si voulu) ; commiter les `tests/snapshots/*.snap`, jamais les `.snap.new`. Toujours masquer les valeurs volatiles (images, durées, chemins).
3. **sccache** (0.18.0), **optionnel** : `source client/sccache-env.sh` (rien si sccache manque ; pas de `rustc-wrapper` dans `.cargo/config.toml`, qui casserait les builds sans sccache). Variables : `SCCACHE_DIR`, `SCCACHE_BASEDIRS=$HOME/dev` (préfixe retiré de la clé pour que `../gp-bm-a` et `../gp-bm-b` partagent), `SCCACHE_CACHE_SIZE` (défaut du script 4G tant que le disque est tendu). Piège trouvé : sccache refuse de tourner si `CARGO_INCREMENTAL=1` est exporté (le script le retire).

## Mesures (machine chargée, load average ~100 : le temps mural est bruité, le temps CPU est la référence)
Construction à froid d'un worktree, `cargo build --features dev`, `CARGO_PROFILE_DEV_DEBUG=0` (économie de disque ; mêmes réglages dans les trois essais). Charge de la machine ~100 pendant A et B.
| Essai (sccache 0.18, cache 4 Go) | Temps mural | Détail |
|---|---|---|
| A. sans sccache | 8 min 31 | 1856 s CPU utilisateur |
| B. sccache, cache vide | 8 min 28 | 381 échecs de cache (Rust 380), cache de 456 Mo |
| C. sccache, cache chaud, reconstruction à froid du `target/` | 4 min 44 | 381 succès sur 381 (100 %) |

Gain à cache chaud : -44 % (mural). Au-delà de 20 % : documenté, mais **toujours optionnel** (gain nul à froid, un seul cache à chauffer une fois). Le reste de C (4 min 44) est ce qui n'est pas mis en cache : 78 appels `crate-type` (dylib dont `bevy_dylib`, proc-macros), 8 incrémentaux, le crate client et l'édition de liens. Le cache doit être effacé quand le disque manque (`rm -rf $SCCACHE_DIR`).

**Infos de débogage (profil `dev`)**. `debug = "line-tables-only"` pour le projet et `debug = false` pour les dépendances (`[profile.dev.package."*"]`). Mesure : `cargo test --features dev --no-run` sur un `target/` neuf :

| | `target/` | `deps` | `incremental` | Temps mural |
|---|---|---|---|---|
| infos complètes (avant) | 16 Go | 11 Go | 6,3 Go | 10 min 21 |
| line-tables-only / false (après) | 6,8 Go | 3,7 Go | 3,6 Go | 9 min 37 |

Les paniques gardent `fichier:ligne:colonne` et les traces `RUST_BACKTRACE=1` (vérifié avec les mêmes options `rustc`) ; on perd seulement les variables locales dans lldb/gdb pour le code du projet (activer `debug = true` localement pour un débogueur : `CARGO_PROFILE_DEV_DEBUG=2`). L'incrémental reste activé (défaut) : il pèse 3,6 Go avec ces réglages au lieu de 6,3, il accélère la boucle d'édition et sccache ne le cache de toute façon pas. Les `target/` préchauffés des worktrees (100 Go observés, hachages obsolètes accumulés) doivent être supprimés au profit de ces réglages après fusion. Les 897 tests passent avec ces réglages.

## Limites
`bevy_dylib` (crate-type dylib) et les crates en incrémental (le client) ne sont pas mis en cache ; c'est le cas aussi du lien final. Le gain porte sur les dépendances du registre (Bevy et ses ~500 crates). L'effet de `SCCACHE_BASEDIRS` entre deux chemins n'a pas été mesuré (consigne disque : un seul worktree ; C est le même chemin) : les dépendances du registre ont le même chemin partout, seuls les crates `core/` et le client dépendent du chemin.
nextest ne lance pas les doc-tests (`test.sh` les lance à part ; le client n'en a pas). 897 tests : nextest 8,2 s, `cargo test` 7,7 s : pas de gain de vitesse, l'apport est les délais par test, les reprises et JUnit.

## Conséquences
Écarté pour l'instant (voir la note de recherche) : bevy_brp_*, debugdump, egui. sccache reste hors configuration partagée.
