# Mises à jour automatiques (MAJ) — ADR 0159

## État (2026-10-02)
Fait, dans main : étape 0 de `tools/launch.sh` (suivi de `stable`), job `promote` de
`.github/workflows/windows.yml`, tests `tools/tests/test_launch_update.py` (9 verts), README.

## Reste
- [ ] Premier push de `main` : vérifier que le run `windows` passe et que `promote` crée
  `stable` (`gh run list --workflow windows.yml`, `git ls-remote origin stable`). Tant que la
  branche n'existe pas, `git clone -b stable` échoue.
- [ ] Si `promote` échoue en 403 : Settings → Actions → General → Workflow permissions →
  « Read and write » (le job demande déjà `contents: write`).
- [ ] Essai sur un vrai PC : une mise à jour qui modifie `Lancer Cent Ans.exe` pendant qu'il
  tourne (renommage en `.exe.old`, non vérifié).
- [ ] Prévenir les joueurs déjà installés : `git fetch && git switch stable`, une fois.
