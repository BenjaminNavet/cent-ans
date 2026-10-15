# Écosystème Bevy : outils de productivité de développement pour Cent Ans

Date : 2026-10-15. Recherche seule, rien installé. Sources : bevy.org/assets, API crates.io (versions, dépendances `bevy`, téléchargements, licences vérifiés ce jour), GitHub, notes de version Bevy 0.20.

## Déjà en place (client/Cargo.toml)

- Bevy `=0.20.0` (publié le 2026-10-08), features `jpeg png webp mp3 wav`.
- `dev` = `bevy/dynamic_linking`.
- `remote` = `bevy/bevy_remote` (BRP HTTP).
- `profile` = `bevy_ecs` avec `trace` et `debug`, plus `tracing` et `tracing-subscriber` (timings par système, banc `--bench-map`).
- `[profile.dev] opt-level = 1`, dépendances à 3.
- Tests : `wesl` et `naga` (validation des shaders).
- Rien d'autre côté outillage : ni egui, ni asset loader, ni lint.

Contrainte transversale : Bevy 0.20 est sorti il y a une semaine. Beaucoup de crates sont encore sur 0.19. La colonne « Bevy » ci-dessous donne ce qui est déclaré dans la dépendance `bevy` de la dernière version publiée.

## Compilation

| Outil | Apport concret | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| Édition de lien rapide (macOS : `ld-prime` d'Xcode, déjà le défaut ; Linux/CI : mold ou lld) | Peu à gagner sur macOS ; vrai gain sur CI Linux. Configuration `.cargo/config.toml`, sans crate. | indépendant | standard | MIT | adopt côté CI Linux, rien à faire sur Mac |
| `sccache` (ou cache `Swatinem/rust-cache` en CI) | Les worktrees parallèles ne partagent pas `target/`, donc chaque agent recompile Bevy (~minutes). Un `RUSTC_WRAPPER=sccache` partagé évite ce coût. Point d'attention : la règle projet « ne pas partager un répertoire de build » concerne `target/`, pas le cache de compilation, qui est sûr. | indépendant | très mûr (Mozilla) | Apache-2.0 | adopt (gain le plus probable pour agents en parallèle) |
| Backend Cranelift (`-Zcodegen-backend=cranelift`, nightly) | Compilation de débogage plus rapide, mais exige un nightly et ne couvre pas bien aarch64 macOS ni les dépendances lourdes. Non vérifié ce jour. | nightly | expérimental | MIT/Apache | watch |
| `cargo-hakari` (workspace-hack) | Unifie les features dans un workspace (client + 7 crates core) : moins de recompilations quand on alterne `-p`. Non vérifié ce jour. | indépendant | mûr (Guppy) | MIT/Apache | try |
| `bevy/dynamic_linking` | Déjà utilisé (`dev`). | | | | déjà là |
| `cargo-nextest` | Exécution parallèle des tests avec une sortie filtrable (JUnit, retry, timeouts par test, utile contre les tests bloqués). | indépendant | très mûr | MIT/Apache | adopt |

## Rechargement à chaud

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| [subsecond](https://crates.io/crates/subsecond) (Dioxus) | Hot-patch du code Rust à l'exécution, sans relancer. Mais c'est un runtime : il faut `dx serve --hot-patch` (CLI Dioxus), et seul le crate « tip » est patché. Utile pour itérer sur l'UI/rendu, peu pour des agents headless. | agnostique ; 0.7.10, 2026-10-14 ; 2,4 M téléchargements | mûr | MIT/Apache | watch |
| [bevy_simple_subsecond_system](https://github.com/janhohenheim/bevy_simple_subsecond_system) | Attribut `#[hot]` sur systèmes. Dernier commit 2026-02-25, crate publiée 2025-06 en 0.2.0. Probablement pas sur 0.20. | ~0.16 | peu actif | MIT/Apache | skip (successeur : `bevy_hotpatching_experiments`, 0.18 seulement) |
| [dexterous_developer](https://github.com/lee-orr/dexterous_developer) | Rechargement par bibliothèques dynamiques. Dernière publication 2024-08. | 0.14 | abandonné | MIT/Apache | skip |
| Hot-patching natif Bevy | Pas dans 0.20 ; prévu (note de conception, via subsecond). | | | | watch |
| Rechargement d'assets (`file_watcher`) | Feature intégrée de Bevy (`bevy/file_watcher`), pas une crate. Recharge les JSON/images/shaders modifiés. Voir « Données / assets ». | 0.20 | natif | MIT/Apache | adopt (feature, à vérifier) |

## Inspection / éditeur

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| [bevy_egui](https://crates.io/crates/bevy_egui) 0.43.1 | Base de tous les panneaux de debug en jeu. Requis par console, inspecteur, config. | **0.20**, 2026-10-10 ; 2,7 M | très mûr | MIT | adopt (si on veut un panneau de debug) |
| [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui) 0.37.0 | Inspecteur d'entités/ressources dans le jeu. | **0.19 seulement** (2026-06-20) ; 1,5 M | très mûr | MIT/Apache | watch (pas encore sur 0.20 ; attendre la montée de version) |
| [bevy_mod_debugdump](https://github.com/jakobhellermann/bevy_mod_debugdump) 0.16.0 | Export du graphe de schedules (dot) : savoir qui tourne avant qui, cible directe pour les agents et pour le diagnostic de conflits de systèmes. | **0.19 seulement** ; 290 k | mûr | MIT/Apache | watch (grand intérêt dès le passage à 0.20) |
| Inspecteur distant (VS Code `splo/vscode-bevy-inspector`, `bevy_inspector.nvim`) | Parlent BRP ; couvrent l'inspection sans embarquer egui. | main / sans version | petit | ? | try (VS Code) |
| Éditeur Bevy officiel | Non livré ; la « remote inspector » est annoncée dans « What's next » de 0.20. | | | | watch |
| bevy_editor_pls, Yoleck, skein, Renzora | Éditeurs de niveau / scène. Hors périmètre (nos niveaux sont des JSON de `data/` et notre éditeur de carte existe). | divers | | | skip |

## Débogage / profilage

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| Diagnostics natifs (`FrameTimeDiagnosticsPlugin`, `LogDiagnosticsPlugin`, `EntityCountDiagnosticsPlugin`), `bevy/trace_tracy`, `bevy/trace_chrome` | Déjà dans Bevy, rien à ajouter. Le projet a son propre calque tracing ; Tracy donne en plus les captures par frame. | 0.20 | natif | MIT/Apache | try (Tracy pour un goulot précis) |
| [bevy_mod_debugdump](https://github.com/jakobhellermann/bevy_mod_debugdump) | Voir ci-dessus. | 0.19 | | | watch |
| Gizmos natifs (`Gizmos`, `GizmoConfig`) | Lignes de debug (zones, trajets IA, chemins) sans crate. | 0.20 | natif | | adopt |
| Erreurs avec contexte (0.20) | Panics dans systèmes/commandes routés vers le gestionnaire d'erreur : meilleurs logs d'agents. Natif. | 0.20 | natif | | adopt (le configurer pour écrire en JSON dans les logs) |
| Randomisation de schedule (0.20, feature `debug`, `shuffle_seed`) | Détecte les dépendances d'ordre implicites entre systèmes : très utile avec de nombreux agents qui ajoutent des systèmes. Natif. | 0.20 | nouveau | | try (un test CI avec graine fixe) |
| [bevy_console](https://github.com/RichoDemus/bevy-console) 0.19.0 | Console déroulante (commandes tapées : « set gold 500 », « teleport army »). | **0.20** (bevy_egui 0.43) ; 75 k | moyen | MIT/Apache | try (humain surtout ; les agents ont BRP) |
| [bevy_framepace](https://crates.io/crates/bevy_framepace) 0.22 | Limiteur de FPS/pacing. | 0.19 | mûr | MIT/Apache | skip |
| [bevy_screen_diagnostics](https://github.com/laundmo/bevy_screen_diagnostics) | Overlay FPS. Le natif `bevy_dev_tools` (FPS overlay) suffit. | 0.16 | | | skip |
| iyes_perf_ui, bevy_metrics_dashboard | Overlays de perf. | 0.5/0.8, 2025 | | | skip |
| puffin / tracy-client | Profileurs externes ; hors Bevy. `samply` (Mac) pour un profil CPU sans modifier le code. | indépendant | | MIT/Apache | try (samply) |
| bevy_lint | Voir « CI / lint ». | | | | |

## Tests / captures

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| Captures natives (`Screenshot::primary_window()` / `Screenshot::image(...)`, observeur `save_to_disk`) | Capture hors écran dans un fichier, sans fenêtre au premier plan si on rend dans une cible image. Natif. | 0.20 | natif | | adopt (déjà probable ; vérifier le rendu offscreen sans fenêtre) |
| [bevy_brp_extras](https://crates.io/crates/bevy_brp_extras) 0.23.0 | Ajoute à BRP : capture d'écran, arrêt propre, injection de clavier. L'agent peut dire « screenshot » sans code de capture dédié dans le client. | **0.20**, 2026-10-09 ; 15 k | jeune, actif | MIT/Apache | try (prioritaire) |
| [bevy-autoplay](https://github.com/tobyselway/bevy-autoplay) | Rejoue des sessions enregistrées comme tests d'intégration. Dernier commit 2024-06, Bevy 0.13. | 0.13 | abandonné | ? | skip (idée à copier : journal d'entrées + graine, dans notre propre outil) |
| `insta` (snapshots de texte/JSON) | Tests d'instantanés pour les dumps d'état : `cargo insta review` pour accepter les changements. Fonctionne sur les dumps JSON existants. Non vérifié ce jour. | indépendant | très mûr | Apache-2.0 | adopt |
| Comparaison d'images (`image-compare`, `dssim`) | Régression visuelle : seuil sur différence entre capture et référence. Les références sont déjà proscrites dans git (captures ignorées), donc à stocker hors dépôt. Non vérifié ce jour. | indépendant | | MIT/Apache | watch |
| [bevy_testing](https://crates.io/crates/bevy_testing) 0.1.2 | Aide aux tests d'ECS ; dernière maj 2024-08, 3,6 k téléchargements. | ancienne | abandonné | ? | skip |
| `App::new()` + `MinimalPlugins` + `app.update()` | Méthode standard pour tester des systèmes sans fenêtre. Natif. | 0.20 | natif | | adopt |
| Randomisation de schedule | Voir ci-dessus. | | | | try |

## Données / assets

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| [bevy_common_assets](https://github.com/NiklasEi/bevy_common_assets) 0.18.0 | Chargeur JSON/RON/TOML/YAML typé (serde) en `Asset` ; avec `file_watcher`, un JSON de `data/` modifié recharge à chaud (lien direct avec nos données). | **0.20**, 2026-10-09 ; 342 k | mûr | MIT/Apache | adopt (si les données passent par l'AssetServer) |
| [bevy_asset_loader](https://github.com/NiklasEi/bevy_asset_loader) 0.28.1 | Déclare les assets d'un état (`LoadingState`) : écran de chargement, erreurs claires si un fichier manque. | **0.20**, 2026-10-13 ; 662 k | très mûr | MIT/Apache | try |
| Asset processing natif (0.20 : `CompressedImageSaver` avec mipmaps) | Traitement d'assets au build. Natif. | 0.20 | natif | | try |
| [bevy_mod_config](https://crates.io/crates/bevy_mod_config) 0.9.0 | Réglages structurés avec UI egui. | **0.20** ; 5 k | jeune | ? | watch |
| [bevy_persistent](https://crates.io/crates/bevy_persistent) 0.11 / [bevy_pkv](https://crates.io/crates/bevy_pkv) 0.16 | Ressources persistantes (réglages, sauvegardes simples). | 0.19 | mûr | MIT/Apache | watch |
| [moonshine-save](https://crates.io/crates/moonshine-save) 0.8.0 | Sauvegarde/chargement d'entités sélectionnées. Notre état de campagne vit dans `core/` (Rust pur), donc peu utile. | **0.20** ; 45 k | moyen | MIT | skip |
| BSN (scènes, 0.20) | Nouveau format de scène, « hot-reloadable » prévu. Évolue vite ; ne pas bâtir dessus aujourd'hui. | 0.20 | instable | | watch |

## Entrées / états

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| [bevy_enhanced_input](https://crates.io/crates/bevy_enhanced_input) 0.27.0 | Actions contextuelles, utile pour les contrôles de bataille façon Total War ; remplaçable en test par l'injection d'actions (donc scénarios sans clavier réel). | **0.20**, 2026-10-14 ; 182 k | actif | MIT/Apache | try |
| [leafwing-input-manager](https://crates.io/crates/leafwing-input-manager) 0.21.0 | Idem, plus ancien. | 0.19 seulement | mûr | MIT/Apache | watch |
| `bevy_state` (inclus dans Bevy) | Déjà disponible. | 0.20 | natif | | déjà là |
| pyri_state, seldom_state, moonshine-behavior | Machines à états alternatives ; apportent peu face à `bevy_state` pour notre cas. | 0.18-0.20 | | | skip |
| bevy_tweening 0.16 | Interpolations (gameplay/UI, hors périmètre de ce relevé). | 0.19 | | | skip |

## IA / MCP

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| [bevy_brp_mcp](https://github.com/natepiano/bevy_brp) 0.23.0 | Serveur MCP pour piloter une appli Bevy via BRP : lancer/arrêter, lister et muter entités/composants, observer, lire les logs. Les versions de la crate suivent Bevy (0.19 -> 0.22.3, 0.20 -> 0.23.0). Il ouvre à nos agents ce que `--features remote` expose déjà, sans écrire d'outil de dump maison. À associer à `bevy_brp_extras` pour capture d'écran et clavier. | **0.20**, 2026-10-09 ; ~4 k téléchargements, ~850 récents | jeune mais suivi (même auteur, mêmes jours que Bevy) | MIT OR Apache-2.0 | try (prioritaire) |
| [bevy_brp_extras](https://crates.io/crates/bevy_brp_extras) 0.23.0 | Voir « Tests / captures ». | 0.20 | | MIT/Apache | try |
| [bevy_ai_editor](https://github.com/cats2333/bevy_ai_editor) 0.3.1 | Éditeur à distance piloté par script Python. 189 téléchargements, Bevy 0.18. | 0.18 | marginal | ? | skip |
| [bevy_mod_scripting](https://crates.io/crates/bevy_mod_scripting) 0.23.0 | Scripts Lua/Rhai pour mods. | 0.20 | actif | MIT/Apache | skip (hors sujet : règles dans `core/`) |

Note : BRP est déjà dans le client (`remote`). L'apport de `bevy_brp_mcp` est d'abord la commodité d'outillage pour les agents (schémas d'outils, reconnexion, lancement). À évaluer contre un script `curl` JSON-RPC maison avant de l'adopter pour de bon. Attention : les instructions projet interdisent de voler le focus. Un MCP qui lance le jeu fenêtré doit passer par `tools/godot_bg.sh` ou son équivalent Bevy, ou le rendu doit être offscreen.

## CI / lint

| Outil | Apport | Bevy / maj | Maturité | Licence | Verdict |
|---|---|---|---|---|---|
| [bevy_lint](https://thebevyflock.github.io/bevy_cli/bevy_lint) (Bevy CLI) | Lints propres à Bevy (par ex. requêtes inutiles, `Res<Time>` vs `Time<Virtual>`). Dernière version `lint-v0.6.0` (2026-02-01), toolchain nightly épinglée (`nightly-2026-01-22`) : probablement calée sur Bevy 0.18, pas 0.20. Le CLI est en alpha (septembre 2025). La crate `bevy_lint` de crates.io est un squat vide de 2022 : installer depuis GitHub. | non confirmé pour 0.20 | alpha | MIT/Apache | watch (compatibilité 0.20 à vérifier avant tout essai) |
| `cargo clippy -- -D warnings` | Déjà exigé par les règles projet. | | | | déjà là |
| [bevy_github_ci_template](https://github.com/bevyengine/bevy_github_ci_template), [bevy_game_template](https://github.com/NiklasEi/bevy_game_template) (0.20.0) | Modèles de workflows de release et CI (matrice Windows/Linux/macOS). Notre CI existe déjà (promotion `stable` après smoke) ; ne servent que de référence. | 0.18 / 0.20 | | MIT/Apache | skip (lire pour idées) |
| `cargo-deny`, `cargo-udeps`/`cargo-machete` | Licences et dépendances inutilisées sur un gros workspace, utile avec beaucoup d'agents qui ajoutent des crates. Non vérifié ce jour. | indépendant | mûr | MIT/Apache | try |

## Top 10 (par rapport gain / risque)

1. **sccache (ou cache `target` partagé de CI)** : supprime la recompilation de Bevy dans chaque worktree d'agent. Adopt.
2. **cargo-nextest** : sorties parsables, timeouts par test, retries ; protège contre les tests bloqués. Adopt.
3. **bevy_brp_extras** (0.23.0, Bevy 0.20) : captures d'écran, arrêt propre et clavier via BRP, sans code maison. Try.
4. **bevy_brp_mcp** (0.23.0, Bevy 0.20) : pilotage d'un jeu lancé par un agent, inspection et mutation d'entités. À comparer à un script JSON-RPC maison. Try.
5. **insta** : snapshots des dumps JSON d'état (revue des changements au lieu de diff à la main). Adopt.
6. **Randomisation de schedule (natif 0.20, feature `debug` + `shuffle_seed`)** : révèle les dépendances d'ordre implicites quand de nombreux agents ajoutent des systèmes. Try, dans un test CI à graine fixe.
7. **bevy_common_assets + `file_watcher`** (0.18.0, Bevy 0.20) : JSON de `data/` typés et rechargés à chaud. Adopt si les données passent par l'AssetServer.
8. **bevy_mod_debugdump** (0.16.0, Bevy 0.19 seulement) : graphe de schedules pour diagnostiquer les conflits de systèmes. Watch jusqu'à la montée en 0.20.
9. **bevy_asset_loader** (0.28.1, Bevy 0.20) : états de chargement et erreurs claires sur assets manquants. Try.
10. **bevy_egui 0.43 + bevy_console 0.19** (Bevy 0.20) : console et panneaux de debug pour l'humain ; l'inspecteur (`bevy-inspector-egui`) viendra quand il passera à 0.20. Try.

À surveiller : hot-patching (subsecond) et inspecteur distant officiel, annoncés mais non livrés dans 0.20 ; `bevy_lint` (compatibilité 0.20 à confirmer).

## Limites de ce relevé

- Versions, dépendances `bevy`, téléchargements et licences viennent de l'API crates.io lue le 2026-10-15. Les entrées marquées « non vérifié » (cranelift, hakari, insta, nextest, sccache, deny, image-compare, samply) viennent de connaissances générales, pas de la lecture des sources ce jour.
- Pour BRP/`bevy_brp_mcp`, la liste précise des outils MCP n'a pas pu être relue (README du dépôt sans détail) ; la description crates.io de `bevy_brp_extras` mentionne capture d'écran, arrêt et clavier.
- Licence « ? » : non lue.
- Les sections « Editors » et « Testing » de bevy.org/assets n'existent pas en tant que telles ; le seul outil de test listé est `bevy-autoplay` (abandonné).
