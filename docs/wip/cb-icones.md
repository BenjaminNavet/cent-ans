# CB — icônes des contrôles de bataille (pipeline DA5)

Branche `feat/cb-icons` (depuis main 6d581bb2). Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § Icônes.

## État
- Catalogue `data/ui/icons_ink.json` : 16 entrées nouvelles (groupe `cb`) + cibles ajoutées à 6 dessins
  existants (réemploi gratuit) ; `order_pavise` retiré (le dessin `pavise` sert à `battle_ability_pavise`) ;
  section `cursors` (6 curseurs dérivés, schéma à jour).
- Clés : `battle_mode_<mode>`, `battle_state_<pastille>`, `battle_alert_<kind>`, `battle_ability_<kind>`,
  `battle_lock` ; curseurs `game/assets/ui/cursors/<contexte>.png`.

## Prochaine étape
Dérivation des curseurs dans `ink_icons.build`, dry-run, sonde 2 icônes, lot complet, branchement Godot.
