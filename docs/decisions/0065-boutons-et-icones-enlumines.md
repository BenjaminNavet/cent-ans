# 0065 — Boutons-médaillons enluminés et icônes d'action à l'encre (DA5)

Date : 2026-09-26. Lot DA5 de la direction artistique (bible `docs/design/2026-09-25-bible-da.md`,
§ 5 et § 8 ; écart n° 5 du § 10).

## Contexte

Les icônes d'action venaient de game-icons.net (SVG CC BY 3.0, huit auteurs, épaisseurs de trait
variées) et les boutons du bandeau étaient des cadres plats. La planche de style validée par le
joueur (`docs/img/da/planche/`) fixe deux registres : un bouton-médaillon enluminé (la cloche de
fin de saison) et des icônes au trait d'encre brune, une seule main.

## Décision

1. **Deux familles, une image par entrée**, décrites dans `data/ui/icons_ink.json` (schéma
   `data/schemas/icons_ink.schema.json`) : 78 icônes d'action (110 identifiants d'interface via
   `targets`) et 15 médaillons. La cloche reprend l'image validée (aucune génération).
2. **Génération** : `cent-ans assets ink-icons` (`tools/cent_ans_tools/ink_icons.py`), modèle
   `openai/gpt-5-image-mini`, réutilise `portraits.generate` (enveloppe du lot = 5 $ moins les lignes
   « DA5 » de `docs/budget.md`, plafond global, une ligne par passe). Chaque prompt = bloc de style
   des données + sujet, avec une **image de référence** tirée de la planche validée (couronne à
   l'encre ; médaillon de la cloche) pour garder le cadre et le trait. Sources brutes réduites
   gardées dans `tools/da5_raw/` : une image présente n'est jamais régénérée (`--only`, `--limit`,
   `--dry-run`, `--build-only`).
3. **Post-traitement par code, gratuit** :
   - icônes : alpha tiré de l'encre (écart de luminance à un fond de parchemin estimé localement
     par fermeture en niveaux de gris, ce qui efface vignettage et lavis), recadrage carré,
     **épaisseur de trait normalisée** (largeur médiane mesurée sur le squelette, décalage par
     distance signée jusqu'à 5 px à 128 px, soit ~1 px à 24 px), RVB = `HudStyle.INK` ;
     PNG 128 px dans `game/assets/icons/ink/` ;
   - médaillons : parchemin rendu transparent par remplissage depuis les bords, bord adouci, PNG
     256 px dans `game/assets/ui/medallions/`.
   Chaque dossier a un `index.json` (identifiant → fichier) et des `.import` avec mipmaps.
4. **Rendu** (`IconLibrary`) : l'index à l'encre est lu avant `icons.json` ; un identifiant couvert
   prend le PNG, sinon le SVG (repli automatique si un PNG manque). Teinte d'état : l'encre est cuite
   dans le PNG et `IconLibrary.tint(couleur)` donne la modulation (composantes > 1) qui la change en
   or au survol et à l'enfoncement, en encre pâlie désactivé (`apply_state_tints`). Médaillons :
   `decorate_medallion` (bouton existant, états par modulation) et `medallion_states` (normal,
   survol, enfoncé, désactivé dérivés d'une seule image par `Image.adjust_bcs`, utilisés par la
   cloche dessinée).
5. **Périmètre** : bandeau du haut (Diplomatie, Chronique, Cour, Techniques, Codex, Objectifs,
   Agents, Menu en médaillons ; le cadre plat ne revient qu'au survol), cloche de fin de saison
   (médaillon + banderole saison/année), Recruter et Former une armée (fiches de ville et de
   province), ordres de bataille et ordres du chef, filtres et légende de la carte, pastilles
   d'alerte, posture/ravitaillement/mouvement du sceau du général, jauges, classes, catégories.
   Les **icônes d'entité** (unités, bâtiments, techniques, compétences, ressources, régimes) restent
   des SVG : la bible les destine à des miniatures peintes (lot ultérieur).

## Conséquences

- Coût réel consigné dans `docs/budget.md` (section Direction artistique, lignes « DA5 »).
- Une nouvelle icône = une entrée du catalogue puis `cent-ans assets ink-icons --only <id>` ; une
  image ratée se refait en supprimant sa source brute.
- Les SVG game-icons.net restent dans le dépôt tant que les icônes d'entité les utilisent
  (`CREDITS.md` inchangé).
- Modulation > 1 : dépend du fait que le canevas 2D ne borne pas `modulate` avant le produit avec
  la texture (vrai en Godot 4) ; à défaut, l'icône resterait à l'encre au survol, sans autre effet.
