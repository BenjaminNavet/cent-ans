# TB — carte de campagne façon Thrones of Britannia (orchestration)

Demande du joueur (02/10) : rendre le jeu « beaucoup plus joli », référence principale **Thrones of
Britannia**, priorité **campagne**, dépassement du plafond de 50 $ accepté **uniquement sur fal.ai**,
références à récupérer en ligne.

## État
- [x] Recherche des références ToB : texte fait (sources dans le plan). Images : bloquées par le
      réseau du conteneur cloud (Steam, ArtStation, Wikimedia, presse et fal.ai refusés).
- [x] Inventaire de l'existant de la carte de campagne (plan § 2)
- [x] Plan `docs/design/2026-10-02-campagne-tob.md` (lots TB0-TB7)
- [x] ADR 0149 (fal.ai hors plafond, enveloppe 25 $) + section dans `docs/budget.md`
- [ ] Validation du plan par le joueur
- [ ] TB0 planche cible (session locale : Godot + fal.ai)
- [ ] Vague 1 : TB1 saisons visibles, TB2 désencombrement

## Prochaine étape
Le joueur valide le plan. Ensuite, TB1 commence par le correctif de masquage de la carte de couleur
(`game/shaders/satellite_ground.gdshaderinc:22-28`), sans asset, testable sans fal.ai, mais pas
vérifiable visuellement sans Godot.
