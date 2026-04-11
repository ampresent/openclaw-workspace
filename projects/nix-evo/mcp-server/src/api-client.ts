/**
 * Agent API client with retry, timeout, and error classification.
 */

export interface AgentRequestOptions {
  timeoutMs?: number;
  maxRetries?: number;
  retryDelayMs?: number;
}

export class AgentError extends Error {
  constructor(
    message: string,
    public readonly statusCode: number,
    public readonly code: string,
    public readonly retryable: boolean
  ) {
    super(message);
    this.name = "AgentError";
  }
}

const DEFAULT_OPTIONS: Required<AgentRequestOptions> = {
  timeoutMs: 60_000,
  maxRetries: 2,
  retryDelayMs: 1000,
};

/**
 * Classify whether an HTTP error is retryable.
 */
function isRetryable(status: number): boolean {
  // 429 (rate limit), 502 (bad gateway), 503 (service unavailable), 504 (timeout)
  return [429, 502, 503, 504].includes(status);
}

/**
 * Classify error code from status.
 */
function errorCode(status: number): string {
  switch (status) {
    case 400: return "VALIDATION_ERROR";
    case 401: return "UNAUTHORIZED";
    case 404: return "NOT_FOUND";
    case 429: return "RATE_LIMITED";
    case 500: return "AGENT_ERROR";
    case 502: return "BAD_GATEWAY";
    case 503: return "SERVICE_UNAVAILABLE";
    case 504: return "TIMEOUT";
    default: return "UNKNOWN_ERROR";
  }
}

/**
 * Fetch with timeout support.
 */
async function fetchWithTimeout(
  url: string,
  init: RequestInit,
  timeoutMs: number
): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const res = await fetch(url, { ...init, signal: controller.signal });
    return res;
  } catch (err: any) {
    if (err.name === "AbortError") {
      throw new AgentError(
        `请求超时 (${timeoutMs}ms)`,
        504,
        "TIMEOUT",
        true
      );
    }
    throw new AgentError(
      `连接失败: ${err.message}`,
      502,
      "CONNECTION_FAILED",
      true
    );
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Agent API GET request with retry.
 */
export async function agentGet(
  baseUrl: string,
  token: string | undefined,
  path: string,
  params: Record<string, string> = {},
  options: AgentRequestOptions = {}
): Promise<any> {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const url = new URL(path, baseUrl);
  for (const [k, v] of Object.entries(params)) {
    if (v) url.searchParams.set(k, v);
  }

  const headers: Record<string, string> = { Accept: "application/json" };
  if (token) headers["Authorization"] = `Bearer ${token}`;

  return requestWithRetry(
    () => fetchWithTimeout(url.toString(), { headers }, opts.timeoutMs),
    opts
  );
}

/**
 * Agent API POST request with retry.
 */
export async function agentPost(
  baseUrl: string,
  token: string | undefined,
  path: string,
  body: any,
  options: AgentRequestOptions = {}
): Promise<any> {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const url = new URL(path, baseUrl);

  const headers: Record<string, string> = {
    Accept: "application/json",
    "Content-Type": "application/json",
  };
  if (token) headers["Authorization"] = `Bearer ${token}`;

  return requestWithRetry(
    () =>
      fetchWithTimeout(
        url.toString(),
        {
          method: "POST",
          headers,
          body: JSON.stringify(body),
        },
        opts.timeoutMs
      ),
    opts
  );
}

/**
 * Execute a request with exponential backoff retry.
 */
async function requestWithRetry(
  doRequest: () => Promise<Response>,
  opts: Required<AgentRequestOptions>
): Promise<any> {
  let lastError: AgentError | undefined;

  for (let attempt = 0; attempt <= opts.maxRetries; attempt++) {
    if (attempt > 0) {
      const delay = opts.retryDelayMs * Math.pow(2, attempt - 1);
      console.error(`Retry ${attempt}/${opts.maxRetries} after ${delay}ms...`);
      await new Promise((r) => setTimeout(r, delay));
    }

    try {
      const res = await doRequest();

      if (!res.ok) {
        const body = await res.text().catch(() => "");
        const retryable = isRetryable(res.status);

        if (retryable && attempt < opts.maxRetries) {
          lastError = new AgentError(
            body || res.statusText,
            res.status,
            errorCode(res.status),
            true
          );
          continue;
        }

        throw new AgentError(
          body || res.statusText,
          res.status,
          errorCode(res.status),
          retryable
        );
      }

      return res.json();
    } catch (err) {
      if (err instanceof AgentError) {
        if (err.retryable && attempt < opts.maxRetries) {
          lastError = err;
          continue;
        }
        throw err;
      }
      throw err;
    }
  }

  throw lastError || new AgentError("Max retries exceeded", 502, "MAX_RETRIES", false);
}
