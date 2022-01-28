const xattr = dlopen("testdata/node_modules/fs-xattr/build/Release/xattr.node");

print(xattr.set("example.txt", "foo", Deno.core.encode("bar")).then(print).catch(print));
// xattr.get("exports.def", "foo").catch(print);
