# bm-launcher — lanceurs Bevy (ADR 0362)

État : fait et testé (`--dry-run`, pytest `tools/tests/test_launch_update.py` 11 verts). Pas de
lancement fenêtré, pas de push.

Reste : valider sur de vrais Windows et Linux (compilation, bibliothèques), premier run CI du job
`bevy`; empaqueter plus tard un binaire précompilé (éviter 10-30 min de compilation au premier
lancement); supprimer `--godot` avec `game/`.
