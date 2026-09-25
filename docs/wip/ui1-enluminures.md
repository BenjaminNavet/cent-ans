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

- [x] Lettrines des titres de fenêtres : `Lettrine.attach(label)` sur province, cour, faction,
      techniques, fiche de personnage ; test `tests/ui1_lettrine_test.gd`
- [x] `core/build.sh` : supprime la dylib avant de la copier (sinon signature macOS invalidée →
      Godot tué, code 137)

## Pistes (non faites)
- Lettrine pour la chronique (titre centré et à retour à la ligne : non géré par `Lettrine`).
- Encore ~25 `StyleBoxFlat` locaux (lignes de sauvegarde, pastilles, cartes d'unité) : volontairement
  plats (couleurs porteuses de sens) ou à reprendre au cas par cas.
- Captures : `godot --path game --script res://tests/ui1_capture.gd -- --out=<dossier>`.
