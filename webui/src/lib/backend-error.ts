type ErrorPayload = {
  code?: string;
  message?: string;
  params?: Record<string, unknown>;
};

function extractRawErrorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error && typeof error.message === "string") return error.message;
  if (error && typeof error === "object") {
    const obj = error as Record<string, unknown>;
    if (typeof obj.message === "string") return obj.message;
    if (typeof obj.error === "string") return obj.error;
  }
  return String(error);
}

function parseErrorPayload(raw: string): ErrorPayload | null {
  const text = raw.trim();
  if (!text.startsWith("{") || !text.endsWith("}")) return null;
  try {
    const parsed = JSON.parse(text) as ErrorPayload;
    if (!parsed || typeof parsed !== "object") return null;
    return parsed;
  } catch {
    return null;
  }
}

function extractName(params?: Record<string, unknown>) {
  if (!params || typeof params !== "object") return "";
  const value = params.name;
  return typeof value === "string" ? value : "";
}

export function localizeBackendErrorMessage(error: unknown, isZh: boolean): string {
  const raw = extractRawErrorMessage(error);
  const payload = parseErrorPayload(raw);
  if (!payload?.code) return raw;

  const name = extractName(payload.params);
  switch (payload.code) {
    case "PROVIDER_NAME_CONFLICT":
      return isZh
        ? `提供商名称已存在：${name || "（未提供）"}`
        : `Provider name already exists: ${name || "(unknown)"}`;
    case "ROUTE_NAME_CONFLICT":
      return isZh
        ? `路由名称已存在：${name || "（未提供）"}`
        : `Route name already exists: ${name || "(unknown)"}`;
    case "API_KEY_NAME_CONFLICT":
      return isZh
        ? `API Key 名称已存在：${name || "（未提供）"}`
        : `API key name already exists: ${name || "(unknown)"}`;
    default:
      return payload.message || raw;
  }
}
