const BASE = "";

export class ApiError extends Error {
  status: number;
  title?: string;

  constructor(message: string, status: number, title?: string) {
    super(message);
    this.status = status;
    this.title = title;
  }
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const jwt = localStorage.getItem("dmsx.jwt")?.trim();
  const headers: Record<string, string> = {};
  if (body) headers["Content-Type"] = "application/json";
  if (jwt) headers["Authorization"] = jwt.startsWith("Bearer ")
    ? jwt
    : `Bearer ${jwt}`;

  const res = await fetch(`${BASE}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({} as any));
    const detail = err?.detail ?? err?.error ?? res.statusText;
    const title = err?.title;
    throw new ApiError(String(detail), res.status, title ? String(title) : undefined);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const api = {
  get: <T>(path: string) => request<T>("GET", path),
  post: <T>(path: string, body?: unknown) => request<T>("POST", path, body),
  put: <T>(path: string, body?: unknown) => request<T>("PUT", path, body),
  patch: <T>(path: string, body?: unknown) =>
    request<T>("PATCH", path, body),
  del: <T>(path: string) => request<T>("DELETE", path),
};

export const TENANT_ID = "00000000-0000-0000-0000-000000000001";
export const tenantPath = (p: string) => `/v1/tenants/${TENANT_ID}${p}`;

export function buildQuery(
  params?: Record<string, string | number | undefined>,
): string {
  if (!params) return "";
  const sp = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== "") sp.set(k, String(v));
  }
  const s = sp.toString();
  return s ? `?${s}` : "";
}
