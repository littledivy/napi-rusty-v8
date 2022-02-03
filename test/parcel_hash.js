const lib = dlopen(
  "./testdata/node_modules/@parcel/hash/parcel-hash.darwin-arm64.node",
);

print(lib.hashString("Hello, Deno!")); // 210a1f862b67f327
print(lib.hashBuffer(Deno.core.encode("Hello, Deno!"))); // 210a1f862b67f327

const hasher = new lib.Hash();
hasher.writeString("Hello, Deno!");
print(hasher.finish()); // 210a1f862b67f327
