# Lot UX : Conseiller unique, focus clavier/manette, débordements

Branche : `worktree-agent-a247ea0e04b8d8ff4`. Sources : `out/juge/iter-02-histux.md` (partie B), `iter-02-art.md` (6, 14, 18).

## État
- [x] `client/src/ui/advisor_band.rs` : bandeau Conseiller (file à priorités, un message actif, repli, puce, historique). ADR 0752.
- [x] `hud/hint.rs` publie vers le bandeau ; `audio/advisor.rs` (sous-titres) idem ; hôte campagne dans `hud/mod.rs`, hôte bataille dans `audio/advisor.rs`.
- [x] `ui/kit/focus.rs` : anneau de focus, Tab/flèches/Entrée, manette (D-pad/A/B), F8 pour engager sur le HUD nu. ADR 0753.
- [x] Légende `(M)` : jetons `{key:…}` dans `data/ui/map_legend.json`.
- [ ] Débordement du guide du tour 1 (pied fixe + corps défilant), 8 échecs de sonde en campagne, 1 en bataille.
- [ ] Libellés du HUD de campagne (Hommes, Prestige, Désordre).
- [ ] Bouton « Retour » du choix de faction.
- [ ] Cartes d'unité de bataille compactes.
- [ ] `CENT_ANS_VIEW_SIZE=WxH` (agent) pour sonder 1920×1080.

## Prochaine étape
Mesurer `ui_kit.layout.failures` (campagne tour 0, 1280×800 / 1920×1080 / échelle max), corriger, tests, smoke.
