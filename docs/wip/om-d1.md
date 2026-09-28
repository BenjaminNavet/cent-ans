# OM-D1 — Registre Nord et Baltique (1337)

Branche `feat/om-d1`. Périmètre : Danemark (interrègne), Suède-Norvège, Finlande, ordre Teutonique,
Livonie, Poméranie.

## État
- [ ] squelette et plan (ce fichier)
- [ ] provinces, titres, factions, personnages, colonies
- [ ] noms, héraldie, front_end, portraits
- [ ] validation (pytest + script de contrôle)

## Décisions de conception (à valider par l'orchestrateur)
- Couronne danoise vacante 1332-1340 : `tit_denmark` (royaume) détenu par `fac_holstein` (Gérard III, régent
  et gagiste) ; `tit_holstein` supprimé (doublon fictif). Jean III de Holstein-Kiel (Sjælland, gage) et le duc de
  Slesvig sont ses vassaux de jure sans hommage effectif ; l'Estonie danoise idem.
- Norvège : pas de faction propre, `tit_norway` tenu par `fac_sweden` (union personnelle).
- Ermland (siège vacant 1334-1337) fondu dans `prov_natangia`, tenu par l'Ordre.

## Prochaine étape
Écrire le générateur (scratchpad) puis les données.
