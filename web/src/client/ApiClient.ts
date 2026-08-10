/**
 * Core API client: typed fetch wrapper with interceptors, error handling,
 * idempotency keys, ETag/If-Match, cursor pagination, and 401 refresh
 * singleflight (docs/06 §4, docs/16 §4).
 *
 * Components never call $fetch directly — they go through this client.
 */

import { ApiClientError, type ApiError, type ApiErrorCode } from '../types/error.js';
import type { ApiEnvelope, ApiMeta } from '../types/envelope.js';

/** Client configuration. */
export interface ApiClientConfig {
  /** Base URL, e.g. `http://localhost:8000/api/v1`. */
  baseUrl: string;
  /** Request timeout in ms (default 30000). */
  timeoutMs?: number;
  /** Locale for X-Locale header. */
  locale?: string;
  /** Timezone for X-Timezone header. */
  timezone?: string;
  /** CSRF token for cookie-changing requests. */
  csrfToken?: string;
  /** Custom fetch implementation (for testing). */
  fetchImpl?: typeof fetch;
  /** Called on 401 to attempt token refresh (singleflight). */
  onRefresh?: () => Promise<void>;
}

/** Request options. */
export interface RequestOptions {
  /** HTTP method (default GET). */
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  /** Request body (will be JSON-serialized). */
  body?: unknown;
  /** Query parameters. */
  query?: Record<string, string | number | boolean | undefined>;
  /** ETag for If-Match header (optimistic concurrency, docs/06 §13). */
  ifMatch?: string;
  /** Idempotency key for safe retry of mutations (docs/06 §4). */
  idempotencyKey?: string;
  /** Abort signal for cancellation. */
  signal?: AbortSignal;
  /** Custom headers. */
  headers?: Record<string, string>;
}

/** Parsed response: data + meta + links. */
export interface ParsedResponse<T> {
  data: T;
  meta: ApiMeta;
  links?: { next?: string; prev?: string };
  etag?: string;
  statusCode: number;
}

/**
 * The typed API client. All methods return a `ParsedResponse<T>` or throw
 * `ApiClientError`. The client handles:
 *
 * - JSON serialization / parsing;
 * - X-Request-Id, X-Locale, X-Timezone headers;
 * - CSRF header for mutations;
 * - If-Match for optimistic concurrency;
 * - Idempotency-Key for safe retry;
 * - Timeout via AbortController;
 * - 401 refresh singleflight;
 * - Typed error parsing from the error envelope.
 */
export class ApiClient {
  private readonly config: Required<Omit<ApiClientConfig, 'onRefresh' | 'csrfToken' | 'locale' | 'timezone'>> &
    Pick<ApiClientConfig, 'onRefresh' | 'csrfToken' | 'locale' | 'timezone'>;
  private refreshPromise: Promise<void> | null = null;

  constructor(config: ApiClientConfig) {
    this.config = {
      timeoutMs: 30000,
      fetchImpl: globalThis.fetch,
      ...config,
    };
  }

  /** Execute a request and return the parsed envelope. */
  async request<T>(path: string, options: RequestOptions = {}): Promise<ParsedResponse<T>> {
    const url = this.buildUrl(path, options.query);
    const headers = this.buildHeaders(options);
    const signal = this.mergeSignals(options.signal, this.config.timeoutMs);

    let response = await this.doFetch(url, headers, options, signal);

    // 401: attempt refresh singleflight, then retry once.
    if (response.status === 401 && this.config.onRefresh) {
      await this.refreshSingleflight();
      const retryHeaders = this.buildHeaders(options);
      response = await this.doFetch(url, retryHeaders, options, signal);
    }

    return this.parseResponse<T>(response);
  }

