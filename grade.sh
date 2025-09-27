#! /bin/zsh

LOGLV=INFO

LOGLV={$1:-$LOGLV}
cd ci-user && make test CHAPTER=8 LOG=$LOGLV