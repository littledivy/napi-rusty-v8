const lib = dlopen(
  "./testdata/node_modules/@parcel/fs-search/fs-search.darwin-arm64.node",
);

const file = lib.findFirstFile(
  [
    "./test/example_non_existent.js",
    "./test/example.js",
    "./test/example_non_existent2.js",
  ],
);

print(file); // ./test/example.js
