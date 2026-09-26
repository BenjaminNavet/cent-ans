# Portage Windows (WIN)

Objectif : le jeu se lance et s'exporte sous Windows x86_64.

## Plan
1. `.gdextension` : entrées `windows.*.x86_64` vers `res://bin/cent_ans.{debug,release}.dll`.
2. Compilation croisée depuis le Mac : `cargo xwin build --target x86_64-pc-windows-msvc`
   (script `core/build-windows.sh`).
3. Profil d'export « Windows Desktop » dans `game/export_presets.cfg` (sortie `export/windows/`).
4. Vérification réelle : workflow GitHub Actions `windows-latest` (compile la dll, lance
   `smoke.gd` en headless).
5. ADR + README.

## État
- [ ] 1  - [ ] 2  - [ ] 3  - [ ] 4  - [ ] 5

## Prochaine étape
Installer `lld`, `cargo-xwin`, cible rustup `x86_64-pc-windows-msvc`.
