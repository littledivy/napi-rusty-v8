import { deferred } from "../../../test_util/std/async/deferred.ts";
import {
  assert,
  assertEquals,
} from "../../../test_util/std/testing/asserts.ts";
import { BufReader, BufWriter } from "../../../test_util/std/io/bufio.ts";
import { TextProtoReader } from "../../../test_util/std/textproto/mod.ts";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const resolvable = deferred();
const hostname = "localhost";
const port = 3505;

const listener = Deno.listenTls({
  hostname,
  port,
  certFile: "./tls/localhost.crt",
  keyFile: "./tls/localhost.key",
});

const response = encoder.encode(
  "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nHello World\n",
);

listener.accept().then(
  async (conn) => {
    assert(conn.remoteAddr != null);
    assert(conn.localAddr != null);
    await conn.write(response);
    // TODO(bartlomieju): this might be a bug
    setTimeout(() => {
      conn.close();
      resolvable.resolve();
    }, 0);
  },
);

const conn = await Deno.connectTls({
  hostname,
  port,
});
assert(conn.rid > 0);
const w = new BufWriter(conn);
const r = new BufReader(conn);
const body = `GET / HTTP/1.1\r\nHost: ${hostname}:${port}\r\n\r\n`;
const writeResult = await w.write(encoder.encode(body));
assertEquals(body.length, writeResult);
await w.flush();
const tpr = new TextProtoReader(r);
const statusLine = await tpr.readLine();
assert(statusLine !== null, `line must be read: ${String(statusLine)}`);
const m = statusLine.match(/^(.+?) (.+?) (.+?)$/);
assert(m !== null, "must be matched");
const [_, proto, status, ok] = m;
assertEquals(proto, "HTTP/1.1");
assertEquals(status, "200");
assertEquals(ok, "OK");
const headers = await tpr.readMIMEHeader();
assert(headers !== null);
const contentLength = parseInt(headers.get("content-length"));
const bodyBuf = new Uint8Array(contentLength);
await r.readFull(bodyBuf);
assertEquals(decoder.decode(bodyBuf), "Hello World\n");
conn.close();
listener.close();
await resolvable;

console.log("DONE");
