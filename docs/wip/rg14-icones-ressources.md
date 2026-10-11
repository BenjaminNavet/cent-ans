# RG14 icônes de ressources (local)

## État : FAIT
- 10 ressources + hud_treasury, 3 graines Z-Image Turbo (768 px), rembg, choix dans ~/dev/cent-ans-raw/icons-rg14/choices.json, planche contact.jpg.
- Résolution unique : `IconLibrary` (entity/index.json > ink > SVG). PNG 128 px dans `game/assets/icons/entity/`, 64 px dans `game/assets/icons/resources/<id>_64.png` (non utilisés par le jeu : une seule taille résolue, Godot réduit).
- Script : `tools/experiments/resource_icons.py`.
- Piège : `cent-ans assets entity-icons` régénérerait les res_* encadrés (catalogue data/ui/entity_icons.json) et écraserait ces PNG.
