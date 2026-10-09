# Concept artist — état des lieux (09/10, lecture seule, 2 images regardées)

## 1. État actuel
- **Charte** : bible DA (`docs/design/2026-09-25-bible-da.md`) à deux registres — enluminure pour la 2D (§ 5, § 13), réalisme peint pour la 3D (§ 3.3, § 6, § 14 ; S ≤ 0,40 ; prompt figurines § 14.8) ; ADR 0211 ; arbitrages D1-D5 (`docs/wip/dn-direction-artistique.md`).
- **Ancre 2D** `data/art/style/anchor.jpg` + `anchor.yaml` ; **prompt de l'ancre non commité** (`tools/nb_raw/da/prompt.txt` absent).
- **Références 3D** : planche SR3 (`docs/research/sr3_references.jpg`), rendus GA3, `~/dev/cent-ans-raw/charte/`.
- **Procédé** : `docs/pipeline-assets-3d.md`, `tools/experiments/dn_batch.py` (contrôle de charte auto, `charter.jsonl`).
- **Prompts** : `data/art/ga3_decor.json` + 12 catalogues `dn_catalog_*.json` (~1 040 entrées) ; blocs `STYLE` 2D dans `portraits.py`, `event_art.py`, `entry_art.py`, `art_plates.py`.
- **Production 2D** : 608 portraits, 153 événements, 536 illustrations ; 148 miniatures locales ; 308 illustrations DN.

## 2. Forces
- Charte mesurable (HSV, budgets de tris, contrôle avant 3D).
- Prompts tirés des données, provenance tracée.
- Gratuit d'abord, meilleur-de-N local, budget suivi.
- Revue outillée (planches-contacts, `fix_prompt`).

## 3. Faiblesses
1. **Plusieurs « mains » en enluminure** (gpt-5-image-mini, NB2, Z-Image local, fal) : `cdx_aides.jpg` plat/enluminé vs `cdx_abu_al_hasan_ali.jpg` semi-photo avec cadre doré interdit. Z-Image ne lit pas l'ancre.
2. Erreurs héraldiques non contrôlées (lys et croix sur la bannière mérinide) ; aucun contrôle d'époque en 2D.
3. Bloc `STYLE` dupliqué dans 4 fichiers ; vêtements de portrait figés sur 1330-1360 (contre le pilier 2).
4. Pas de catalogue de figurines (`data/art/ga3_figure.json` absent) ; suffixe « Rural medieval France circa 1340 ».
5. **Aucun livrable de concept** : ni color script, ni keyframes, ni moodboard, ni key art ; pas de silhouettes de factions non françaises, de planches biome/saison, de concepts de siège.
6. Sujets jamais conceptés : 36 événements, 149 fiches Codex (dont 28 lieux), curseurs, lettrines, icônes d'édits ; évolution 1340→1440 ; nuit/hiver/pluie/incendie ; multi-vues de 22 cités ; `archer_3_jack` non cuit.
7. A-pose stricte non obtenue en local ; LoRA multi-angles Qwen non éprouvée.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Unifier le registre 2D : `style_blocks.py` (période paramétrée), un générateur par registre, Qwen-Edit avec l'ancre, régénérer les plus dérivantes | Fort | M | 0 (NB2 ~0,04 $/img si besoin) | outils, UI |
| 2 | Contrôle héraldique/époque : vrais blasons en post-traitement, interdits positifs, relecture VLM ou planche | Fort | M | 0 | historien |
| 3 | Combler les trous 2D (36 événements, 28 lieux, 87 personnages liés, icônes) | Moyen-fort | S-M | 0 | UI |
| 4 | `data/art/ga3_figure.json` : planches par faction et époque, A-pose éprouvée | Fort | M | 0,02 $/fig. | tech art, animation |
| 5 | Color script + ~8 keyframes cibles d'étalonnage | Moyen | M | 0 | éclairage |
| 6 | Versionner les prompts maîtres, ancre 3D « faisant foi », README `charte/` | Durable | S | 0 | — |
| 7 | Catalogues en formulations positives, `target_albedo_mean` réellement lu | Moyen | S | 0 | outils |
| 8 | Multi-vues des 22 cités/châteaux | Moyen | S | ~1,5 $ | budget |
| 9 | Planches props de siège et vie quotidienne par saison | Moyen | M | 0 | level |
