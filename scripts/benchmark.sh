#!/bin/sh

hyperfine --warmup 3 \
  'seq 1 1000 | rust-parallel echo' \
  'seq 1 1000 | rust-parallel --disable-path-cache echo' \
  'seq 1 1000 | xargs -P8 -L1 echo' \
  'seq 1 1000 | parallel echo'
