# Cheval « Rigged Horse » (lot FG0)

- **Source** : https://opengameart.org/content/rigged-horse — fichier
  https://opengameart.org/sites/default/files/riggedHorse.blend (20 Mo, textures 2k empaquetées).
- **Auteur** : Lyndon Daniels (déposé par ChadM, 20/05/2012).
- **Licence** : **CC0 1.0** (champ « License(s) » de la page OpenGameArt, vérifié le 26/09/2026).
- **Contenu utilisé** : corps (7 390 triangles), crinière (3 920), queue (1 756), yeux ; textures
  couleur, normale et occlusion 2k du pelage, couleur et normale 2k des crins. Le squelette
  d'origine (19 os) n'est pas utilisé.
- **Hors dépôt** : le `.blend` n'est pas versionné (`.gitignore`) ; `battle_fine_horse.py` le
  télécharge s'il manque. Dossier ignoré par Godot (`.gdignore`).
- **Modifications** : mise à l'échelle, puis ajustement articulation par articulation sur le
  squelette du cheval Quaternius (rig `cavalry`, lot FG4 ; FG0 : déformation RBF), poids par
  chaleur des os dans la pose naturelle, paupières rouvertes, décimation en trois niveaux,
  montures amincies (roncin, genet), ombrage du pelage reporté par sommet (textures non
  livrées) ; harnachement modelé par script (`tools/blender_scripts/battle_fine_cavalry.py`).
  Dérivés : `game/assets/models/battle_fine/` (`cavalry_*`, `standard_1`).
- **Licence du dérivé** : CC0 1.0.
