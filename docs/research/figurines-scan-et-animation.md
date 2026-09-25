# Piste — figurines scannées et animations réalistes

Idée du joueur (2026-09-25), **à aborder plus tard**. Ce document ne lance aucun lot : il conserve
l'analyse pour qu'une session future puisse la reprendre. Elle complète le chantier FG
(`docs/wip/fg-figurines-fines.md`), en particulier FG2 (équipement fin) et FG3 (matières cuites).

## L'idée d'origine

1. Partir de photos d'une pièce réelle (une armure, par exemple).
2. Reconstruire une scène 3D par Gaussian Splatting (Kerbl, Kopanas, Leimkühler, Drettakis,
   Inria GraphDeco, SIGGRAPH 2023).
3. Compléter les parties manquantes par IA générative.
4. Remailler avec les méthodes de Pierre Alliez (Inria, CGAL) à une précision choisie, pour Godot.
5. Générer automatiquement des animations réalistes (marcher, combattre, fêter la victoire).

## Analyse de faisabilité

| Étape | Faisable ? | Difficultés |
|---|---|---|
| 1. Photos | Oui, mais il faut 50 à 200 vues sous tous les angles (poses calculées par COLMAP) ; une seule photo ne suffit pas | Métal poli : photographier sous lumière diffuse |
| 2. Gaussian Splatting | Oui | Le résultat n'est pas un maillage (gaussiennes sans topologie ni UV) : conversion nécessaire (SuGaR, 2DGS, Gaussian Opacity Fields). Les reflets du métal sont « cuits » dans la géométrie. **Licence** : le code Inria original est non commercial ; utiliser `gsplat` (nerfstudio, Apache 2.0) |
| 3. Trous par IA | Oui (vues manquantes générées par Zero123++, SV3D, puis nouvelle reconstruction) | Cohérence entre vues fragile ; l'IA invente du plausible, pas du 1340 : validation historienne obligatoire |
| 4. Remaillage | Oui, avec une correction : Godot consomme des triangles, pas un diagramme de Voronoï. Le Voronoï centroïdal (CVT) place N sommets et sa triangulation de Delaunay duale donne le maillage ; N est la « précision choisie » | Un maillage uniforme gaspille des triangles sur les zones plates : préférer l'adaptatif (VSA d'Alliez, décimation Blender, Instant Meshes). CGAL est GPL mais utilisé hors ligne : les maillages produits ne sont pas concernés |
| Étapes manquantes | — | Dépliage UV et cuisson haute → basse définition (normales, ORM) ; retrait de l'éclairage d'origine ; **rigging** : découper l'armure par pièce, plates rigides par os, maille skinnée, sur le rig `human` Quaternius existant (mêmes os = tous les clips réutilisés) |
| 5. Animations | Oui, sujet à part entière | Voir plus bas |

Contrainte structurante : le budget par figurine (LOD0 ≈ 8-12 k, LOD1 ≈ 2,4 k, LOD2 ≈ 500
triangles, cible FG1). Au-delà de 32 m, un scan millimétrique n'apporte rien ; sa valeur est
comme **source haute définition à cuire en carte de normales**, et pour les gros plans (Codex,
portraits).

## Voie A — pragmatique (pour FG2, plus tard)

Contourne le Gaussian Splatting :

1. Source : scans de musée ouverts (Met Open Access en CC0, Wallace Collection, scans CC0 sur
   Sketchfab), sinon génération image → 3D (Hunyuan3D, TRELLIS, Tripo ; Hunyuan3D et Tripo sont
   accessibles via le MCP Blender) à partir de photos de pièces d'époque.
2. Décimation adaptative et remaillage dans Blender.
3. Dépliage UV, cuisson normales + ORM + masque de livrée sur la figurine légère (FG3).
4. Découpe par os et ajustement au rig `human`.
5. Validation historienne (datation de la pièce ≈ 1337-1453) puis A/B de performance (FG5).

Essai proposé le moment venu : un lot « FG2-proto » sur une seule pièce (un bassinet), passée par
les deux sources (scan de musée et image → 3D), rendus comparés à l'actuel. Coût : ≈ 0 $ en CC0,
quelques dollars si génération (à consigner dans `docs/budget.md`, section DA).

La voie Gaussian Splatting complète (étapes 1 à 4 d'origine) reste réservée à des pièces
photographiées soi-même (Musée de l'Armée par exemple) et aux gros plans.

## Voie B — animations réalistes (sujet séparé, plus tard)

Toutes les sources finissent adaptées (retargeting) au rig `human` puis cuites dans la texture
d'os existante (animations par rig, ADR 0014).

| Source | Adaptée à | Remarques |
|---|---|---|
| Capture de mouvement existante (Mixamo, base CMU) | Marche, course, gestes | Mixamo gratuit en jeu mais pas CC0 ; CMU libre |
| Mouvement extrait de vidéo (WHAM, GVHMR) | Combat, escrime médiévale | Filmer un reconstituteur en armure : la voie la plus réaliste pour le combat |
| Texte → mouvement (MDM, MotionGPT) | Célébrations, attentes | Faible sur le combat |
| Contrôle physique (DeepMimic, AMP) | Chutes, impacts | Coûteux, plutôt recherche |

Points à trancher le moment venu : liste des clips par type d'unité, poids de la texture d'os,
variation par soldat (décalage, vitesse) pour éviter l'effet « ballet synchronisé ».

## Prérequis avant de reprendre

- FG0 (planche de style) validé, FG1 (nouveau corps) fusionné : le rig et le budget sont figés.
- DA1 fusionné (même shader que FG3).
