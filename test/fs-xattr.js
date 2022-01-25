const xattr = dlopen("testdata/node_modules/fs-xattr/build/Release/xattr.node");

xattr.set("example.txt", "foo", Deno.core.encode("bar"));
xattr.get("exports.def", "foo").catch(print);

