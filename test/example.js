const exports = dlopen("./example_module/target/release/libexample_module.so");

print(exports.hello("Rust"));
print(exports.add(1, 2));

const point = new exports.Point(1, 2);
print("point.x: " + point.get_x());
print("point.y: " + point.get_y());
point.set_x(3);
print("point.x: " + point.get_x());

// For testing async
// print(exports.readFileAsync("exports.def"));
