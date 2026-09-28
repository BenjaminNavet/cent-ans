# CB — icônes des contrôles de bataille (pipeline DA5)

Branche `feat/cb-icons` (depuis main 6d581bb2). Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § Icônes.

## État
- Catalogue `data/ui/icons_ink.json` : 16 entrées nouvelles (groupe `cb`) + cibles ajoutées à 6 dessins
  existants (réemploi gratuit) ; `order_pavise` retiré (le dessin `pavise` sert à `battle_ability_pavise`) ;
  section `cursors` (6 curseurs dérivés, schéma à jour).
- Clés : `battle_mode_<mode>`, `battle_state_<pastille>`, `battle_alert_<kind>`, `battle_ability_<kind>`,
  `battle_lock` ; curseurs `game/assets/ui/cursors/<contexte>.png`.
- `ink_icons.build_cursors` / `process_cursor` (dérivation gratuite 32 px) + tests pytest.
- Sonde payante 2 icônes (cb_lock, cb_under_fire) : 0,09 $, style validé. Lot complet (14) lancé.
- Branchement Godot : `BattleModeIcons.draw_ink_icon` (repli glyphe), modes, pastilles, alertes,
  capacités, cadenas ; test `game/tests/cb_icons_test.gd`.

## Prochaine étape
Fin du lot payant (commande : `uv run --project tools cent-ans assets ink-icons --kind icon --group cb
--subject "CB : icônes des contrôles de bataille à l'encre" --budget-cap 3`, les bruts existants ne sont
pas regénérés), puis déplacer la ligne de grand livre ajoutée en fin de `docs/budget.md` (section PO)
vers la section Direction artistique, `godot --import`, tests Godot.
