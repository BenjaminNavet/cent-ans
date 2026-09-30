# 0135 — Ancre de style et redessin guidé du kit d'interface

Date : 2026-09-30. Statut : acceptée.
Spec : `docs/superpowers/specs/2026-09-30-nb-nano-banana-interface-design.md`.
Complète l'ADR 0050 (kit d'interface enluminée en 9 tranches).

## Contexte

La bible DA ne définissait le registre enluminure que par du texte (bloc `STYLE`). Nano Banana 2
(`google/gemini-3.1-flash-image`, OpenRouter) accepte jusqu'à 14 images de référence et suit bien
une référence de style. Le kit d'interface (ADR 0050) est procédural : exact en 9 tranches, mais
d'un dessin pauvre. Un générateur d'images ne sait pas produire de lui-même des marges et des
bandes répétables exactes.

## Décision

1. **Ancre de style** : une planche maîtresse générée par NB2 à partir de folios du domaine public
   (Grandes Chroniques, Très Riches Heures, Pucelle, Froissart), choisie par le joueur (v0), dans
   `data/art/style/anchor.jpg`. Toute génération du registre enluminure la joint en image 1
   (bible § 13). Jamais pour le registre 3D.
2. **Modèle** : NB2 + ancre par défaut ; Lite + ancre pour les icônes en nombre ; Pro écarté
   (pas meilleur, deux fois plus cher). Sonde : `docs/research/nb0-sonde-modeles.md`.
3. **Redessin guidé** plutôt que panneaux entiers ou ornements assemblés : NB2 reçoit l'ancre et
   la texture procédurale agrandie ×6 sur fond vert comme guide de forme ; la chaîne locale
   (`ui_ornaments.py`) détoure, ramène à la géométrie de `kit.json` et rend les bandes centrales
   répétables (mode `TILE_FIT` du thème). `kit.json` et `parchment_theme.tres` ne changent pas.
4. **Périmètre** : seuls les cadres à marges larges sont redessinés (`panel_illuminated` v1,
   `panel` v2, `top_bar` v2, `tooltip` v2) ; boutons, onglets, barres, curseur et encart
   (bordures de 1 à 9 px) restent procéduraux, un dessin peint y serait invisible.
5. **Repli** : `ui_illumination.build` prend `nb/<id>.png` s'il existe, sinon le tracé
   procédural (sortie identique à l'avant-NB, testée). `nb/` porte un `.gdignore`.

## Conséquences

- Coût NB : 3,10 $ (planche 0,40 $, sonde 1,44 $, kit et décors 1,26 $).
- Les images brutes restent hors dépôt (`tools/nb_raw/`, ignoré) ; une repasse locale est gratuite.
- Le détail peint est limité par la taille 1× du kit (≤ 204 px) ; une interface 2× (HiDPI) est
  un chantier ultérieur.
- Chantiers ouverts par cette décision : parchemin peint de la carte stratégique, planches de
  référence des figurines (piste C), reprise des portraits et événements en NB2 + ancre.
