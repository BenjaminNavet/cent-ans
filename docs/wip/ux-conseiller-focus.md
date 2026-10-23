# Lot UX : Conseiller unique, focus clavier/manette, débordements

Branche : `worktree-agent-a247ea0e04b8d8ff4`. Sources : `out/juge/iter-02-histux.md` (partie B), `iter-02-art.md` (6, 14, 18). ADR 0752 (Conseiller), 0753 (focus).

## Fait (compile, `cargo check` propre)
- `client/src/ui/advisor_band.rs` : bandeau unique (file à priorités, un message actif, repli, puce, historique dans le journal). Branché sur `hud/hint.rs`, sous-titres de `audio/advisor.rs`, hôtes campagne et bataille. Le guide s'efface devant lui (`yield_band`).
- `client/src/ui/kit/focus.rs` : anneau de focus, Tab/flèches/Entrée, D-pad + A/B, F8 sur HUD nu.
- Guide du tour 1 : corps défilant (`scroll_body`) + pied fixe. Paddings des cadres HUD corrigés, sonde ignore les fenêtres flottantes, barre du haut hors sonde.
- Légende `{key:…}` (data/ui/map_legend.json), libellés de chiffres (data/ui/hud_labels.json, seulement si barre >= 1500 px logiques), « Retour » lisible (encre claire sur plaque cuir), cartes de bataille compactes (x0,7, pleine taille à la sélection).
- `CENT_ANS_VIEW_SIZE=WxH` (agent).
- Mesure `ui_kit.layout.failures` campagne tour 0 avec guide visible : avant 8 (audit) / 7 (mesuré sans guide) ; après 0 à 1280x800, 1920x1080 et 1280x800 échelle 1,25. 1920x1080 à échelle max non mesuré.

## Non testé / à faire
- Tests unitaires `advisor_band` (corrigés après échec, non relancés), clippy -D warnings, tests complets, smoke : à lancer une fois au merge.
- Focus non vérifié en run (script prêt : scratchpad `uxfocus.txt`, sur MainMenu).
- Bataille : sonde de mise en page et compacité des cartes non mesurées.
- Liste des 14 étapes du guide pas déplacée dans l'aide F1 ; libellés du HUD absents sous 1500 px ; captures non faites.
