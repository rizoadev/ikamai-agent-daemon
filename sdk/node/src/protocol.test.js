const { test } = require('node:test');
const assert = require('node:assert');
const { FrameReader, ErrorClass, isRetryable } = require('./protocol');

function makeFrame(msg) {
  const p = Buffer.from(JSON.stringify(msg), 'utf8');
  const h = Buffer.allocUnsafe(4); h.writeUInt32LE(p.length, 0);
  return Buffer.concat([h, p]);
}

test('FrameReader single frame', () => {
  const frames = [];
  const r = new FrameReader((f) => frames.push(f));
  r.push(makeFrame({ type: 'PING', seq: 1 }));
  assert.strictEqual(frames.length, 1);
  assert.deepStrictEqual(frames[0], { type: 'PING', seq: 1 });
});

test('FrameReader chunked', () => {
  const frames = [];
  const r = new FrameReader((f) => frames.push(f));
  const full = makeFrame({ type: 'PONG', seq: 42 });
  r.push(full.slice(0, 2)); r.push(full.slice(2, 5)); r.push(full.slice(5));
  assert.strictEqual(frames.length, 1);
});

test('FrameReader multiple frames', () => {
  const frames = [];
  const r = new FrameReader((f) => frames.push(f));
  r.push(Buffer.concat([makeFrame({ n: 1 }), makeFrame({ n: 2 }), makeFrame({ n: 3 })]));
  assert.strictEqual(frames.length, 3);
});

test('ErrorClass isRetryable', () => {
  assert.ok(isRetryable(ErrorClass.TRANSIENT));
  assert.ok(isRetryable(ErrorClass.RATE_LIMIT));
  assert.ok(isRetryable(ErrorClass.TIMEOUT));
  assert.ok(!isRetryable(ErrorClass.FATAL));
});
