/**
 * Nitro server route: proxy /api/v1/** to the backend API.
 *
 * This avoids CORS by making the browser send same-origin requests
 * to the Nuxt server, which forwards them to the API.
 */
import { defineEventHandler, getRequestWebStream, getQuery, getHeaders, setResponseHeaders, setResponseStatus, getRequestIP } from 'h3';
import { Readable } from 'node:stream';

export default defineEventHandler(async (event) => {
  const apiUrl = process.env.ENGINER_API_URL ?? 'http://localhost:8010';
  const path = event.context.params?._ ?? '';
  const query = getQuery(event);
  const queryString = new URLSearchParams(
    Object.entries(query)
      .filter(([, v]) => v !== undefined && v !== null)
      .map(([k, v]) => [k, String(v)]),
  ).toString();
  const target = `${apiUrl}/api/v1/${path}${queryString ? '?' + queryString : ''}`;

  const method = event.method;
  const headers = getHeaders(event);
  // Stream the request body untouched — binary uploads (dump .xz archives,
  // file chunks) must not go through JSON parse/stringify, and multi-GB
  // uploads must not be buffered in the proxy's memory.
  const bodyStream = ['GET', 'HEAD'].includes(method)
    ? undefined
    : getRequestWebStream(event);

  // Forward headers except host/content-length (fetch recomputes it).
  const forwardHeaders: Record<string, string> = {};
  for (const [key, value] of Object.entries(headers)) {
    const lower = key.toLowerCase();
    if (lower === 'host' || lower === 'content-length' || lower === 'connection') continue;
    if (typeof value === 'string') {
      forwardHeaders[key] = value;
    }
  }
  // Preserve the real client address for the API's audit log.
  const clientIp = getRequestIP(event, { xForwardedFor: true });
  if (clientIp) {
    forwardHeaders['x-forwarded-for'] = clientIp;
  }

  try {
    const response = await globalThis.fetch(target, {
      method,
      headers: forwardHeaders,
      body: bodyStream,
      // Required by undici when the body is a ReadableStream.
      duplex: 'half',
    } as RequestInit & { duplex: 'half' });

    // Forward status
    setResponseStatus(event, response.status);

    // Forward headers (skip encoding/length — h3 handles these)
    response.headers.forEach((value, key) => {
      const lower = key.toLowerCase();
      if (lower === 'transfer-encoding') return;
      if (lower === 'content-encoding') return;
      if (lower === 'content-length') return;
      if (lower === 'connection') return;
      setResponseHeaders(event, { [key]: value });
    });

    // Stream the body — large payloads (dumps, file downloads) must not
    // be buffered in the proxy's memory.
    if (response.body) {
      return Readable.fromWeb(response.body as import('node:stream/web').ReadableStream);
    }
    return null;
  } catch (err) {
    console.error('[proxy] error:', err);
    setResponseStatus(event, 502);
    return JSON.stringify({ error: 'Bad Gateway', message: String(err) });
  }
});
