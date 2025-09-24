#! /bin/zsh

rm -rf ci-user
git clone https://github.com/LearningOS/rCore-Tutorial-Checker.git ci-user
git clone https://github.com/LearningOS/rCore-Tutorial-Test.git ci-user/user
cd ci-user && make test CHAPTER=$1