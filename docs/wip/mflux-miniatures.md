# Images locales gratuites mflux (ADR 0190)

## État
- Modèle `local/z-image-turbo` routé dans `openrouter.request_image` (tests `tools/tests/test_local_art.py`).
- `cent-ans assets illustrations --local` : lot des 148 miniatures de factions en cours (nouveau
  prompt : paysage de la province capitale, scène variée). Blasons inventés, cadre doré fin.

## Lots en cours (10-08)
- A (mech) : FAIT (10-08). `--local` (mflux gratuit, aucun appel réseau ni ligne de budget) sur
  `assets` : illustrations 16:9 ; event-art 16:9 ; codex-art 16:9 ; portraits 3:4 ;
  portrait-archetypes 3:4 ; art-plates 16:9 (loading, ending) / 21:9 (vignette) ;
  entity-icons 1:1 ; ink-icons 1:1 (icônes et médaillons) ; horizon-panoramas 21:9 ;
  materials 1:1 (sans garde de section du grand livre) ; ui-ornaments (aspect du YAML, sans
  `image_size` Gemini, ancre vide ignorée) ; ground-materials generate 1:1 (au lieu de fal.ai).
  Écartées : menu-art et ui-illumination (procéduraux, aucun appel d'image). Tests :
  `tools/tests/test_local_cli.py` ; fixture `fake_mflux` dans `tests/conftest.py`.
- B (dev) : chaîne figurines GA3 (`tools/experiments/ga3_fal_*.py`) : planche de référence
  locale (Z-Image img2img) + détourage local (rembg) ; seul TRELLIS reste payant (0,02 $).

## Ensuite
- Rogner le cadre doré des miniatures de factions, contrôle visuel d'un échantillon, commit.
- Incrustation des vrais blasons (données `heraldry`).
- Essais réels de A et B quand le lot de factions est fini (GPU libre).

## Lot B : chaîne GA3 locale (10-08, code fait, aucun essai réel mflux)
- `tools/experiments/ga3_local.py` (commun) : `local_sheet_prompt` (prompt nano-banana -> description
  pour Z-Image : 3 vues face/profil gauche/dos, A-pose, mains vides, fond blanc, couleurs clés),
  `render_local` (cache : un fichier existant n'est jamais refait), `cut_local` (rembg
  `isnet-general-use`, poids ~179 Mo dans `~/.rembg/models`, vérifié sur image synthétique).
- `ga3_fal_figure.py` : `--sheet-backend {fal,local}`, `--cut-backend {fal,local}`, `--strength`
  (défaut 0.45, img2img depuis la planche SR3, 3:2). `ga3_fal_decor.py` : `--image-backend`,
  `--cut-backend` (flux-2 et flux-2/edit -> Z-Image ; vues latérales en img2img à 0.55 depuis `src.png`).
- `local_art.render_image/command_for` : paramètre `strength` optionnel (rétrocompatible).
- Reste payant : TRELLIS seul (0,02 $). Aucun coût local dans costs.json.
- Essai réel à lancer GPU libre (une unité) :
  `uv run --with rembg --with onnxruntime --with fal-client --with pillow --with numpy python
  tools/experiments/ga3_fal_figure.py ~/dev/cent-ans-raw/ga3/local --unit longbowman
  --sheet-backend local --cut-backend local` (ajouter `--model multi` par défaut ; pour juger la
  planche seule, interrompre avant TRELLIS ou ouvrir `sheet.png`/`sheet_cut.png`). Risque : la force
  0.45 peut ne pas suffire à imposer la A-pose 3 vues ; ajuster `--strength` (0.3-0.6) et `--attempt`.
