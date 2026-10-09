# QW-G — bataille : terrain sous le curseur, daltonisme, ▲/▼, plancher de texte

État : G1-G4 codés ; test `game/tests/qw_g_test.gd`.

- G1 : `Battlefield::decor_hover_at` (cœur, `decor.rs`, test `decor_hover_reports_rule_values`) ; `hover_context()["decor"]` (pont) ; `BattleTerrainTip` (étiquette « Verger — couvert ▲ +x %, vitesse ▼ −y % »), posée par `battle_scene.gd` quand le curseur n'est pas sur une troupe.
- G2 : `BattleAccess` (Okabe-Ito bleu/vermillon + hachures des plaques ennemies, losanges sur la minicarte, décales ennemies) branché sur `Accessibility.colorblind()`.
- G3 : `RichTooltip.mark(sign)` (▲/▼), appliqué aux effets, forces/faiblesses, monnaie, valeur d'en-tête et encyclopédie.
- G4 : `Caption` 14 → 15 (thème + `UiType`), `po_ui_test` MIN_SIZE 15.
- Les nouvelles classes sont référencées par `preload` (cache de classes globales non régénéré sans `--import`).
