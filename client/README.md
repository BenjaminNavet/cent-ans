# Client Bevy de Cent Ans

Client Bevy 0.20 (ADR 0357), workspace Cargo séparé de `core/`. Il ne contient que rendu, interface et entrées : toute règle de jeu vit dans `core/`, les vues pour l'interface dans `core/crates/game-api`. Les assets (`game/assets/`) et les données (`data/`) sont lus en place.

## Compiler et lancer
- Itérations : `cargo build --features dev` (liaison dynamique de Bevy). Binaire joueur : `cargo build --release`.
- Lancer sans voler le focus : `client/run_bg.sh [options]` (bundle `.app` « CentAnsBevy », `open -g -n -W`, journal sur la sortie standard, code 1 si `AGENT_FATAL` ou panique). `CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug` pour le build de développement.
- Avant de commiter : `cargo fmt && cargo clippy --features dev -- -D warnings && cargo test`, et `cd core && cargo test -p game-api` si `game-api` change.
- Racine du dépôt : variable `CENT_ANS_ROOT`, sinon recherche de `data/` + `game/assets/` depuis le dossier courant puis l'exécutable.

## Surface de contrôle (plugin `agent`)
| Option | Effet |
|---|---|
| `--state <État>` | écran de départ : `MainMenu`, `FactionSelect`, `Campaign`, `Battle`, `Codex` |
| `--faction <id>` / `--seed <n>` | faction du joueur (`fac_france`) et graine (1337) |
| `--dump-state f.json` | écrit le JSON des sections de dump après `--frames` images |
| `--shot f.png` | capture hors écran (implique `--offscreen`) |
| `--frames <n>` | images à attendre avant `--shot` / `--dump-state` (60) |
| `--exit-after <n>` | quitte après n images quoi qu'il arrive |
| `--script f.txt` | commandes ci-dessous |
| `--offscreen` | rend dans une image (`OffscreenTarget`) au lieu de la fenêtre |

Fonctionnalité `remote` : Bevy Remote Protocol en HTTP local.

Script, une commande par ligne (`#` = commentaire) : `state <État>`, `wait <images>`, `key <KeyCode>` (`Escape`, `Enter`, `KeyA`, `Digit1`, `ArrowUp`, `F1`…), `click <Nom>` (nœud d'interface dont le `Name` est ce texte, espaces permis ; attend jusqu'à 300 images qu'il existe), `dump <fichier>`, `shot <fichier>`, `exit`. Exemple :

```
wait 30
click Nouvelle partie
wait 60
dump out/v0.json
exit
```
`client/run_bg.sh --state MainMenu --script s.txt` : les chemins relatifs sont résolus depuis le dossier courant.

## Ajouter une section au dump
```rust
use crate::agent::dump::DumpAppExt;
fn dump_armies(world: &World) -> (String, serde_json::Value) {
    ("armies".into(), serde_json::json!({ "count": world.resource::<Armies>().len() }))
}
// dans Plugin::build :
app.register_dump(dump_armies);
```
Sections intégrées : `state`, `entities`, `frame`, `sim` (résumé de la faction du joueur via `game-api`).

## Plugins et propriété
Un dossier = un plugin = un seul lot d'agent : `app`, `sim`, `agent`, `ui/kit`, `menu`, `audio`, `codex`, `settings`, `campaign/{camera,terrain,provinces,water,markers,armies,hud,panels}`, `battle/{scene,units,orders,hud}`. Un lot ne modifie ni `main.rs` (tous les plugins y sont déjà enregistrés) ni les dossiers des autres lots. L'en-tête de chaque `mod.rs` nomme les scripts Godot (`game/scripts/`) qu'il remplace. Les boutons passent par `ui::kit::button` et le message `ButtonPressed` (souris ou agent) ; les caméras d'un écran rendent dans `OffscreenTarget` quand elle existe.

## Campagne : caméra et terrain (lot `campaign/camera`, `campaign/terrain`)
- **Caméra** (`campaign/camera`, port de `campaign_camera.gd`) : ZQSD/WASD et flèches, Q/E (lacet), molette ou pincement (zoom), glisser molette (déplacement avec inertie), Alt/Maj + glisser molette (rotation), bords d'écran (fenêtre active seulement ; ScrollLock l'active ou le coupe). Tangage dérivé de la distance (30° à 70°, plus rasant sous 22 unités), bornes de la carte, caméra jamais sous le relief. Ressenti dans `data/ui/camera_feel.json`, profil dans `game/resources/close_camera.tres`. Elle rend dans `OffscreenTarget` quand elle existe (le lot passe `UiCamera` en `ClearColorConfig::None` pendant la campagne pour que l'interface se dessine par-dessus le monde).
- **Placer la caméra** (scripts d'agents) : `CENT_ANS_CAMERA="lon,lat,distance[,lacet_deg]"` (longitude/latitude réelles, projection de la carte composée) ou `CENT_ANS_CAMERA="xz:x,z,distance[,lacet_deg]"` (unités carte ; 1 unité = 1 pixel de carte). `open -g` (run_bg.sh) ne transmet pas l'environnement : écrire la même valeur dans `out/camera.txt` (racine du dépôt). La distance est celle de la caméra au point visé, en unités carte ; elle ne descend pas sous 22 (20 avec les maquettes de ville, `camera_floor_distance`).
- **Terrain** (`campaign/terrain`) : pyramide de relief `data/map/pyramid` (E1-E3) + `data/map/height` (E0), sélection CDLOD par la crate `relief-lod`, pages décodées en fils natifs (256 pages en mémoire, LRU), maillages de nœuds échantillonnés sur CPU (altitude en mètres et champs du relief exagéré dans les attributs ; l'exagération dynamique se calcule dans `terrain.wesl`), colormap BC1 (`map.json` `colormap.bc1`), soleil `CampaignSun`. Les autres lots lisent la hauteur affichée du sol par le paramètre système `terrain::surface::GroundHeight::at(x, z)`.
- **Dump** : `campaign_camera` (position, foyer, distance, lacet, tangage…) et `campaign_terrain` (pages résidentes par étage, nœuds sélectionnés / maillés, triangles, échelle verticale, `settled`).
