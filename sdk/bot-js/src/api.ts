export type ApiOptions = {
  /** For example `https://rspc.tawks.app/rspc`, or `http://localhost:8787/rspc` for devhost. */
  rspcUrl: string;
  token: string;
  /** How the token is sent in the Authorization header. Bot tokens use `Bot`. */
  tokenType?: string;
  /** Per request, in milliseconds. */
  timeoutMs?: number;
};

export class TawksApiError extends Error {
  constructor(
    readonly procedure: string,
    message: string,
    readonly data?: unknown,
  ) {
    super(`${procedure}: ${message}`);
    this.name = 'TawksApiError';
  }
}

/** Tokens only travel in the clear to this machine or a private network. */
export function requireTls(url: string): void {
  const parsed = new URL(url);
  const host = parsed.hostname.replace(/^\[|\]$/g, '');
  const privateHost =
    host === 'localhost' ||
    !host.includes('.') ||
    /^127\./.test(host) ||
    /^10\./.test(host) ||
    /^192\.168\./.test(host) ||
    /^172\.(1[6-9]|2\d|3[01])\./.test(host) ||
    /^169\.254\./.test(host) ||
    host === '::1';
  const scheme = parsed.protocol.replace(':', '');
  if (scheme === 'https' || scheme === 'wss') return;
  if ((scheme === 'http' || scheme === 'ws') && privateHost) return;
  throw new Error(`refusing to send the bot token over ${scheme}:// to a public host; use https or wss`);
}

type Envelope = { type?: string; data?: unknown; result?: { type?: string; data?: unknown } };

function unwrap(procedure: string, text: string): unknown {
  let body: Envelope;
  try {
    body = JSON.parse(text);
  } catch {
    throw new TawksApiError(procedure, `unexpected response: ${text.slice(0, 200)}`);
  }
  const inner = body.result ?? body;
  if (inner.type !== 'response') {
    const data = inner.data as { message?: string } | string | undefined;
    const message = typeof data === 'string' ? data : (data?.message ?? JSON.stringify(data));
    throw new TawksApiError(procedure, message, data);
  }
  return inner.data;
}

/** Calls tawks API procedures by name. Every procedure takes and returns JSON. */
export class TawksApi {
  readonly rspcUrl: string;
  private readonly authorization: string;
  private readonly timeoutMs: number;

  constructor(options: ApiOptions) {
    requireTls(options.rspcUrl);
    this.rspcUrl = options.rspcUrl.replace(/\/+$/, '');
    this.authorization = `${options.tokenType ?? 'Bot'} ${options.token}`;
    this.timeoutMs = options.timeoutMs ?? 60_000;
  }

  /** Runs a query procedure, such as `workflow_definition_availability`. */
  query<T = unknown>(procedure: string, input?: unknown): Promise<T> {
    return this.call<T>(procedure, input);
  }

  /** Runs a mutation procedure, such as `chat_create_message`. */
  mutate<T = unknown>(procedure: string, input?: unknown): Promise<T> {
    return this.call<T>(procedure, input);
  }

  /** A short-lived session token, which the events socket needs. */
  async accessToken(): Promise<string> {
    const response = await fetch(`${this.rspcUrl}/auth_access_token`, {
      method: 'POST',
      headers: { authorization: this.authorization },
      signal: AbortSignal.timeout(this.timeoutMs),
    });
    const text = await response.text();
    try {
      const body = JSON.parse(text);
      const token = body?.result?.data?.access_token ?? body?.access_token;
      if (typeof token === 'string') return token;
    } catch {
      // Falls through to the error below.
    }
    throw new TawksApiError('auth_access_token', `failed with ${response.status}; check the token`);
  }

  private async call<T>(procedure: string, input: unknown): Promise<T> {
    const response = await fetch(`${this.rspcUrl}/${procedure}`, {
      method: 'POST',
      headers: { authorization: this.authorization, 'content-type': 'application/json' },
      body: JSON.stringify(input ?? null),
      signal: AbortSignal.timeout(this.timeoutMs),
    });
    return unwrap(procedure, await response.text()) as T;
  }
}
