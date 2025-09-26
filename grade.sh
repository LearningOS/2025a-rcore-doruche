#! /bin/zsh

LOGLV=WARN

LOGLV=${1:-$LOGLV}
cd ci-user && make test CHAPTER=5 LOG=$LOGLV