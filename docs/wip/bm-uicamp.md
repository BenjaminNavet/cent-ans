# bm-uicamp : remise en ordre du HUD de campagne (client Bevy)

Branche `bm/uicamp`. Dossiers : `client/src/campaign/hud/`, `client/src/ui/kit/` (styles, gabarits), gabarit des panneaux `client/src/campaign/panels/*/`.

## But
Structure de Godot (barre du haut compacte à icônes, conseiller en haut à gauche, pastilles d'alerte + sceau rond « Printemps 1337 » + minicarte en bas à droite), plus de rangées de boutons texte, plus de chevauchements.

## État
- [x] `ui/kit/layout.rs` : constantes (barre 40 px, marges), `FitZone` (Side / Bottom / Full), `HudReserve` publié par le HUD.
- [x] Barre du haut (`hud/bar.rs`) : écu + nom, chiffres à icône, date, lanceur rond (médaillons, touche en coin), menu (= touche pause).
- [x] Bas à droite (`hud/seal.rs`) : pastilles à icône + compteur, sceau rond « Printemps 1337 », minicarte en coin avec les 6 modes de carte en icônes + légende, bascule relief sur la minicarte.
- [x] Colonne gauche : conseil, missions (s'efface devant la carte du tutoriel).
- [x] Panneaux sur `FitZone` (province, recrutement, économie, faction : Side ; féodal : SideTall ; armée : Bottom ; diplomatie, cour : Full ; siège : Left). Avis centrés sous la barre.

- [x] Barre compacte sous 1380 px logiques (4:3, taille d'interface > 115 %) : date et « / saison » masquées.
- Vérifié (hors écran 1280×800, UiScale 0,9) : province, armée, féodal ouverts → `panel_overlaps` vide ; fumée `tests/smoke/campaign.txt` passe.

## Écarts à Godot (améliorations)
- Lanceur en icônes seules (nom + touche en infobulle, lettre en coin) au lieu de libellés selon la place : la barre ne passe jamais à la ligne.
- Panneaux placés par zone (`FitZone`) autour du groupe bas-droite au lieu de le décaler.
- Le conseil s'efface pendant le tutoriel.

## Restes
- `progress/tutorial.rs` (carte en dur à left 12 / top 120), `progress/traditions.rs`, `armies/roster.rs` (top 72, right 12) : pas sur `FitZone` (hors dossier).
- « Menu » passe par l'arête Échap : un panneau ouvert se ferme d'abord.
- Journal d'événements (Godot `EventLog` en haut à gauche) non porté.
- Résolutions : `UiScale` auto ramène tout à ~900 px logiques de haut ; vérifié en 1280×800 seulement (taille hors écran fixe dans `agent/`).
