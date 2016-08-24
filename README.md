Really dumb streaming diff program that assumes that the only differences will be intra-line (e.g.,
self-synchronizes on line boundaries).

Really, our use case is if we have `mysqldump` output for two tables which have the same number of rows,
but may have changes on some of the rows. The number and order of the lines is the same, but some contents may
differ. This tool will find those differing lines.

Prints out status to stderr every 100k lines.

### Advantages:

 - Everything is done in a streaming fashion; doesn't have to pull giant files into RAM

### Disadvantages

 - Nearly everything else

### Alternatives

 - `rdiff` works, but the output is really hard to work with, and it's somehow even slower than this naive tool
 - `cmp` doesn't give enough information and requires a lot of scripting to actually find useful information
 - `diff`, even with `--speed-large-files`, puts the entire file in RAM. lame.
