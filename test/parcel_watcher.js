const watcher = dlopen(
  "./testdata/node_modules/@parcel/watcher/prebuilds/darwin-arm64/node.napi.glibc.node",
);

watcher.subscribe(".", (err, events) => {
  print(events);
}, {});