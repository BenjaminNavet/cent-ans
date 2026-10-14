# MN — Meshy en maillage seul, texture gratuite (banc 14/10)

**État : banc fait, en attente du choix du joueur.** 10 crédits (solde 270 → 260).

## Idée
`meshy-6-lite` sans texture = 5 crédits au lieu de 15 (≈ 210 objets/mois au lieu de 70). La texture
est refaite gratuitement en local.

## Banc
- Objets : `castle_byz` (enceinte à cour, église au centre) et `siege_ram_roofed` (engin compact).
- Soumission : `tools/experiments/meshy_notex_bench.py submit|fetch` → `~/dev/cent-ans-raw/meshy/<id>/notex.glb`.
- Texturage : `blender -b -P tools/blender_scripts/mn_texture_bake.py -- <raw_dir> <source.png> <t1.glb> <detail.png> <out_dir>`
  → `<id>/mn/{proj,detail,transfer}.glb` + rendus. Planches : `~/dev/cent-ans-raw/meshy/sheets/mn_<id>.jpg`.
- Méthodes :
  - `proj` : projection orthographique de l'image source ; angle (lacet + plongée) trouvé par IoU de
    silhouette (castle 0,91, bélier 0,77) ;
  - `detail` : `proj` × grain micro en projection boîte (pierre/bois) ;
  - `transfer` : UV du modèle T1 livré recopiées sur le maillage Meshy (Data Transfer, face la plus
    proche), texture T1 recuite.
- Hunyuan3D-2.1 Paint : le Space HF n'accepte pas de maillage fourni (`/generation_all` refait la
  forme) → écarté ; en local il faut CUDA.

## Résultat
- **Bélier (objet compact, vu de face)** : `proj` ≈ Meshy texturé (bardeaux, poteaux, roues) ;
  `detail` n'apporte rien ; `transfer` plus terne. → maillage seul + `proj` viable.
- **Château (cour, objets cachés)** : `proj` étire le sol et la cour, église mal placée ;
  `transfer` cohérent mais sombre (texture T1). Meshy texturé nettement meilleur.
- Dos sombre sur toutes les lignes, y compris la référence : éclairage du rendu, pas la texture.

## Proposition
Maillage seul + `proj` pour les objets compacts à une face dominante (engins, charrettes, ponts,
portes, tours isolées, bêtes de trait) ; Meshy texturé pour tout ce qui a une cour ou des parties
cachées (châteaux, villes, abbayes, villages) et pour les figurines.

## Prochaine étape (si go)
- Option `--notex` dans `meshy_waves.py` (classe d'objets → maillage seul) et passage de `mn_texture_bake.py`
  dans la chaîne `dn-ingest`.
- Améliorer `proj` : masquer les faces non vues (test de profondeur) et les remplir par diffusion
  plutôt que par la couleur de l'avant.
