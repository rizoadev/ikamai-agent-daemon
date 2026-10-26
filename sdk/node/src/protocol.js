// IAD wire protocol — length-prefixed JSON frames (Node.js)
const MAX_FRAME_SIZE = 4 * 1024 * 1024;
const ErrorClass = Object.freeze({ TRANSIENT: 'transient', RATE_LIMIT: 'rate_limit', FATAL: 'fatal', TIMEOUT: 'timeout' });
function isRetryable(ec) { return [ErrorClass.TRANSIENT, ErrorClass.RATE_LIMIT, ErrorClass.TIMEOUT].includes(ec); }

function writeFrame(socket, msg) {
  const payload = Buffer.from(JSON.stringify(msg), 'utf8');
  if (payload.length > MAX_FRAME_SIZE) throw new Error(`frame too large: ${payload.length}`);
  const hdr = Buffer.allocUnsafe(4);
  hdr.writeUInt32LE(payload.length, 0);
  socket.write(Buffer.concat([hdr, payload]));
}

class FrameReader {
  constructor(onFrame) { this._buf = Buffer.alloc(0); this._onFrame = onFrame; }
  push(data) {
    this._buf = Buffer.concat([this._buf, data]);
    while (this._buf.length >= 4) {
      const len = this._buf.readUInt32LE(0);
      if (len > MAX_FRAME_SIZE) throw new Error(`frame too large: ${len}`);
      if (this._buf.length < 4 + len) break;
      const payload = this._buf.slice(4, 4 + len);
      this._buf = this._buf.slice(4 + len);
      this._onFrame(JSON.parse(payload.toString('utf8')));
    }
  }
}

module.exports = { writeFrame, FrameReader, ErrorClass, isRetryable };
