COMMON RULES (Bevy migration of the game "Cent Ans", wave 3):
- You work ONLY in your worktree (given below; branch bm/<lot>, from bevy-migration). Never touch other directories, never push, never merge. Commit with `git commit -m "..." -- <explicit paths>` (never -a / add -A), no Co-Authored-By. Commit a first skeleton early, then small commits.
- Read first: client/README.md, docs/decisions/0357-architecture-client-bevy.md. Client = Bevy 0.20.0 in client/ (own workspace). Code in English; UI strings in French.
- You own ONLY the folder(s) named in your lot. Do NOT edit client/src/main.rs, client/src/lib.rs, other lots' folders, or the agent/ sim/ app/ modules. If you truly need a change there (e.g. a new Cargo dependency in client/Cargo.toml), make it minimal and list it in your report. Need a game rule? It belongs in core/ (ask nothing; just note it in your report).
- Reference implementation: the Godot GDScript in game/scripts/... (and game/shaders/*.gdshader) of the same repo. Port behaviour and look faithfully, idiomatically in Bevy (ECS systems, WGSL materials via ExtendedMaterial / Material when needed). Data comes from data/ (via data-model where it already parses it, else read the JSON/PNG/bin directly, documenting the format). Assets: game/assets (Bevy asset root).
- Build: `cd client && cargo build --features dev` (target dir is pre-warmed; incremental is seconds). Before committing: `cargo fmt`, `cargo clippy --features dev --all-targets -- -D warnings`, `cargo test` (if a plain `cargo test` would rebuild Bevy without `dev`, use `cargo test --features dev`).
- Verify WITHOUT screenshots (you must not take or view screenshots): register a dump section (`app.register_dump(...)`, see client/README.md) exposing counts/state of your plugin, then run `CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug client/run_bg.sh --state <State> --dump-state out/<lot>.json` (or `--script`) and check the JSON. run_bg.sh opens a background window; that is allowed. Never launch the binary any other way with a window.
- Keep the run_bg launches few (each ~10-30 s). Commands > 1 min: fine but avoid loops.
- Time box: aim to finish within ~2 h of work. Better a solid, tested, faithful subset than a broad broken one; list precisely what remains.
- Final report (≤15 lines, concise): what is ported (Godot file → Rust file), dump evidence, deps added, edits outside your folder, what remains (with Godot file names), known issues.
WAVE 3 ADDITIONS (read client/README.md first: it documents everything merged so far):
- Merged in bevy-migration: UI kit (ui/kit; a kit-fix lot for inner padding / scrollbars runs in parallel, don't work around it), menu (main menu, faction select, saves, pause, `menu::KeyEdges`), settings (`Res<Settings>`, `settings::OpenSettings`, F10), campaign HUD (top bar, alerts, end of turn on a thread, season report, decisions), panels province / tech (F4) / diplomacy (F3) / army+recruitment, terrain+camera, provinces, water, markers, armies, audio, battle scene+units (instanced). game-api modules: army, common, diplomacy, faction, hud, pprovince, province, recruit, saves, tech, turn.
- Script keys now produce `just_pressed` normally (fixed). Escape priority: the pause menu opens on Escape only if no kit modal/dialog is open; a panel that closes on Escape must run `.before(crate::menu::pause::pause_escape)` and set `KeyEdges::escape = false` when it consumed it.
- `client/run_bg.sh` forwards every `CENT_ANS_*` env var to the app (use them for dev hooks like `CENT_ANS_OPEN_PROVINCE`).
- Loading a save re-enters `AppState::Campaign`: campaign plugins must (re)build in `OnEnter(Campaign)` from `Sim::state` and clean up with `DespawnOnExit`.
- Panels open through `OpenPanel` (client/src/campaign/panels/mod.rs): Province, Army, Recruitment, Diplomacy, Tech, Siege(id), Character(id), Council, Faction(id), Feudal, Chronicle, CloseAll. Match it with a `_ =>` arm (variants get added).
- Campaign ground height on the CPU: `crate::campaign::terrain::surface::GroundHeight` (SystemParam, `.at(x, z)` → displayed world y incl. the dynamic exaggerated relief; changes with the camera zoom, so re-place objects when `ReliefState` changes). The old `water::GroundHeight` resource and `provinces::SurfaceHeight` are flat placeholders. A shared GPU relief module (WESL import) is being written by the campaign-look lot; don't write your own copy of the relief formula.
- Battle hand-over: `battle::scene::PendingBattle` (filled by flow before `AppState::Battle`), `battle::scene::BattleSession` (`sim_mut()`, `snapshot`), `flow::LeaveBattle { abandoned }` (sent by the battle result screen).
- Merge conflicts: append to client/README.md under your own `##` section only; docs/wip/bm-<lot>.md for your note.
- Other shared interfaces: `battle::orders::{BattleSelection, SelectBattleUnits}` (selected regiments by `sim_battle::Unit::id`), `campaign::camera::FocusCamera` (Province/Army/Point/Follow), `ui::kit::panel_stack` (`StackedPanel` on panel roots), `ui::kit::icons::icon(...)`, `ui::kit::tooltip_spec::TooltipSpec`.
- First report line: branch + final commit hash. Leave nothing uncommitted (check `git status --untracked-files=all client/src core` before you finish: an untracked .rs file breaks the merge).

## Vague 4 (ajouts)
- Base : `bevy-migration` contient toute la vague 3 (413 tests). Lis `docs/wip/bm-<ton dossier>.md` s'il existe : ce sont les restes du lot précédent, ton point de départ.
- Ordres clavier/souris de campagne : ajoute `.run_if(crate::campaign::hud::flow::orders_open)` (ou le chemin réel, cherche `orders_open`) pour ne rien émettre pendant le thread de fin de tour.
- Hauteur du sol : `campaign::terrain::surface::GroundHeight` ; shaders drapés : module `campaign/terrain/relief.wesl` (patron documenté en tête du fichier).
- Brouillard de guerre : ressource `campaign::weather::fog::FogOfWar`. Zoom : `campaign::camera::zoom::ZoomView` (poids des vues, alpha frontières/brouillard).
- Échap : consomme `KeyEdges` et ordonne ton système `.before(pause_escape)` si ton panneau doit se fermer avant la pause.

## Vague 5 (ajouts)
- Base : vagues 1 à 4 fusionnées (client 599 tests, game-api 63). Les restes de chaque domaine sont dans `docs/wip/bm-*.md` : lis ceux de ton dossier en premier.
- Touches : toute touche de ton dossier passe par `settings::KeyMap` (`keymap.just_pressed("<action>", &keys)`, actions nommées dans `settings::ACTIONS`). Si une action manque, ajoute-la dans ton code avec un repli sur la touche par défaut et signale-la dans ton rapport (le dossier settings n'est pas à toi).
- Batailles en preuve : `CENT_ANS_NO_DEPLOY=1` saute la phase de déploiement. Muet : `CENT_ANS_MUTE=1`.
- Les commandes sur une entité peut-être détruite dans la même image : `try_insert` / `try_remove` (un panic de ce type vient d'être corrigé dans life/models.rs).

## Consigne du joueur (15/10 ~7 h)
- « On n'est pas obligé de reproduire exactement le jeu comme dans Godot, si on peut refaire en mieux. » Godot est la référence des fonctions et des données, pas une spécification : améliore la conception, l'ergonomie, le rendu ou la structure quand c'est nettement mieux, et note l'écart dans ta note wip. Les règles restent dans core/.

## Fin de lot (à partir de 15/10 ~7 h)
- Dernière action, après ton dernier commit et la vérification `git status` : `touch /Users/jean_hubert/dev/gp-bm-<lot>/.bm-ready` (fichier non suivi). L'agent de fusion permanent surveille ces marqueurs et fusionne ton lot.
- Puis, tout dernier geste (disque) : `rm -rf /Users/jean_hubert/dev/gp-bm-<lot>/client/target /Users/jean_hubert/dev/gp-bm-<lot>/core/target` (la fusion recompile dans gp-bevy).

## RÈGLE JOUEUR (15/10, prioritaire) : ni focus ni son
- Tout lancement du client Bevy : `CENT_ANS_MUTE=1` ET `--offscreen` (aussi pour dumps et scripts), toujours via client/run_bg.sh. Jamais de fenêtre visible, jamais de son.
- Godot seulement via tools/godot_bg.sh (muet), uniquement quand indispensable (captures de référence).

## RÈGLE JOUEUR (15/10) : réutiliser SES assets
- Toujours charger les assets existants du joueur (game/assets : icônes, médaillons, enluminures, modèles glb, animations, sons, musiques, voix) exactement comme Godot les choisit. Ne jamais redessiner par code, ni remplacer par une forme ou un son généré, ce qui existe en asset. Format non lisible par Bevy (SVG, .tres/.res) : le signaler et proposer une conversion, ne pas l'escamoter.

## Compilation (15/10, après devtools1)
- Si ta branche n'a pas encore `bm/devtools1` (profil debug allégé), fais `git merge bm/devtools1` d'abord (s'il n'existe plus, il est dans bevy-migration). Supprime ensuite un `client/target` cloné ancien (hachages obsolètes, ~100 Go) : un neuf fait ~7 Go.
- Compile avec le cache partagé : `source client/sccache-env.sh` avant cargo (n'exporte jamais CARGO_INCREMENTAL=1).

- cargo test: never with CENT_ANS_MUTE exported (audio_playback test fails under it); mute only for running the game/smoke.
