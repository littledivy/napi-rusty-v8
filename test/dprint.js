const dprint = dlopen("./testdata/node_modules/dprint-node/dprint-node.linux-x64-gnu.node");

print(
  dprint.format(
    "hello.js",
    "function x(){let a=1;return a;}",
    {
      lineWidth: 100,
      semiColons: "asi",
    },
  ),
);