  /** GET request. */
  async get<T>(path: string, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ParsedResponse<T>> {
    return this.request<T>(path, { ...options, method: 'GET' });
  }

  /** POST request. */
  async post<T>(path: string, body?: unknown, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ParsedResponse<T>> {
    return this.request<T>(path, { ...options, method: 'POST', body });
  }

  /** POST request with a FormData body (multipart upload). */
  async postMultipart<T>(path: string, body: FormData, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ParsedResponse<T>> {
    return this.request<T>(path, { ...options, method: 'POST', body });
  }

  /** PUT request. */
  async put<T>(path: string, body?: unknown, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ParsedResponse<T>> {
    return this.request<T>(path, { ...options, method: 'PUT', body });
  }

  /** PATCH request. */
  async patch<T>(path: string, body?: unknown, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ParsedResponse<T>> {
    return this.request<T>(path, { ...options, method: 'PATCH', body });
  }

  /** DELETE request. */
  async delete<T>(path: string, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ParsedResponse<T>> {
    return this.request<T>(path, { ...options, method: 'DELETE' });
  }

  /** GET a binary blob (e.g. file download). Returns the raw Blob. */
  async downloadBlob(path: string, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<Blob> {
    const url = this.buildUrl(path, options?.query);
    const headers = this.buildHeaders({ ...options, method: 'GET' });
    headers['Accept'] = options?.headers?.Accept ?? 'application/octet-stream';
    const signal = this.mergeSignals(options?.signal, this.config.timeoutMs);

    let response = await this.doFetch(url, headers, { method: 'GET' }, signal);

    if (response.status === 401 && this.config.onRefresh) {
      await this.refreshSingleflight();
      const retryHeaders = this.buildHeaders({ ...options, method: 'GET' });
      retryHeaders['Accept'] = options?.headers?.Accept ?? 'application/octet-stream';
      response = await this.doFetch(url, retryHeaders, { method: 'GET' }, signal);
    }

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      throw new ApiClientError(
        {
          code: 'download_failed',
          message_key: 'errors.download_failed',
          request_id: response.headers.get('X-Request-Id') ?? '',
          message: text || `HTTP ${response.status}`,
        },
        response.status,
      );
    }
    return response.blob();
  }

  /** PUT request with raw body (ArrayBuffer/Blob, for file uploads). */
  async putRaw<T>(path: string, body: ArrayBuffer | Blob, options?: Omit<RequestOptions, 'method' | 'body'>): Promise<ParsedResponse<T>> {
    return this.request<T>(path, { ...options, method: 'PUT', body });
  }

  // --- internals ---

  private buildUrl(path: string, query?: Record<string, string | number | boolean | undefined>): string {
    // Join baseUrl + path via string concatenation (new URL drops the path
    // prefix when given a relative path against an absolute base).
    const base = this.config.baseUrl.replace(/\/$/, '');
    const fullUrl = base + (path.startsWith('/') ? path : '/' + path);
    // If baseUrl is absolute (http://...), parse with URL for query handling.
    // If relative, use window.location.origin as the base.
    const urlBase = /^https?:\/\//.test(fullUrl)
      ? undefined
      : (typeof window !== 'undefined' ? window.location.origin : 'http://localhost');
    const url = new URL(fullUrl, urlBase);
    if (query) {
      for (const [key, value] of Object.entries(query)) {
        if (value !== undefined) {
          url.searchParams.set(key, String(value));
        }
      }
    }
    // Return pathname + search (relative) if baseUrl was relative,
    // otherwise the full absolute URL.
    return /^https?:\/\//.test(this.config.baseUrl)
      ? url.toString()
      : url.pathname + url.search;
  }

  private buildHeaders(options: RequestOptions): Record<string, string> {
    const headers: Record<string, string> = {
      'Accept': 'application/json',
      'X-Request-Id': crypto.randomUUID(),
      ...options.headers,
    };
    // Only set JSON Content-Type when body is not FormData/Blob/ArrayBuffer.
    const body = options.body;
    const isBinaryBody = body instanceof FormData || body instanceof Blob || body instanceof ArrayBuffer;
    if (!isBinaryBody) {
      headers['Content-Type'] = 'application/json';
    }
    if (this.config.locale) {
      headers['X-Locale'] = this.config.locale;
    }
    if (this.config.timezone) {
      headers['X-Timezone'] = this.config.timezone;
    }
    if (options.ifMatch) {
      headers['If-Match'] = options.ifMatch;
    }
    if (options.idempotencyKey) {
      headers['Idempotency-Key'] = options.idempotencyKey;
    }
    if (this.config.csrfToken && options.method && options.method !== 'GET') {
      headers['X-CSRF-Token'] = this.config.csrfToken;
    }
    return headers;
  }

  private mergeSignals(external?: AbortSignal, timeoutMs?: number): AbortSignal {
    const controller = new AbortController();
    if (external) {
      external.addEventListener('abort', () => controller.abort());
    }
    if (timeoutMs) {
      setTimeout(() => controller.abort(), timeoutMs);
    }
    return controller.signal;
  }

  private async doFetch(
    url: string,
    headers: Record<string, string>,
    options: RequestOptions,
    signal: AbortSignal,
  ): Promise<Response> {
    const fetchImpl = this.config.fetchImpl;
    const body = options.body;
    const isBinaryBody = body instanceof FormData || body instanceof Blob || body instanceof ArrayBuffer;
    const init: RequestInit = {
      method: options.method ?? 'GET',
      headers,
      signal,
      credentials: 'include', // cookies for session
    };
    if (body !== undefined && options.method !== 'GET') {
      init.body = isBinaryBody ? body : JSON.stringify(body);
    }
    try {
      return await fetchImpl(url, init);
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') {
        throw new ApiClientError(
          {
            code: 'query.timeout' as ApiErrorCode,
            message_key: 'errors.query.timeout',
            request_id: headers['X-Request-Id'] ?? '',
          },
          504,
        );
      }
      throw err;
    }
  }

  private async parseResponse<T>(response: Response): Promise<ParsedResponse<T>> {
    const text = await response.text();
    let body: unknown;
    try {
      body = text ? JSON.parse(text) : null;
    } catch (e) {
      console.error('[ApiClient] JSON parse error:', e, 'text:', text?.substring(0, 200), 'url:', response.url, 'status:', response.status);
      throw e;
    }

    if (!response.ok) {
      // Error body may be wrapped (`{ error: { code, ... } }` per docs/06)
      // or unwrapped (`{ code, ... }`).
      const errorBody = body?.error ?? body;
      const error = errorBody as ApiError | undefined;
      if (error && error.code && error.request_id) {
        throw new ApiClientError(error, response.status);
      }
      // Unknown error shape — wrap generically.
      throw new ApiClientError(
        {
          code: 'internal_error',
          message_key: 'errors.internal_error',
          request_id: response.headers.get('X-Request-Id') ?? '',
          message: text || `HTTP ${response.status}`,
        },
        response.status,
      );
    }

    const envelope = body as ApiEnvelope<T>;
    if (envelope && envelope.data !== undefined) {
      return {
        data: envelope.data,
        meta: envelope.meta,
        links: envelope.links,
        etag: response.headers.get('ETag') ?? undefined,
        statusCode: response.status,
      };
    }

    // Non-envelope response (e.g. health check).
    return {
      data: body as T,
      meta: { request_id: response.headers.get('X-Request-Id') ?? '' },
      etag: response.headers.get('ETag') ?? undefined,
      statusCode: response.status,
    };
  }

  private async refreshSingleflight(): Promise<void> {
    if (!this.config.onRefresh) return;
    if (!this.refreshPromise) {
      this.refreshPromise = this.config.onRefresh().finally(() => {
        this.refreshPromise = null;
      });
    }
    await this.refreshPromise;
  }
}
