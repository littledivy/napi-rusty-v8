const usb = dlopen("./testdata/node_modules/usb/prebuilds/linux-x64/node.napi.glibc.node");

print(usb.getDeviceList());

