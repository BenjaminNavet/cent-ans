# RG13-V1 — salle du conseil en 3D multivue (fal)

Comparaison avec les modèles actuels (TRELLIS HF / SF3D depuis une seule vue de face). Sorties :
`game/assets/models/council/multiview/council_<objet>.glb` (+ `.glb.import`). Brutes, vues, planche :
`~/dev/cent-ans-raw/council/multiview/` (scripts `views.py`, `trellis.py`, `ingest.py`, `planche.py`).

## État : TERMINÉ (9/9)
Chaîne : image de face existante (graine du catalogue) → `fal-ai/flux-2/edit` (dos ¾ + profil ; table et
tabouret : ¾ haute) → rembg → `fal-ai/trellis/multi` (stochastic) → `dn-ingest --lods 1`.
Lacet : figures 180 (constable aussi : TRELLIS multi l'oriente comme les autres), table 0, trône 90, tabouret 180.
Coût réel 0,61 $ (plafond 2 $), voir `docs/budget.md`.

## Défauts par objet (rendu Blender face + dos, planche.jpg)
- Souverain, chancelier, connétable, trésorier, chapelain, chambellan : vues de dos cohérentes (dos non noir, siège
  sous la figure). Facettes visibles sur les dos (maillage 12 k triangles). Chambellan : manteau de dos un peu
  différent de la face (l'édition a inventé la fourrure du dos).
- Chapelain et chambellan : l'image de face a un banc, le dos généré montre un tabouret ; le 3D garde un tabouret.
- Table : plateau long et mince ; mise à 3,2 x 1,1 x 0,85 m par échelle non uniforme (x3,2 / x2,2 / x1,5 sur les
  axes bruts), jambes un peu épaisses.
- Trône : correct, dossier de dos sobre.
- Tabouret : premier essai raté (3 pieds fondus en un seul, 2 vues quasi identiques) ; refait avec une vue de profil
  de plus (3 images) : pieds évasés corrects.
- Les textures extraites par l'import Godot (`*_Image_*.jpg`) ne sont pas versionnées (comme les glb existants).
