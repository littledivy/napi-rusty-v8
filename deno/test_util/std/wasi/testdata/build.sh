#!/usr/bin/env bash

shopt -s globstar

rm -rf fixtures
rm *.json
rm *.wasm

mkdir crates
cd crates

git clone https://github.com/khronosproject/wasi-test
cd wasi-test

commit_sha="$(git rev-parse HEAD)"
commit_message="$(git show --format='%B' --quiet $commit_sha)"
author_name="$(git show --format='%aN' --quiet $commit_sha)"
author_email="$(git show --format='%aE' --quiet $commit_sha)"

echo $author_name
echo $author_email
echo $commit_message

cargo build --target=wasm32-wasi --release

mv fixtures ../../
mv target/wasm32-wasi/release/**/*.json ../../
mv target/wasm32-wasi/release/**/*.wasm ../../
cd ..
rm -rf wasi-test
cd ..

git add fixtures/*
git add *.json
git add *.wasm

export GIT_AUTHOR_NAME="$author_name"
export GIT_AUTHOR_EMAIL="$author_email"

git commit --message "$commit_message"
