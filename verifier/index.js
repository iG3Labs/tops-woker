import express from "express";
import morgan from "morgan";
import { blake3 } from "@noble/hashes/blake3";
import { sha256 } from "@noble/hashes/sha256";
import { secp256k1 } from "@noble/curves/secp256k1";

const app = express();
app.use(express.json({ limit: "1mb" }));
app.use(morgan("dev"));

const VERIFY_PUBKEY = process.env.VERIFY_PUBKEY || ""; // hex (compressed or uncompressed)
const VERIFY_DISABLE = process.env.VERIFY_DISABLE === "1";

// Minimal schema check
function isValidReceipt(r) {
  if (!r) return false;
  const has = (k, t) =>
    Object.prototype.hasOwnProperty.call(r, k) && typeof r[k] === t;
  return (
    has("device_did", "string") &&
    has("epoch_id", "number") &&
    has("prev_hash_hex", "string") &&
    has("nonce", "number") &&
    has("work_root_hex", "string") &&
    r.sizes &&
    typeof r.sizes === "object" &&
    typeof r.sizes.m === "number" &&
    typeof r.sizes.n === "number" &&
    typeof r.sizes.k === "number" &&
    has("time_ms", "number") &&
    has("kernel_ver", "string") &&
    has("driver_hint", "string") &&
    has("sig_hex", "string")
  );
}

function hexToBytes(hex) {
  if (hex.startsWith("0x")) hex = hex.slice(2);
  if (hex.length % 2 !== 0) throw new Error("invalid hex length");
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++)
    out[i] = parseInt(hex.slice(2 * i, 2 * i + 2), 16);
  return out;
}

function computeMessageDigest(receipt) {
  const copy = { ...receipt, sig_hex: "" };
  const msg = new TextEncoder().encode(JSON.stringify(copy));
  const b3 = blake3(msg);
  return sha256(b3);
}

function parseSignatureBytes(sigHex) {
  const bytes = hexToBytes(sigHex);
  // Try DER -> convert to compact raw bytes for verify()
  try {
    const sigObj = secp256k1.Signature.fromDER(bytes);
    return sigObj.toCompactRawBytes();
  } catch {}
  // If already compact 64B, accept as-is
  if (bytes.length === 64) return bytes;
  throw new Error("unsupported signature encoding");
}

function normalizePubkey(pubHex) {
  const b = hexToBytes(pubHex);
  // Accept compressed(33) or uncompressed(65)
  if (b.length === 33 || b.length === 65) return b;
  throw new Error("invalid pubkey length");
}

// Health
app.get("/healthz", (_req, res) => res.json({ ok: true }));

// Verify endpoint
app.post("/verify", async (req, res) => {
  const receipt = req.body;
  if (!isValidReceipt(receipt)) {
    return res.status(400).json({ ok: false, error: "invalid schema" });
  }
  try {
    if (!/^([0-9a-f]{64})$/.test(receipt.work_root_hex)) {
      return res
        .status(400)
        .json({ ok: false, error: "invalid work_root_hex" });
    }
    if (!/^([0-9a-f]{64})$/.test(receipt.prev_hash_hex)) {
      return res
        .status(400)
        .json({ ok: false, error: "invalid prev_hash_hex" });
    }
    const { m, n, k } = receipt.sizes;
    if (m <= 0 || n <= 0 || k <= 0 || m > 8192 || n > 8192 || k > 8192) {
      return res.status(400).json({ ok: false, error: "unreasonable sizes" });
    }

    const digest = computeMessageDigest(receipt);

    let sigOk = false;
    let pubHexUsed = null;
    if (VERIFY_DISABLE) {
      sigOk = true;
    } else if (VERIFY_PUBKEY) {
      const pub = normalizePubkey(VERIFY_PUBKEY);
      const sigBytes = parseSignatureBytes(receipt.sig_hex);
      sigOk = secp256k1.verify(sigBytes, digest, pub);
      pubHexUsed = VERIFY_PUBKEY;
    } else {
      // If no VERIFY_PUBKEY provided, accept unsigned for dev but report digest
      sigOk = receipt.sig_hex.length === 0;
    }

    return res.json({
      ok: true,
      sig_ok: sigOk,
      pubkey_hex: pubHexUsed,
      digest_hex: Buffer.from(digest).toString("hex"),
    });
  } catch (e) {
    return res
      .status(500)
      .json({ ok: false, error: e.message || "verify failed" });
  }
});

const port = process.env.PORT ? Number(process.env.PORT) : 8081;
app.listen(port, () => {
  console.log(`verifier listening on :${port}`);
});
