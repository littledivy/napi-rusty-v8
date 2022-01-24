const xattr = dlopen("testdata/node_modules/fs-xattr/build/Release/xattr.node");

xattr.get("exports.def", "A").catch(print);

