import { ApiError } from "./client";

function fallbackMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export function formatApiError(error: unknown): string {
  if (!(error instanceof ApiError)) {
    return fallbackMessage(error);
  }

  const detail = error.message?.trim() || "请求失败";

  switch (error.status) {
    case 400:
      return `请求参数无效：${detail}`;
    case 401:
      return `认证失败：请检查当前 JWT 是否有效，或在 disabled 模式下清空 JWT。${detail}`;
    case 403:
      return `无权访问当前租户或执行此操作：请检查活动租户、JWT 中的 tenant_id / allowed_tenant_ids / tenant_roles。${detail}`;
    case 404:
      return `资源不存在：${detail}`;
    case 409:
      return `资源冲突：${detail}`;
    case 429:
      return `请求过于频繁：${detail}`;
    default:
      return detail;
  }
}
