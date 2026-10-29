# Lot MEM5 : ~3 Go de mémoire graphique non expliqués en campagne

Branche `worktree-agent-a319572925dedf007`, base main 99d672914. Suite de MEM4 (ADR 0794).

## État
- Worktree remis sur main ; données ignorées liées (pyramid, models/dn) ou copiées (rsync --ignore-existing).
- Mesure de base en cours (binaire release du checkout principal, `mem_protocol.sh idle`).

## Prochaine étape
1. Décomposer l'empreinte (vmmap -v, footprint, memlog) menu -> campagne.
2. Bissection par sous-système (relief : sans pyramide -2,6 Go selon MEM4).

## Points ouverts
- (à remplir)
