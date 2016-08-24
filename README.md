Really dumb streaming diff program that assumes that the only differences will be intra-line (e.g.,
self-synchronizes on line boundaries).

## Advantages:

 - Everything is done in a streaming fashion; doesn't have to pull giant files into RAM

## Disadvantages

 - Nearly everything else
