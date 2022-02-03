// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
// Used for benchmarking Deno's networking.
// TODO(bartlomieju): Replace this with a real HTTP server once
// https://github.com/denoland/deno/issues/726 is completed.
// Note: this is a keep-alive server.
const addr = Deno.args[0] || "127.0.0.1:4500";
const [hostname, port] = addr.split(":");
const listener = Deno.listen({ hostname, port: Number(port) });
const response = new TextEncoder().encode(
  "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello World\n",
);
async function handle(conn: Deno.Conn): Promise<void> {
  const buffer = new Uint8Array(1024);
  try {
    while (true) {
      await conn.read(buffer);
      await conn.write(response);
    }
  } catch (e) {
    if (
      !(e instanceof Deno.errors.BrokenPipe) &&
      !(e instanceof Deno.errors.ConnectionReset)
    ) {
      throw e;
    }
  }
  conn.close();
}

console.log("Listening on", addr);
for await (const conn of listener) {
  handle(conn);
}
