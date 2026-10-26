const net = require('net');
const { writeFrame, FrameReader, ErrorClass } = require('./protocol');

class CancellationToken {
  constructor() { this._c = false; }
  cancel()      { this._c = true; }
  isCancelled() { return this._c; }
}

class IadAgent {
  constructor({ socketPath = '/home/user/.iad/run/iad.sock', name = 'node-agent', capabilities = [], heartbeatMs = 15000 } = {}) {
    this.socketPath   = socketPath;
    this.name         = name;
    this.capabilities = capabilities;
    this.heartbeatMs  = heartbeatMs;
    this._handlers    = new Map();
    this._tokens      = new Map();
    this._seq         = 0;
  }

  onTask(...caps) {
    return (fn) => { for (const c of caps) this._handlers.set(c, fn); return fn; };
  }

  run() {
    return new Promise((resolve, reject) => {
      const socket = net.createConnection(this.socketPath, () => {
        writeFrame(socket, { type: 'CAPABILITIES', caps: this.capabilities });
      });
      const reader = new FrameReader((f) => this._handle(f, socket));
      socket.on('data', (d) => reader.push(d));
      socket.on('error', reject);
      socket.on('close', resolve);
      const hb = setInterval(() => {
        if (!socket.destroyed) writeFrame(socket, { type: 'PONG', seq: ++this._seq });
      }, this.heartbeatMs);
      socket.on('close', () => clearInterval(hb));
    });
  }

  _handle(frame, socket) {
    if (frame.type === 'ASSIGN_TASK') this._task(frame.task_id, frame.payload, socket);
    else if (frame.type === 'CANCEL_TASK') { const t = this._tokens.get(frame.task_id); if (t) t.cancel(); }
    else if (frame.type === 'PING') writeFrame(socket, { type: 'PONG', seq: frame.seq ?? 0 });
    else if (frame.type === 'SHUTDOWN') socket.end();
  }

  async _task(id, payload, socket) {
    const h = payload?.capability && this._handlers.has(payload.capability)
      ? this._handlers.get(payload.capability)
      : this._handlers.values().next().value;
    if (!h) { writeFrame(socket, { type: 'TASK_FAILED', task_id: id, error: 'no handler', error_class: ErrorClass.FATAL, retryable: false }); return; }
    const ct = new CancellationToken();
    this._tokens.set(id, ct);
    try {
      const result = await h(id, payload, ct);
      writeFrame(socket, { type: 'TASK_COMPLETE', task_id: id, result });
    } catch (e) {
      writeFrame(socket, { type: 'TASK_FAILED', task_id: id, error: e.message, error_class: ErrorClass.TRANSIENT, retryable: true });
    } finally {
      this._tokens.delete(id);
    }
  }
}

module.exports = { IadAgent, CancellationToken };
