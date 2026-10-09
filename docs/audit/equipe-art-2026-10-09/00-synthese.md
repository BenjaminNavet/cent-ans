# Équipe art — synthèse de l'état des lieux (09/10, lecture seule)

Ce dossier rassemble 23 rapports, un par rôle d'un studio, rédigés chacun par un agent qui a lu le code, les données et la documentation. Aucun fichier du jeu n'a été modifié. Les agents ont regardé très peu d'images et n'ont rien écouté : les jugements de rendu sont à confirmer en jeu.

| # | Rôle | # | Rôle |
|---|---|---|---|
| 01 | [Directeur artistique](01-directeur-artistique.md) | 13 | [Rigger](13-rigger.md) |
| 02 | [Art producer](02-art-producer.md) | 14 | [Animateur](14-animation.md) |
| 03 | [Concept art](03-concept-art.md) | 15 | [Mocap](15-mocap.md) |
| 04 | [Illustration](04-illustration.md) | 16 | [Cinématiques](16-cinematiques.md) |
| 05 | [Character](05-character.md) | 17 | [VFX](17-vfx.md) |
| 06 | [Environnement](06-environnement.md) | 18 | [UI/UX](18-ui-ux.md) |
| 07 | [Props / hard surface](07-props-hard-surface.md) | 19 | [Motion design](19-motion-design.md) |
| 08 | [Textures / matériaux](08-textures-materiaux.md) | 20 | [Technical art](20-technical-art.md) |
| 09 | [Outsourcing](09-outsourcing.md) | 21 | [Pipeline / outils](21-pipeline-outils.md) |
| 10 | [Level art](10-level-art.md) | 22 | [Son](22-son.md) |
| 11 | [Monde / terrain](11-monde-terrain.md) | 23 | [Marketing](23-marketing.md) |
| 12 | [Éclairage](12-eclairage.md) | | |

## 1. Constat d'ensemble
Le projet produit beaucoup et à bas coût. Ses chaînes sont reproductibles et ses données sont sourcées : relief réel, licences tracées, figurines, et 667 modèles et 308 illustrations produits en une nuit (DN). La faiblesse ne tient pas à la quantité. Elle tient à **ce qui reste entre la production et le joueur** : assets produits mais pas branchés, vérifications visuelles et d'écoute jamais faites, règles du joueur contredites dans le code, budget et documents de suivi décrochés.

## 2. Thèmes transversaux
1. **Assets DN produits mais pas branchés en bataille** (06, 07, 01)
   - Le kit de siège (courtines, tours, portes, gravats, bombardes) n'est pas utilisé : les murs restent des boîtes procédurales.
   - La bataille n'a que 6 essences d'arbres et une seule architecture, contre 40 essences et 21 sous-familles en campagne.
   - Branches non fusionnées : `dn/champs`, `dn/fig-bake`, `as8c`, `feat/faction-minis` (02).
2. **Le paquet DN (1,1 Go, environ 42 $) n'a ni hébergement ni sauvegarde** (02, 09, ADR 0212). Il y a aussi 1 390 textures en triple, une par LOD (20).
3. **Vérifications jamais faites**
   - AS7, l'animation vue en jeu (14, 15, 13).
   - RC3, HB5 et HC3 sur le terrain (11).
   - Feux de camp, bivouac et crue LR-18 (17).
   - Aucune écoute du son (22).
   - Les outils de mesure ont été supprimés par SC et aucun banc ne détecte les régressions de performance (20).
4. **Compression des textures**
   - 2 177 des 2 199 textures de modèles ne sont pas compressées (environ 5,3 Mo de VRAM au lieu de 0,7 Mo).
   - Les normales sont en BC1.
   - L'eau n'a pas de mipmaps.
   - Sources : 08 et 20.
5. **Règles du joueur contredites dans le code et la doc** (09)
   - TRELLIS 2 est autorisé dans `pipeline-assets-3d.md` et proposé par `dn_batch.py --backend3d fal2` ; 9 $ y ont été dépensés.
   - Le repli local est actif par défaut.
   - Le meilleur-de-N est documenté sur un modèle payant.
6. **Budget incohérent** (02, 09)
   - L'en-tête dit « 50 $ v1 » alors que le total atteint environ 117 $.
   - La nuit DN a coûté environ 42 $ pour une enveloppe de 10 $.
   - Les plafonds se contredisent : 29,50 $ dans un endroit, 43,5 $ dans un autre.
   - `budget.py` ne lit que la dernière table.
   - Le solde fal et le quota gratuit HF sont épuisés.
