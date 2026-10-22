# Lot misc — features Bevy 0.20 diverses

État : TERMINÉ. clippy `--features dev --all-targets -D warnings` propre, `cargo fmt`, tests : 1404 OK + 1 échec préexistant (`campaign::markers::dn::tests::real_table_and_glbs` : glb absent du worktree, sans lien avec le lot), nouveau test `settings_is_reflected_for_brp` OK. Pas de lancement BRP en direct (disque à 99 %).

## Tableau de décisions

| Candidat | Décision | Raison |
|---|---|---|
| `reflect_auto_register` + `serialize` | Adopté, derrière `dev` (feature `reflect-inspect`) | `Settings` dérive `Reflect` (`#[reflect(Resource, Default)]`), s'auto-enregistre ; lisible/modifiable via BRP (`brp.sh world.get_resources`). Aucun coût joueur : absent du binaire release. |
| `bevy_settings` | Rejeté (conclusion écrite) | Notre `settings/` fait déjà : JSON dans le dossier utilisateur, redirection `CENT_ANS_SETTINGS_PATH`, migrations des clés Godot (`migrate.rs`), `KeyMap` (diff avec défauts), annuler/restaurer, valeurs par défaut, application en direct. `bevy_settings` ne couvre qu'une partie (persistance de ressources reflect) ; le remplacer ferait perdre migrations et annulation sans gain. |
| `ghost_nodes` | Rejeté | Aucun conteneur UI existant n'existe seulement pour grouper ; pas de `Display::Contents` dans le code, la feature ajouterait un coût de propagation de layout sans simplification. |
| `bevy_feathers` | Rejeté (évalué, aucun écran migré) | Notre kit `ui/kit` (boutons -> message `ButtonPressed`, thème, tooltips BBCode, modales, dumps agent, noms de widgets scriptables) est déjà le standard du jeu ; feathers a un style propre incompatible avec le thème médiéval et ne donne pas les noms/dumps utilisés par les tests et scripts. |
| `bevy_camera_controller` / `pan_camera` / `free_camera` | Rejeté | Nos caméras campagne/bataille dépendent de `KeyMap`, du bord d'écran, de la rotation/zoom bornés par le relief, des modes cinématique/`KeepCamera` ; les contrôleurs Bevy sont génériques (libre/pan) sans ces contraintes. Pas de remplacement. |

## Refactors `#[require]` / `Single<>`
- `WaypointLabel` (`battle/orders/marks.rs`) : `#[require(Node, Visibility::Hidden, Pickable::IGNORE)]`, spawn allégé.
- `Single<(&Camera,&GlobalTransform), With<BattleCamera>>` : `marks.rs` (labels de waypoints), `formations.rs` (`draw_ghosts`).
- `Single<Entity, With<BattleHudRoot>>` : `hud/journal.rs`, `cursor_tips.rs`, `orders_bar.rs`, `info.rs`, `formation_menu.rs`.
- `Populated<>` : non appliqué. Les gardes `is_empty()` du code sont des inversions (« spawn si absent ») ou portent sur des `Vec`, pas sur des requêtes à early-return ; aucun cas simple et sûr.

## Reste
Vérification live optionnelle : `client/brp.sh world.get_resources` avec le type `cent_ans_client::settings::model::Settings` sur un client `dev` offscreen.
