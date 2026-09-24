# D0 — acquisition d'assets libres (top 15 de l'audit A4)

Source : `docs/audit/a4-assets-libres.md`. Assets rangés sous `game/assets/third_party/<catégorie>/<nom>/`,
chacun avec un `SOURCE.md` (source, URL, licence, auteur). Crédits ajoutés à `CREDITS.md`.
**Non branchés dans le jeu** (lots suivants). Pas de git LFS (le dépôt ne l'utilise pas).

## État

- [x] Polices EB Garamond, IM Fell English
- [x] Kenney Fantasy UI Borders, Castle Kit
- [x] Poly Haven HDRI (Belfast Open Field, Autumn Field Pure Sky)
- [ ] Poly Haven arbres (Pine Tree 01, Fir Tree 01), herbe (Grass Medium 01/02) -> GLB
  (re-téléchargement en cours après crash machine ; scripts dans `tools/blender_scripts/polyhaven_vegetation.*`)
- [x] Parchment GUI (OpenGameArt)
- [x] Quaternius : 4 personnages riggés (King, Adventurer, Hooded Adventurer, Farmer) + Horse, White Horse, Donkey
- [ ] Quaternius Medieval Village MegaKit : **échec** (itch.io seulement, Google Drive des autres packs en quota dépassé)
- [ ] SFX Freesound (Church Bell, Swords Clash) : **sautés** (téléchargement de l'original = connexion requise ; Chrome indisponible)
- [x] Musiques incompetech (Lord of the Land, Village Consort)
- [ ] Crédits `CREDITS.md`
- [ ] Import Godot vérifié

## Prochaine étape

Si interrompu : relancer le téléchargement Poly Haven (`api.polyhaven.com/files/<asset>`, blend
2k pour l'herbe, 1k pour les arbres) dans le scratchpad puis
`bash tools/blender_scripts/polyhaven_vegetation.sh <work_dir>`. Ensuite CREDITS.md, import Godot.

## Détail par asset

Voir la fin du fichier une fois terminé (tableau complet).
