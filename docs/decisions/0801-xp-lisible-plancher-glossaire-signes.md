# 0801 — XP-LISIBLE : plancher typographique, glossaire testé, signes à l'encre

Statut : accepté (boucle juge 2, lot XP-LISIBLE). Amende 0065 (icônes à l'encre partout) et 0753 (raccourcis).

## Contexte
L'audit d'expérience du 29/10 relève trois défauts de lisibilité : des textes de 10 à 12 px dans le
HUD de bataille (M5), trois langages d'icônes dont une centaine de glyphes Unicode rendus par une
police de repli (M6), un vocabulaire qui se contredit (« mécontentement » / « agitation »,
« localité » / « colonie », M7). Les réglages d'accessibilité manquaient (sous-titres, délai des
bulles, police lisible, taille du texte à chaud).

## Décisions
1. **Plancher de 13 px** : `style::MIN_TEXT_PX`. `UiTheme::text_font` et `title_font` bornent la taille
   finale (échelle `text_size` comprise) ; les littéraux sous 13 sont corrigés et le test source
   `client/tests/text_floor.rs` refuse d'en réintroduire.
2. **Réglages** : `subtitles` (vrai par défaut ; coupe le texte des voix du conseiller et le bandeau
   des discours de bataille, pas le cri), `tooltip_delay` (0,2 à 1,5 s, défaut 0,5), `readable_font`
   (Atkinson Hyperlegible Next, OFL, pour le texte courant ; les titres IM Fell restent), `text_size`
   appliqué à chaud : `apply_text_scale` réécrit la taille et la police des `TextFont` du thème déjà à
   l'écran, sans reconstruire les panneaux.
3. **Glossaire** `data/ui/glossary.json` (schéma `ui_glossary.schema.json`) : terme canonique, formes
   interdites, portée (champs de texte de fichiers de données, littéraux Rust). Test
   `tools/tests/test_ui_glossary.py`. Les textes d'histoire (« colonie génoise ») restent hors portée.
4. **Signes** : nouveaux glyphes à l'encre dessinés au trait par `tools/make_ink_glyphs.py` (aucun
   coût, même traitement `process_icon` que la famille) : `livre`, `hausse`, `baisse`, `oui`, `non`,
   `fleche_droite`, `fleche_gauche` ; alias `epees`, `etoile`, `croix` vers les `glyph_*` existants.
   Composant `InkGlyph` : un nœud porte un id d'icône que le kit résout (builders sans `AssetServer`).
   Migré : case à cocher, liste d'objectifs. Le reste des glyphes de widgets migre au fil des lots ;
   `₶` reste en texte dans les montants (police Noto Serif Currency).
5. **Raccourcis** : K (`codex_open`) lu dans le keymap ; nouvelles actions `map_toggle_recruitment`
   (Maj+R), `map_toggle_faction` (P), `map_toggle_feudal` (Maj+F) ; `map_toggle_agents` (G),
   `map_toggle_holdings` (B) et `encyclopedia_open` (L), jamais câblés, sont retirés de F1 et du keymap.

## Conséquences
- Un terme interdit dans un texte de portée fait échouer pytest.
- Reste à migrer : glyphes `▲▼✓✗★⚔` des tooltips et du HUD de bataille (lots suivants).
