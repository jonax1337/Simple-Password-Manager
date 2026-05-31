// Thin wrapper around the password-wallet-server REST API. Mirrors the
// types used by the desktop client; kept independent so the dashboard can
// evolve at its own pace.

import { argon2id } from "hash-wasm";

export interface VaultSummary {
  id: string;
  name: string;
  role: "owner" | "editor" | "reader";
  owner_user_id: string;
  etag: string;
  created_at: number;
  updated_at: number;
  wrapped_vault_key_b64: string;
}

export interface KdfParamsResp {
  kdf_salt_b64: string;
  wrapped_master_key_b64: string;
}

export interface LoginResp {
  token: string;
  user_id: string;
  kdf_salt_b64: string;
  wrapped_master_key_b64: string;
}

const KDF_PARAMS = {
  memorySize: 64 * 1024,
  iterations: 3,
  parallelism: 4,
  hashLength: 32,
};

function b64Decode(s: string): Uint8Array {
  const binary = atob(s);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i);
  return out;
}

function b64Encode(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

async function sha256(bytes: Uint8Array): Promise<Uint8Array> {
  // Cast through a fresh Uint8Array view backed by a regular ArrayBuffer.
  // TS 5.6 tightened BufferSource: a Uint8Array typed as ArrayBufferLike
  // is no longer assignable. Allocating a new buffer + copying bytes
  // guarantees the typed-array's `buffer` is a plain ArrayBuffer.
  const buf = new ArrayBuffer(bytes.length);
  new Uint8Array(buf).set(bytes);
  const out = await crypto.subtle.digest("SHA-256", buf);
  return new Uint8Array(out);
}

/// Derive the same `client_auth_hash` the desktop sends. The dashboard
/// doesn't need the vault_key — it never decrypts vault contents — but
/// it does need to prove possession of master_key to the server.
export async function deriveAuthHash(
  password: string,
  kdfSaltB64: string,
  wrappedMasterKeyB64: string,
): Promise<string> {
  const salt = b64Decode(kdfSaltB64);
  const passwordKey = await argon2id({
    password,
    salt,
    ...KDF_PARAMS,
    outputType: "binary",
  });
  const wrapped = b64Decode(wrappedMasterKeyB64);
  // AES-GCM unwrap: nonce(12) || ciphertext+tag(rest). Copy into fresh
  // ArrayBuffers so SubtleCrypto's BufferSource accepts them under
  // strict TS 5.6 typings.
  const nonceBuf = new ArrayBuffer(12);
  new Uint8Array(nonceBuf).set(wrapped.subarray(0, 12));
  const ctBuf = new ArrayBuffer(wrapped.length - 12);
  new Uint8Array(ctBuf).set(wrapped.subarray(12));

  const keyBuf = new ArrayBuffer(passwordKey.length);
  new Uint8Array(keyBuf).set(passwordKey);
  const key = await crypto.subtle.importKey(
    "raw",
    keyBuf,
    { name: "AES-GCM" },
    false,
    ["decrypt"],
  );
  const masterKey = new Uint8Array(
    await crypto.subtle.decrypt({ name: "AES-GCM", iv: nonceBuf }, key, ctBuf),
  );
  const authInput = new Uint8Array(masterKey.length + 4);
  authInput.set(masterKey);
  authInput.set(new TextEncoder().encode("auth"), masterKey.length);
  const authHash = await sha256(authInput);
  return b64Encode(authHash);
}

export class DashboardApi {
  constructor(public baseUrl: string, public token: string | null = null) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
  }

  private async json<T>(path: string, init?: RequestInit): Promise<T> {
    const headers = new Headers(init?.headers);
    headers.set("content-type", "application/json");
    if (this.token) headers.set("authorization", `Bearer ${this.token}`);
    const res = await fetch(this.baseUrl + path, { ...init, headers });
    if (!res.ok) {
      const text = await res.text().catch(() => "");
      throw new Error(`${res.status} ${res.statusText}${text ? `: ${text}` : ""}`);
    }
    return res.json();
  }

  async kdfParams(email: string): Promise<KdfParamsResp> {
    return this.json("/auth/kdf-params", {
      method: "POST",
      body: JSON.stringify({ email, client_auth_hash: "x" }),
    });
  }

  async login(email: string, clientAuthHash: string): Promise<LoginResp> {
    const resp = await this.json<LoginResp>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ email, client_auth_hash: clientAuthHash }),
    });
    this.token = resp.token;
    return resp;
  }

  async listVaults(): Promise<VaultSummary[]> {
    const resp = await this.json<{ vaults: VaultSummary[] }>("/vaults");
    return resp.vaults;
  }

  async renameVault(id: string, name: string): Promise<void> {
    const headers = new Headers({ "content-type": "application/json" });
    if (this.token) headers.set("authorization", `Bearer ${this.token}`);
    const res = await fetch(`${this.baseUrl}/vaults/${id}`, {
      method: "PATCH",
      headers,
      body: JSON.stringify({ name }),
    });
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  }

  async deleteVault(id: string): Promise<void> {
    const headers = new Headers();
    if (this.token) headers.set("authorization", `Bearer ${this.token}`);
    const res = await fetch(`${this.baseUrl}/vaults/${id}`, {
      method: "DELETE",
      headers,
    });
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  }
}
