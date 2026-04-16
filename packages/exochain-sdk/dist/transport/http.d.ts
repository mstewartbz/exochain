/**
 * HTTP transport for the `exo-gateway` REST API.
 *
 * Uses the global `fetch` which is available in Node 18+ and all modern
 * browsers. No third-party HTTP library is used to keep the SDK dependency-
 * free.
 */
import type { HealthResponse } from '../types.js';
/** Options for {@link HttpTransport}. */
export interface HttpTransportOptions {
    /** Optional API key sent as `Authorization: Bearer <apiKey>`. */
    readonly apiKey?: string;
    /** Request timeout in milliseconds. Defaults to 30_000. */
    readonly timeout?: number;
    /** Override the `fetch` implementation (useful for tests). */
    readonly fetch?: typeof fetch;
}
/** Small fetch wrapper that serializes and deserializes JSON bodies. */
export declare class HttpTransport {
    #private;
    constructor(baseUrl: string, opts?: HttpTransportOptions);
    /** Gateway `/health` probe. */
    health(): Promise<HealthResponse>;
    /** Issue a GET and parse the JSON body as `T`. */
    get<T>(path: string): Promise<T>;
    /** Issue a POST with a JSON body and parse the JSON response as `T`. */
    post<T>(path: string, body: unknown): Promise<T>;
    request<T>(method: string, path: string, body?: unknown): Promise<T>;
}
//# sourceMappingURL=http.d.ts.map