# 0357 — Architecture du client Bevy (migration depuis Godot)

Statut : accepté (15/10).

## Contexte
L'ADR 0347 décide la migration vers Bevy. Le client Godot compte environ 134 000 lignes de GDScript
dans 461 fichiers : `map/` 65 k, `ui/` 30 k, `battle/` 30 k, puis audio, visual et codex. Le pont
`godot-bridge` expose 366 `#[func]`. Godot reste le jeu jouable sur `main` jusqu'à la parité, et la
migration avance sur la branche `bevy-migration` avec de nombreux agents en parallèle.

## Décision
1. **`client/` à la racine, workspace Cargo séparé** (binaire `cent-ans`), avec son propre `target/`.
   Il dépend des crates de `core/` par chemin. `cargo test` de `core/` reste rapide et ne compile
   pas Bevy. Bevy est épinglé `=0.20.0`. La fonctionnalité `dev` (Bevy `dynamic_linking`) sert aux
   itérations ; le binaire joueur est lié statiquement.
2. **`core/crates/game-api`** (workspace `core/`, sans Bevy ni Godot) : vues pour l'interface, en
   types Rust `Serialize` (fiches de province, de faction, d'armée, aperçus d'ordres). Elles sont
   tirées de la logique de `godot-bridge` et testées par `cargo test`. Le client n'appelle jamais
   `godot-bridge`. Le doublon avec le pont est temporaire : le pont est retiré à la parité.
3. **Un plugin Bevy par domaine**, chacun dans son propre dossier, et chaque dossier appartient à un
   seul lot d'agent : `app` (états), `sim` (ressource simulation), `ui/kit` (boîte à outils
   parchemin), `menu`, `campaign/{camera,terrain,provinces,water,markers,armies,hud,panels}`,
   `battle/{scene,units,orders,hud}`, `audio`, `codex`, `settings`, `agent` (surface de contrôle).
   `main.rs` enregistre tous les plugins dès le squelette ; un lot ne touche ni `main.rs` ni les
   dossiers des autres lots.
4. **États** : `AppState::{Boot, MainMenu, FactionSelect, Campaign, Battle, Codex}`. Les entités
   d'un écran portent `DespawnOnExit(état)`.
5. **Données et assets réutilisés en place** : `data/` est lu par `data-model`, `game/assets/` est
   la racine d'assets de Bevy (glTF, PNG/JPG, OGG, polices EB Garamond / IM Fell / Noto). Les
   `.tscn`, `.tres` et shaders Godot sont portés en code Rust et en WGSL ; aucun asset n'est
   dupliqué.
6. **Surface de contrôle par agents** (exigence première, ADR 0347) :
   - `--state <état>` démarre directement dans un écran ;
   - `--faction` et `--seed` ;
   - `--dump-state f.json` (ressources et compteurs d'entités par plugin) ;
   - `--shot f.png --frames N` (rendu hors écran) ;
   - `--script f.txt` (commandes texte : état, clic sur une entité nommée, touche, attente) ;
   - `--exit-after N` (images) ;
   - Bevy Remote Protocol derrière la fonctionnalité `remote`.

   Chaque plugin expose ses compteurs à `--dump-state` par un trait commun.
7. **Fenêtre** : le client se lance comme le prototype, par un bundle `.app` et `open -g`
   (`client/run_bg.sh`). Le rendu hors écran sert aux captures d'agents, jamais pour voler le focus.

## Conséquences
- Les règles de jeu restent dans `core/`. Toute logique trouvée dans du GDScript à porter et qui
  est une règle remonte dans `core/` (ou dans `game-api` si c'est une vue), jamais dans `client/`.
- Avant de commiter, chaque lot exige : `cargo clippy -- -D warnings`, `cargo test` dans `client/`
  et `core/` si `game-api` change, plus une passe `--dump-state` qui montre ses entités.
- Les écrans dépendent de `ui/kit` : la première vague le construit en parallèle des lots de
  rendu.
