// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
import {
  assert,
  assertEquals,
  assertThrows,
  fail,
} from "../../../test_util/std/testing/asserts.ts";
import { deferred } from "../../../test_util/std/async/deferred.ts";

Deno.test("invalid scheme", () => {
  assertThrows(() => new WebSocket("foo://localhost:4242"));
});

Deno.test("fragment", () => {
  assertThrows(() => new WebSocket("ws://localhost:4242/#"));
  assertThrows(() => new WebSocket("ws://localhost:4242/#foo"));
});

Deno.test("duplicate protocols", () => {
  assertThrows(() => new WebSocket("ws://localhost:4242", ["foo", "foo"]));
});

Deno.test("invalid server", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:2121");
  let err = false;
  ws.onerror = () => {
    err = true;
  };
  ws.onclose = () => {
    if (err) {
      promise.resolve();
    } else {
      fail();
    }
  };
  ws.onopen = () => fail();
  await promise;
});

Deno.test("connect & close", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.onerror = () => fail();
  ws.onopen = () => {
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("connect & abort", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.close();
  let err = false;
  ws.onerror = () => {
    err = true;
  };
  ws.onclose = () => {
    if (err) {
      promise.resolve();
    } else {
      fail();
    }
  };
  ws.onopen = () => fail();
  await promise;
});

Deno.test("connect & close custom valid code", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.onerror = () => fail();
  ws.onopen = () => ws.close(1000);
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("connect & close custom invalid code", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.onerror = () => fail();
  ws.onopen = () => {
    assertThrows(() => ws.close(1001));
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("connect & close custom valid reason", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.onerror = () => fail();
  ws.onopen = () => ws.close(1000, "foo");
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("connect & close custom invalid reason", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.onerror = () => fail();
  ws.onopen = () => {
    assertThrows(() => ws.close(1000, "".padEnd(124, "o")));
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("echo string", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.onerror = () => fail();
  ws.onopen = () => ws.send("foo");
  ws.onmessage = (e) => {
    assertEquals(e.data, "foo");
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("echo string tls", async () => {
  const promise1 = deferred();
  const promise2 = deferred();
  const ws = new WebSocket("wss://localhost:4243");
  ws.onerror = () => fail();
  ws.onopen = () => ws.send("foo");
  ws.onmessage = (e) => {
    assertEquals(e.data, "foo");
    ws.close();
    promise1.resolve();
  };
  ws.onclose = () => {
    promise2.resolve();
  };
  await promise1;
  await promise2;
});

Deno.test("websocket error", async () => {
  const promise1 = deferred();
  const ws = new WebSocket("wss://localhost:4242");
  ws.onopen = () => fail();
  ws.onerror = (err) => {
    assert(err instanceof ErrorEvent);

    // Error message got changed because we don't use warp in test_util
    assertEquals(err.message, "UnexpectedEof: tls handshake eof");
    promise1.resolve();
  };
  await promise1;
});

Deno.test("echo blob with binaryType blob", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  const blob = new Blob(["foo"]);
  ws.onerror = () => fail();
  ws.onopen = () => ws.send(blob);
  ws.onmessage = (e) => {
    e.data.text().then((actual: string) => {
      blob.text().then((expected) => {
        assertEquals(actual, expected);
      });
    });
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("echo blob with binaryType arraybuffer", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.binaryType = "arraybuffer";
  const blob = new Blob(["foo"]);
  ws.onerror = () => fail();
  ws.onopen = () => ws.send(blob);
  ws.onmessage = (e) => {
    blob.arrayBuffer().then((expected) => {
      assertEquals(e.data, expected);
    });
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("echo uint8array with binaryType blob", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  const uint = new Uint8Array([102, 111, 111]);
  ws.onerror = () => fail();
  ws.onopen = () => ws.send(uint);
  ws.onmessage = (e) => {
    e.data.arrayBuffer().then((actual: ArrayBuffer) => {
      assertEquals(actual, uint.buffer);
    });
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("echo uint8array with binaryType arraybuffer", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.binaryType = "arraybuffer";
  const uint = new Uint8Array([102, 111, 111]);
  ws.onerror = () => fail();
  ws.onopen = () => ws.send(uint);
  ws.onmessage = (e) => {
    assertEquals(e.data, uint.buffer);
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("echo arraybuffer with binaryType blob", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  const buffer = new ArrayBuffer(3);
  ws.onerror = () => fail();
  ws.onopen = () => ws.send(buffer);
  ws.onmessage = (e) => {
    e.data.arrayBuffer().then((actual: ArrayBuffer) => {
      assertEquals(actual, buffer);
    });
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("echo arraybuffer with binaryType arraybuffer", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  ws.binaryType = "arraybuffer";
  const buffer = new ArrayBuffer(3);
  ws.onerror = () => fail();
  ws.onopen = () => ws.send(buffer);
  ws.onmessage = (e) => {
    assertEquals(e.data, buffer);
    ws.close();
  };
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("Event Handlers order", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4242");
  const arr: number[] = [];
  ws.onerror = () => fail();
  ws.addEventListener("message", () => arr.push(1));
  ws.onmessage = () => fail();
  ws.addEventListener("message", () => {
    arr.push(3);
    ws.close();
    assertEquals(arr, [1, 2, 3]);
  });
  ws.onmessage = () => arr.push(2);
  ws.onopen = () => ws.send("Echo");
  ws.onclose = () => {
    promise.resolve();
  };
  await promise;
});

Deno.test("Close without frame", async () => {
  const promise = deferred();
  const ws = new WebSocket("ws://localhost:4244");
  ws.onerror = () => fail();
  ws.onclose = (e) => {
    assertEquals(e.code, 1005);
    promise.resolve();
  };
  await promise;
});
