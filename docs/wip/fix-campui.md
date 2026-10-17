# Lot campui — mise en page des panneaux de campagne (post-playtest 17/10)

Branche `fix/campui`. Sources : `docs/wip/playtest-2026-10-17/ui-campagne.md`, `ux.md`.

## État
- [x] 7. Bouton de fermeture : `close_button` / `CLOSE_CAPTION` (« Fermer ») dans `ui/kit/widgets.rs`, utilisé par tous les panneaux de campagne.
- [x] 6. Lieu de l'armée : nom affiché de la ville (`ArmyPanelData::location_name`).
- [~] 1. Recrutement : cartes à hauteur mini, « Bientôt » en liste défilante, reste non écrasable.
- [~] 2. Économie : onglet à hauteur réductible (`scroll_list_fill`).
- [~] 3. Traditions : liste d'options défilante, page réductible (`shrinkable`).
- [ ] 4. Taille d'interface « Très grande ».
- [ ] 5. Sélection de faction à 1280x800 + Échap.

## Prochaine étape
Vérifier par les sondes `ui_kit.layout` (scripts du bac à sable `panels.txt`, `panelsbig.txt`).
