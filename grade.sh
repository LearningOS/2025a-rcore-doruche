#! /bin/zsh

LOGLV=INFO

LOGLV=${1:-$LOGLV}
cd ci-user && make test CHAPTER=6 LOG=$LOGLV