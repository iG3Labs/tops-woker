import express from 'express';
import morgan from 'morgan';
import { blake3 } from 'blake3';

const app = express();
app.use(express.json({ limit: '1mb' }));
app.use(morgan('dev'));

// Minimal schema check
function isValidReceipt(r) {
  if (!r) return false;
  const has = (k, t) => Object.prototype.hasOwnProperty.call(r, k) && typeof r[k] === t;
  return has('device_did', 'string') && has('epoch_id', 'number') && has('prev_hash_hex', 'string') &&
    has('nonce', 'number') && has('work_root_hex', 'string') && r.sizes && typeof r.sizes === 'object' &&
    typeof r.sizes.m === 'number' && typeof r.sizes.n === 'number' && typeof r.sizes.k === 'number' &&
    has('time_ms', 'number') && has('kernel_ver', 'string') && has('driver_hint', 'string') && has('sig_hex', 'string');
}

// Health
app.get('/healthz', (_req, res) => res.json({ ok: true }));

// Verify endpoint: only light checks here
app.post('/verify', async (req, res) => {
  const receipt = req.body;
  if (!isValidReceipt(receipt)) {
    return res.status(400).json({ ok: false, error: 'invalid schema' });
  }
  try {
    // Light validation: work_root_hex format, sizes sane, prev_hash_hex length
    if (!/^([0-9a-f]{64})$/.test(receipt.work_root_hex)) {
      return res.status(400).json({ ok: false, error: 'invalid work_root_hex' });
    }
    if (!/^([0-9a-f]{64})$/.test(receipt.prev_hash_hex)) {
      return res.status(400).json({ ok: false, error: 'invalid prev_hash_hex' });
    }
    const { m, n, k } = receipt.sizes;
    if (m <= 0 || n <= 0 || k <= 0 || m > 8192 || n > 8192 || k > 8192) {
      return res.status(400).json({ ok: false, error: 'unreasonable sizes' });
    }
    // Optionally, recompute message hash and verify signature (omitted minimal MVP)
    // Placeholder: compute blake3(JSON(receipt without sig_hex)) for logging
    const copy = { ...receipt, sig_hex: '' };
    const msg = Buffer.from(JSON.stringify(copy));
    const digest = blake3(msg);
    return res.json({ ok: true, work_root_hex: receipt.work_root_hex, digest_hex: Buffer.from(digest).toString('hex') });
  } catch (e) {
    return res.status(500).json({ ok: false, error: e.message || 'verify failed' });
  }
});

const port = process.env.PORT ? Number(process.env.PORT) : 8081;
app.listen(port, () => {
  console.log(`verifier listening on :${port}`);
});


