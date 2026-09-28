# 0109 — Infobulles en sections, chaîne de bulles à Alt maintenu

Date : 2026-09-28. Statut : accepté. Spec : `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`.

## Contexte

Comparées à celles de Baldur's Gate 3, nos infobulles (`RichTooltip`, F2) ont un contenu riche mais
une mise en page plate : un seul `RichTextLabel` de 330 px en 14 px, lignes enchaînées par « · »,
pas de chiffre vedette ni de sections. La chaîne de bulles imbriquées (`CodexBubbles`, H2 + B1)
existe mais demande un appui sur T par niveau, ne peut pas partir de l'infobulle native (qui
disparaît au mouvement de la souris), et ses bulles filles ne montrent qu'un résumé du Codex.
T ouvre aussi l'arbre des techniques.

## Décision

1. **Modèle structuré.** `RichTooltip` produit une spec (dictionnaire : en-tête, vedettes, effets,
   stats, conditions, ambiance, pied) ; `TooltipView` la rend en sections séparées par des filets.
   Le BBCode reste un repli (`RichTooltip.to_bbcode`). Styles et textes dans `data/ui/`
   (`tooltip_style.json`, `tooltips.json`, avec schémas).
2. **Deux niveaux de détail, sans touche dédiée.** Survol : version courte (≤ 8 lignes de corps).
   Bulle verrouillée : version complète. Maj est pris en bataille (file d'ordres), Ctrl par les
   groupes.
3. **Chaîne à Alt maintenu** (action `tooltip_explore`, Option sur macOS). Alt fige l'infobulle
   native en bulle verrouillée à la même place ; tant qu'Alt est tenu, survoler un mot-clé ouvre sa
   fille en 0,12 s, déjà verrouillée, et changer de mot remplace la branche. Au relâchement, la chaîne
   reste tant que la souris est sur une bulle. T garde son rôle actuel (bascule, B1).
4. **Bulles filles riches.** Liens `ib:<kind>:<id>` à côté de `cdx:` : une entité montre sa spec
   complète ; une règle (effet, stat, jauge, posture) montre son texte de `tooltips.json`.
5. **Placement latéral**, fil d'Ariane dès le 3ᵉ niveau, 10 bulles au plus, les anciennes se
   réduisent à leur en-tête au lieu de se fermer.

Présentation pure : aucune règle de jeu dans `game/` ; les valeurs « avant → après » viennent du
core par les dictionnaires `live`.

## Conséquences

- Tous les constructeurs de `RichTooltip` et les ~120 `tooltip_text` littéraux migrent (lot IB2).
- `EFFECT_LABELS`, `STAT_LABELS`, `GAUGE_TEXTS`, `HUD_TEXTS` quittent le GDScript pour `data/`.
- Alt seul devient réservé à l'exploration des infobulles sur la carte et en bataille ; Alt+Maj
  reste aux formations.
- Tests : `ib_layout_test.gd`, `ib_chain_test.gd`.
