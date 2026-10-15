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

## Prochaine étape
Vérifier panneaux ouverts (province, armée) et fumée campagne ; README.