7. **Héraldique et époque**
   - Blasons inventés : `fac_thomond`, `chr_otto_le_doux`, bannière mérinide (03, 04).
   - Unités mal attribuées : archers montés et akinci armés d'une lance, yaya habillés en francs-archers, teutoniques en chevaliers français (05).
8. **Plusieurs « mains » dans le registre 2D**
   - Les images enluminées viennent de quatre générateurs différents ; les archétypes dérivent vers la peinture semi-réaliste ; formats et cadres sont mélangés (01, 03, 04).
   - Le bloc `STYLE` est dupliqué dans 4 fichiers.
9. **Écarts entre campagne et bataille** : essences, architecture, lumière (contrat commun absent, 12) et sol de bataille olive et peu lisible (10, 12).
10. **Accessibilité partielle** (18, 19)
    - Le mode daltonien n'est branché que dans 4 fichiers et absent en bataille.
    - Les infobulles opposent bon et mauvais par le seul vert/rouge.
    - Le texte est trop petit en 720p.
11. **Documents de suivi périmés**
    - `status.md` date du 24/09 et `roadmap.md` s'arrête à V6 ; il y a 175 notes wip sans index (02).
    - Le paquet de relief v3 n'est pas commité, ce qui expose un autre poste au téléchargement de v2 (11).
    - `CREDITS.md` ne mentionne pas les assets générés par IA (09, 22).
12. **Vitrine absente** (23) : pas d'icône d'application, pas de Release GitHub, pas d'image Open Graph, pas de key art.
13. **Décisions du joueur en attente** : forêts DN, ADR 0212, `ga`, lot B mflux (02).

## 3. Gains rapides (S, gratuits, à fort effet)
| # | Action | Rapport |
|---|---|---|
| 1 | Commiter le paquet de relief v3 et finir RA (Release, ADR, `docs/geo.md`) | 11 |
| 2 | Sauvegarder les 1,1 Go de glb DN (miroir local daté) | 02 |
| 3 | Inscrire les règles du joueur dans le code : bloquer `fal2`, `--no-local-fallback` par défaut, pas de multi-graines payant ; corriger la doc | 09 |
| 4 | Réconcilier le budget : plafond global, en-tête, `budget.py` | 02, 09 |
| 5 | Forcer la compression VRAM des textures de modèles, mipmaps sur l'eau, contrôle CI (mesurer la VRAM avant) | 08, 20 |
| 6 | Brancher `bow_walk`/`xbow_walk` ; refaire `c_walk` | 14, 13 |
| 7 | Corriger les attributions `figure` (archers montés, akinci, yaya) | 05 |
| 8 | Fumée brun-rouge près des feux | 17 |
| 9 | Terrain et effet sous le curseur en bataille | 10 |
| 10 | Plancher de texte en 720p ; daltonien en bataille | 18 |
| 11 | Icône de l'application + image Open Graph | 23 |
| 12 | Section IA dans `CREDITS.md` / `LICENSE-ASSETS.md` | 09 |
| 13 | Session AS7 (une check-list, le joueur regarde) | 14, 15 |

## 4. Chantiers de fond (M-L), dans l'ordre proposé
1. **Brancher le DN en bataille** : kit de siège, essences selon le biome, architecture par sous-famille. Fusionner DN-forêts après un banc. (06, 07)
2. **Contrat de lumière commun et lisibilité du sol de bataille** : contraste, boue, parcelles. (12, 10, 01)
3. **Coût du terrain** : variante lointaine du shader, MultiMesh découpés par tuile, banc de régression rétabli. (20, 11)
4. **Unifier le registre 2D** : blocs de style communs, vrais blasons, 100 portraits vieillis, 64 fiches nature. (03, 04)
5. **Figurines** : seigneur, GA3 sur les 9 figurines restantes, civils élargis. (05)
6. **Animation** : tournage AS6, pas du cheval, pivots, charge. (14, 15)
7. **Son** : couches de 30-60 s et stingers, pistes primaires sans MIDI. (22)
8. **Vitrine** : première Release v0.1, logo unifié, README, puis key art et bande-annonce. (23)

## 5. Dépenses éventuelles (toutes à décider par le joueur)
- Recharge fal d'environ 2-3 $ : multi-vues des 22-25 maquettes au dos sombre, glacier (02, 03, 06).
- GA3 pour les 9 figurines restantes : environ 1,5-2 $ (05).
- SS4, détail proche de campagne : environ 1-2 $ (08).
- HF PRO à 9 $/mois, au lieu de recharger fal (09).
- Hors budget cloud : voix refaites (5-50 €), musique originale (2-10 k€), key art par un illustrateur (500-2 000 €).
