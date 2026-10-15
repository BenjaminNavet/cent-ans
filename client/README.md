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

## Boîte à outils UI
Dossier `src/ui/kit/` (plugin `UiKitPlugin`, ADR 0357 § 4). Remplace `hud_style`, `front_end_style`, `ui_type`, `rich_*`, `tooltip_*`, `panel_section`, `confirm_*`, `list_menu`, `icon_chip`, `money`, `fr_text`, `modal_queue`, `ui_motion`, `lettrine`, `illuminated_title` de `game/scripts/ui/`. Tout écran se construit avec lui ; il lit `UiTheme` (ressource) et ne touche jamais `Interaction`.

| Module | Contenu |
|---|---|
| `mod.rs` | `UiTheme` (couleurs, polices EB Garamond + repli Noto pour `₶ ▲ ✓`, IM Fell pour les titres, textures 9-slice de `assets/ui/illumination/` avec les marges de `kit.json`), `panel`, `title`, `label`, `typed(variation)`, `ButtonPressed`, `UiCamera`, section de dump `ui_kit` |
| `style.rs` | palette (`INK`, `RUBRIC`, `GOLD`, jauges…), échelle `UiType` (Title 26, Heading 20, Body 17, Caption 15), étages `UiTier` (`GlobalZIndex` : HUD < panneaux < bandeaux < modales < tutoriel < avis < infobulles) |
| `widgets.rs` | `button(theme, nom)`, `button_captioned`, `button_small`, `button_primary`, `button_icon`, `toggle_button`, `checkbox`, `tab_bar` + `tab_pane`, `slider`, `gauge`, `scroll_list` + `list_item`, `icon_chip`, `separator`, `section`, `heading`, `caption`, `error_line`, `panel_page` / `panel_sized` / `panel_illuminated` / `panel_note` / `panel_inset` / `card`, `lettrine`, `illuminated_title`, `set_disabled` |
| `richtext.rs` | `rich_text(theme, assets, bbcode, &RichStyle)` : `[b] [i] [u] [s] [color=#hex] [font_size=N] [url=cible]…[/url] [img=LxH]chemin[/img] [center] [right] [p] [lb] [rb]` → `Text` + `TextSpan` (+ `InlineImage`) ; `parse` pur ; les liens portent `RichLink` + `Hovered` |
| `tooltip.rs` | `Tooltip { bbcode }` sur n'importe quel nœud ; `TooltipRegistry` (clé de lien → BBCode) ; `TooltipState::open_at / open_child_of` |
| `modal.rs` | `ModalQueue` (file à priorités, `request(clé, priorité, ModalSpec)`), message `ModalResult`, `ToastRequest`, `SlideIn`, `ReduceMotion` |
| `icons.rs` | `icon(theme, assets, "nom", taille)` / `icon_tinted` (port d'`IconLibrary.make_rect`) : miniature d'entité (`assets/icons/entity`) > icône à l'encre (`assets/icons/ink`, teintable par `ink_tint`) > médaillon (`ui/medallions`) ; `UiTheme.icons` (`IconLibrary`) construit au démarrage. Les SVG (game-icons) ne se chargent pas dans Bevy : leurs identifiants sans PNG (`svg_only`) donnent une boîte transparente de la bonne taille. Dump `ui_kit.icons` : tous les noms par famille, icônes à l'écran, non résolues |
| `tooltip_spec.rs` | infobulle structurée (`tooltip_view.gd`, `rich_tooltip.gd`) : `TooltipSpec` (titre, sous-titre, icône, vedettes, effets ▲▼•, stats, forces/faiblesses, conditions, détail, ambiance, pied), `blocks(détaillée)`, `to_bbcode` pour `Tooltip` / `TooltipRegistry`, `tooltip_view` (cadre, bandeau d'en-tête, filets entre blocs) ; textes de `data/ui/tooltips.json` (`TooltipTexts`, `register_rule_texts` enregistre les bulles `ib:rule:<clé>`, `fill_rules` résout `{rule.…}` à la demande de l'appelant), style de `tooltip_style.json` (`TooltipStyle`) |
| `panel_stack.rs` | `panel_stack.gd` : composant `StackedPanel { tier, id, companion_of }` sur la racine d'un panneau (`display: none` = fermé), ressource `PanelStack` (état pur testé), messages `PanelStackRequest` (`Open`, `Close`, `Toggle`, `CloseTop`, `Reveal`, `ForgetSuspended`), `PanelClosed { id }` (le propriétaire met son état à jour), `PanelStackChanged`. Docked mis de côté par un central et rendus à la fermeture, central exclusif, compagnon lié à ses hôtes, modal garde Échap ; Échap ferme le panneau du dessus (hors modale du kit) |
| `layout_probe.rs` | dump `ui_kit.layout` : pour chaque cadre (page, enluminée, note, encart, carte, liste, infobulle), rect, marge requise (`max(content, margins)`), distance réelle aux enfants et `ok` ; `failures` doit valoir 0 ; barres de défilement (piste, pouce, contenu/visible) |
| `fr.rs` | `digits`, `amount`, `signed`, `charge`, `delta`, `color_of` (monnaie `₶`, espace insécable, vrai signe moins), `plural_s`, `count`, `date`, `date_from_iso` |

**Interaction.** Souris, clavier et agent (`click <Nom>`) aboutissent au même message `ButtonPressed { entity, name }` ; un bouton `Name::new(nom)` se clique par son nom. Les widgets à état l'écoutent et émettent leur message : `ToggleChanged` (bascule, case), `TabSelected` (onglet ; les `tab_pane` du même groupe suivent), `ListSelected` (une sélection par groupe), `SliderChanged` (curseur, `is_final` faux pendant le glissement). Un widget désactivé porte `InteractionDisabled` (`set_disabled`). Boutons : 4 états (normal, survol, appuyé, désactivé) = textures `button_*` / `button_primary_*`, onglets = `tab_selected` / `tab_unselected`.

**Infobulles (ADR 0109).** `Tooltip` : survol 0,5 s → bulle de 330 px près du curseur, recadrée à l'écran. **Alt** maintenu fige la bulle ; tant qu'il l'est, survoler un `[url=clé]` d'une bulle ouvre sa fille en 0,12 s (verrouillée, sur le côté ; changer de mot remplace la branche) ; au relâchement la chaîne reste tant que la souris est sur une bulle ; Échap ferme tout ; 10 bulles au plus, les plus anciennes se réduisent à leur première ligne. Les clés se résolvent par `TooltipRegistry::register("ib:rule:morale", "[b]Moral[/b]…")` ; le contenu (specs) reste aux écrans.

**Modales et avis.**
```rust
queue.request("saison", PRIORITY_REPORT, ModalSpec::info("Rapport", "Le [b]printemps[/b] s'achève."));
queue.request("taxe", PRIORITY_DECISION, ModalSpec::confirm("Décision", "Lever une taxe ?")); // passe avant
// puis lire MessageReader<ModalResult> { key, confirmed } ; Échap = renoncer
toasts.write(ToastRequest::new("Partie sauvegardée"));
```

**Galerie.** `CENT_ANS_KIT_GALLERY=1 … run_bg.sh --state MainMenu` affiche tous les widgets ; `CENT_ANS_KIT_GALLERY_DEMO=1` ouvre en plus une chaîne d'infobulles, deux modales en file et deux avis. Section de dump `ui_kit` : widgets par genre, noms des bascules actives / onglets choisis / valeurs de curseurs, infobulles ouvertes (niveaux, verrou), file des modales, avis.

**Cadres et marges.** Tout cadre texturé a un remplissage `UiTheme::inner_padding(texture, air)` = `max(content, margins)` de `kit.json` + air (panneau 8 px, page enluminée 6, note/encart 4), et une hauteur minimale de 40 px (carte, note, panneau). Bevy réduit la bordure peinte d'un 9-slice quand le nœud est plus petit que la texture (`taille nœud / taille texture`) : au chargement, les textures à marges positives sont donc rognées en 9-slice compact (marges + centre de 16 px pris au milieu, centre étiré, `SliceMetrics.compact`) pour que la bordure garde sa taille sur une note de 34 px de haut. La lettrine a sa propre boîte (largeur fixe, sans rétrécissement) et 6 px avant la suite du titre.

**Liste défilante.** `scroll_list` reçoit une barre fine parchemin (piste de 8 px collée à droite, `IgnoreScroll`, pouce glissable : widget `Scrollbar` de Bevy) en plus de la molette ; le remplissage droit lui réserve la place.

Écarts : Bevy UI n'a pas d'opacité par nœud (pas de fondu de 0,12 s, seulement le glissement de 8 px) ; pas de SVG (icônes game-icons sans PNG absentes) ; la lettrine n'a ni initiale réelle (`FaUi`) ni rinceau.
