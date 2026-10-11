# 0342 — TRELLIS.2 local comme étape 3D du procédé d'assets

## Contexte
Les modèles DN (706 au manifeste) viennent presque tous de TRELLIS 1 sur fal (ADR 0210, 0212). Défauts
récurrents : dos inventés et sombres (18 `dark_back` sur 459 bâtiments/accessoires/engins), faces à
normale inversée (abbaye 41 %) que Godot élimine (matériaux à une face) et qui laissent voir l'intérieur.
Essai TP (11/10, `docs/wip/tp-peinture-texture.md`) : retexturage IA du dos (`retex.py --dark-only`),
correctif des faces inversées (`fix_inverted_faces.py`), et banc A/B contre TRELLIS.2 exécuté en local
et gratuitement sur le M4 Pro (portage `trellis-mac`, PyTorch MPS). TRELLIS.2 sur fal reste interdit.

Banc sur castle_empire_rhine, abbey_byz_serbia, fort_postern_gate, même image source
(planche `~/dev/cent-ans-raw/tp/trellis2/planche_trellis2.png`) : TRELLIS.2 nettement meilleur —
fidèle à l'image (vignes, enceinte, couleurs), dos cohérent sans zone sombre ni élément inventé, pas
de faces inversées visibles. 140-170 s/objet (pipeline 512, texture 1024), ~180-195 k triangles bruts.
Piste low poly procédural testée et **exclue par le joueur**.

## Décision
Adopté par le joueur le 11/10 :
1. **Étape 3D = TRELLIS.2 local** (`~/dev/cent-ans-raw/trellis-mac`, gratuit, sous `dn/gpu.lock`)
   pour les assets non figurines. Étape image inchangée (Z-Image Turbo local ; Qwen-Image-Edit pour
   les figurines). TRELLIS HF / fal TRELLIS 1 / SF3D deviennent des replis.
2. Le brut (~190 k triangles) passe par `cent-ans dn-ingest` inchangé (décimation aux budgets de
   classe, LOD, étalonnage) ; `model_3d` = `trellis2-local`, `cost_usd` = 0.
3. Le correctif des faces inversées (`fix_inverted_faces.py`, n'écrit que s'il y a gain) est
   appliqué à la sortie d'ingestion.
4. `retex.py --dark-only` n'est plus qu'un rattrapage ponctuel (il invente des détails).
5. Le parc existant (459 modèles bâtiments/accessoires/engins de siège) est régénéré, puis publié
   en paquet `models-v3` (feu vert du joueur donné le 11/10).

## Conséquences
- Coût 0 $ ; ~2,5-3 min machine par objet → ~20-23 h pour 459 objets (lots de nuit, une seule
  génération GPU à la fois).
- Disque : poids TRELLIS.2 ~15 Go déjà en cache ; bruts ~13 Go de glb + obj par 3 objets → ne garder
  que le glb brut, supprimer les `.obj`.
- Pic mémoire ~18 Go : pas de seconde charge GPU lourde en parallèle.
- Figurines animées, arbres (imposteurs), animaux et champs ne sont pas concernés par la
  régénération initiale.

## Addendum (11/10 après-midi)
- Le brut est cuit par TRELLIS.2 au budget LOD0 de la classe (`dn_ingest_classes.json`) ;
  `dn-ingest` décime ensuite vers LOD1/LOD2.
- Les images sources sont refaites en style « rendu 3D » (vue trois-quarts haute, rendu d'asset de
  jeu, lumière blanche de studio), style par défaut de `ga3_decor.json`, 2 graines. Le joueur l'a
  jugé nettement meilleur que le style photo ; c'est la distribution d'entraînement de TRELLIS.2.
- Pas de règle générale sur le budget ni sur T1 contre T2 : choix modèle par modèle, délégué à
  Claude. Les arbres restent à 6 000 triangles (12 000 essayés, écart minime).
