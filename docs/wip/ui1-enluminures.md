# WIP — UI1 : habillage « manuscrit enluminé » de l'interface

Demande (2026-09-25) : l'UI n'est qu'une bande sépia et des boutons plats ; la rendre plus
belle, avec des assets « moines », enluminures. Décision : ADR 0050.

## État : terminé (dans main)
- [x] Générateur `tools/cent_ans_tools/ui_illumination.py` + commande `cent-ans assets ui-illumination`
- [x] 15 textures 9-slice dans `game/assets/ui/illumination/` (+ `kit.json`)
- [x] Thème : `StyleBoxTexture` partout, variations `IlluminatedPanel` et `TopBarPanel`
- [x] Bandeau de campagne, fenêtres (province, cour, fiche, chronique, faction, techniques)
- [x] Infobulles, bulles épinglées, conseiller (note marginale), carte de conseil, tutoriel, agents
- [x] Bataille : bandeau bas, confirmation, avant-bataille, bilan ; HUD naval
- [x] Correctif polices Noto (lignes 36 → 24 px) ; correctif boucle `diplomacy_panel`
- [x] Captures `docs/img/ui1/` ; smoke : pas de régression (échecs restants préexistants :
      niveaux de difficulté, playlists musicales)

## Pistes (non faites)
- Lettrines enluminées pour les titres de fenêtres (IM Fell, première lettre sur champ d'azur).
- Encore ~25 `StyleBoxFlat` locaux (lignes de sauvegarde, pastilles, cartes d'unité) : volontairement
  plats (couleurs porteuses de sens) ou à reprendre au cas par cas.
- Captures : `godot --path game --script res://tests/ui1_capture.gd -- --out=<dossier>`.
