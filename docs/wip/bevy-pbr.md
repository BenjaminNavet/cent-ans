# Lot pbr : fonctionnalités PBR et post-effets Bevy 0.20 optionnels

Branche : `worktree-agent-af54920576edee64b` (base remise à jour sur main par fast-forward : le worktree
démarrait avant la création de `client/`).

## État (22/10)
- Fait, `cargo fmt` + `cargo clippy --features dev --all-targets -- -D warnings` passent :
  - `client/Cargo.toml` : features `pbr_anisotropy_texture`, `pbr_multi_layer_material_textures`,
    `pbr_specular_textures`, `pbr_transmission_textures`, `experimental_pbr_pcss` (bloc commenté « pbr lot »).
  - `data/fx/postfx.json` + `data/schemas/postfx.schema.json` (+ entrée dans `tools/tests/test_schemas.py`),
    clé optionnelle `postfx` (liste d'ids) dans les préréglages de `render_quality.json` (schéma mis à jour).
    Aucun préréglage n'active quoi que ce soit.
  - `client/src/settings/postfx.rs` : `PostFxFlags` (réglage `on`/`off` sinon préréglage), `PostFxParams`,
    ajout/retrait de DepthOfField, MotionBlur, AutoExposure, ChromaticAberration, ContrastAdaptiveSharpening
    sur les caméras de bataille et de campagne, ColorGrading sur la caméra de campagne seulement (celui de la
    bataille appartient à `BattleGrade`), PCSS (`soft_shadow_size`) sur les soleils, synchro HDR de la caméra
    UI, section de dump `postfx`. 7 réglages `fx_*` dans `Settings` et dans l'onglet Affichage.
- Tests écrits (postfx.rs) mais NON exécutés : disque plein (708 Mo libres), `cargo test` a échoué en
  « No space left on device » à la compilation du profil de test.

## Prochaine étape
1. Libérer du disque, puis `cd client && source sccache-env.sh && cargo test --features dev` (corriger
   d'éventuels tests de la fenêtre Réglages qui comptent les lignes de l'onglet Affichage).
2. `uv run --project tools pytest tools/tests/test_schemas.py`.
3. Dump `postfx` en `run_bg.sh --offscreen` avec `CENT_ANS_MUTE=1`.
4. Mesure au banc bataille (effet par effet) ; n'ajouter un effet à la liste `postfx` d'un préréglage
   (ultra seulement) que s'il tient le budget.
