# U1 — bogues d'interface (lots U0 et U2 de l'audit A3)

Branche : `worktree-agent-ac3b609abdc259c3f`. Source : `docs/audit/a3-ui.md` § 3 et § 8.
Captures de vérification : `docs/audit/captures/u1/`.

## Reprise
1. `core/build.sh` puis `godot --headless --path game --import`.
2. Capture : `godot --resolution 1280x720 --path game res://scenes/campaign_map.tscn -- --stage=tech --screenshot=<png>`.

## État
- [x] U0 : `.uid` non suivis commités ; `--resolution` / `-f` l'emportent sur le réglage enregistré
  (`settings.gd` : fenêtre de départ différente de celle du projet = ligne de commande).
- [ ] C1 minicarte au-dessus des panneaux
- [ ] B1 conflit AZERTY en bataille
- [ ] C12 assaut de siège + aide F1
- [ ] C2 tour 0
- [ ] E1 revenu incohérent
- [ ] M4 crédits Markdown
- [ ] erreurs brutes (agents)
- [ ] C13, B5, autres petits défauts

## Prochaine étape
C1.
