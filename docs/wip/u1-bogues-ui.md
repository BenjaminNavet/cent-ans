# U1 — bogues d'interface (lots U0 et U2 de l'audit A3)

Branche : `worktree-agent-ac3b609abdc259c3f`. Source : `docs/audit/a3-ui.md` § 3 et § 8.
Captures de vérification : `docs/audit/captures/u1/`.

## Reprise
1. `core/build.sh` puis `godot --headless --path game --import`.
2. Capture : `godot --resolution 1280x720 --path game res://scenes/campaign_map.tscn -- --stage=tech --screenshot=<png>`.

## État
- [x] U0 : `.uid` non suivis commités ; `--resolution` / `-f` l'emportent sur le réglage enregistré
  (`settings.gd` : fenêtre de départ différente de celle du projet = ligne de commande).
- [x] C1 minicarte sous les panneaux (`minimap_controller.gd`, `move_child` après la barre du haut)
- [x] B1 ordres du chef en touches physiques + libellés selon la disposition ; B5 recharge en coin ;
  `map_toggle_unrest` (M) en keycode (en AZERTY, M n'est pas à la place physique de M)
- [x] C12 : le bloc de siège fonctionne en jeu ; c'est la mise en scène qui s'arrêtait trop tôt
  (corrigée) ; encart décalé s'il couvre le journal ; aide F1, encyclopédie, tutoriel sans « panneau d'armée »
- [x] C2 tour affiché = tour moteur + 1 (barre, sauvegardes, rançons)
- [x] E1 : `FactionEconomy::net_income` et `CampaignState::faction_net_last_turn` (core,
  `economy_balance.rs`), exposés `net_income`, `net_income_last_turn` ; barre et panneau affichent
  le même solde, charges signées
- [ ] M4 crédits Markdown
- [ ] erreurs brutes (agents)
- [ ] C13, B5, autres petits défauts

## Prochaine étape
M4 crédits, erreurs brutes (agents), C13.
