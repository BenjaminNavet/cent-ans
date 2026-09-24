# U1 — bogues d'interface (lots U0 et U2 de l'audit A3)

Branche : `worktree-agent-ac3b609abdc259c3f`. Source : `docs/audit/a3-ui.md` § 3 et § 8.
Captures de vérification : `docs/audit/captures/u1/`.

## Reprise
1. `core/build.sh` puis `godot --headless --path game --import`.
2. Captures : `godot --resolution 1280x720 --path game res://scenes/campaign_map.tscn -- --stage=tech --screenshot=<png>` ;
   les mises en scène `--flow-stage=…` prennent `--flow-shot=<png>` (et non `--screenshot`).

## État : terminé (smoke : 23 « smoke OK », sortie 0 ; cargo fmt / clippy / test verts)
- [x] U0 : `.uid` non suivis commités ; `--resolution` / `-f` l'emportent sur le réglage enregistré
  (`settings.gd` : fenêtre de départ différente de celle du projet = ligne de commande, car le
  moteur consomme ces arguments et `OS.get_cmdline_args()` ne les rend pas).
- [x] C1 minicarte sous les panneaux (`minimap_controller.gd`, `move_child` après la barre du haut)
- [x] B1 ordres du chef en touches physiques + libellés selon la disposition ; B5 recharge en coin ;
  `map_toggle_unrest` (M) en keycode (en AZERTY, M n'est pas à la place physique de M)
- [x] C12 : le bloc de siège fonctionne en jeu ; c'est la mise en scène qui s'arrêtait trop tôt
  (corrigée) ; encart décalé s'il couvre le journal ; aide F1, encyclopédie, tutoriel sans « panneau d'armée »
- [x] C2 tour affiché = tour moteur + 1 (barre, sauvegardes, rançons) ; la chronique n'efface plus le n° de tour
- [x] E1/E4 : `FactionEconomy::net_income` et `CampaignState::faction_net_last_turn` (core,
  `economy_balance.rs`), exposés `net_income`, `net_income_last_turn` ; barre et panneau affichent
  le même solde, charges signées ; E5 « Matières premières »
- [x] M4 crédits : titre unique, paragraphes recollés, tableaux en lignes, parenthèses techniques retirées
- [x] Erreur serde brute → message français (`invalid_order_message`, pont) ; détail dans le journal Godot
- [x] C13 style `focus` distinct de l'état désactivé
- [x] M1 (capture prise pendant le fondu), M3 (ligne de débogage, « Graine aléatoire »), M5 (chemin
  `data/` retiré), D2 (pluriels, « soumission obtenue », infobulle du score), C7 (« Aux mains de »),
  P4 (Épouse / Époux), T4 (nom de sauvegarde et dates en français), D3 (coquille « Son…. »)

## Non traité (refontes prévues ou hors mandat)
C3/C4/C5/C6 (U1, U6), E2/E3 (U3), U4 échelle, C8–C11, B2/B3/B4/B6/B7 (V1, U9), T1/T2/T5 (U5),
D1/D4/D5/D6, P1–P3, U1/U2 tutoriel, pluriels « (s) » restants hors objectifs.
